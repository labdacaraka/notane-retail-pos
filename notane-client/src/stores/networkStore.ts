import { defineStore } from 'pinia'
import axios from 'axios'
import { useDialogStore } from './dialogStore'

export interface ConnectedDevice {
  device_id: string
  device_name: string
  device_type: 'CASHIER' | 'CUSTOMER_DISPLAY' | 'TABLET' | 'BACKOFFICE' | string
  client_ip: string
  user_name?: string
  connected_at: string
  last_heartbeat: string
  status: 'ONLINE' | 'DISCONNECTED' | string
}

export interface ServerStatus {
  is_running: boolean
  port: number
  control_port?: number
  started_at: string | null
  uptime_seconds: number
  host_ips: string[]
  total_connected_devices: number
  pairing_code?: string
}

export const useNetworkStore = defineStore('network', {
  state: () => ({
    // Node Role: Apakah mesin ini bertindak sebagai SERVER (Host Utama) atau CLIENT (Terminal Kasir)
    nodeRole: (localStorage.getItem('notane_node_role') || 'SERVER') as 'SERVER' | 'CLIENT',

    // Client Connection State
    targetHost: localStorage.getItem('notane_target_host') || '127.0.0.1',
    targetPort: (() => {
      const stored = localStorage.getItem('notane_target_port')
      if (stored === '' || stored === 'null' || stored === 'undefined') {
        return null
      }
      if (stored === '8080') {
        localStorage.setItem('notane_target_port', '18080')
        return 18080
      }
      if (stored) {
        const num = Number(stored)
        return isNaN(num) || num <= 0 ? null : num
      }
      return 18080
    })() as number | null,
    deviceId: localStorage.getItem('notane_device_id') || localStorage.getItem('notane_terminal') || `KASIR-01`,
    deviceName: localStorage.getItem('notane_device_name') || localStorage.getItem('notane_terminal') || `Terminal Utama`,
    deviceType: (localStorage.getItem('notane_device_type') || 'CASHIER') as ConnectedDevice['device_type'],
    pairingCode: localStorage.getItem('notane_pairing_code') || 'NOTANE-8899',

    // Connection Intent: 'CONNECTED' jika user pernah klik sambung, 'DISCONNECTED' jika sengaja diputus
    connectionIntent: (localStorage.getItem('notane_connection_intent') || 'DISCONNECTED') as 'CONNECTED' | 'DISCONNECTED',
    
    connectionStatus: 'DISCONNECTED' as 'CONNECTED' | 'CONNECTING' | 'DISCONNECTED' | 'ERROR',
    latencyMs: null as number | null,
    lastPingTime: null as string | null,
    lastHeartbeatTime: null as number | null,
    autoReconnect: localStorage.getItem('notane_auto_reconnect') !== 'false',
    disconnectReason: '' as string,

    // Host Server State (Default isolated ports 18080 and 18081)
    hostStatus: {
      is_running: false,
      port: 18080,
      control_port: 18081,
      started_at: null,
      uptime_seconds: 0,
      host_ips: ['127.0.0.1'],
      total_connected_devices: 0,
      pairing_code: 'NOTANE-8899',
    } as ServerStatus,
    connectedDevices: [] as ConnectedDevice[],
    isHostLoading: false,

    // Internal WS & Timer
    _ws: null as WebSocket | null,
    _watchdogTimer: null as any,
    _reconnectTimer: null as any,
    _statusPollTimer: null as any,
  }),

  getters: {
    serverUrl(state): string {
      const raw = (state.targetHost || '127.0.0.1').trim()
      if (raw.startsWith('http://') || raw.startsWith('https://')) {
        return raw.replace(/\/$/, '')
      }
      const port = state.targetPort ? `:${state.targetPort}` : ''
      return `http://${raw}${port}`
    },
    controlUrl(state): string {
      if (typeof window !== 'undefined' && window.location?.origin) {
        return window.location.origin
      }
      const raw = (state.targetHost || '127.0.0.1').trim()
      if (raw.startsWith('http://') || raw.startsWith('https://')) {
        return raw.replace(/\/$/, '')
      }
      const port = state.targetPort ? `:${state.targetPort}` : ''
      return `http://${raw}${port}`
    },
    wsUrl(state): string {
      const raw = (state.targetHost || '127.0.0.1').trim()
      let isHttps = false
      let hostWithoutProto = raw

      if (raw.startsWith('https://')) {
        isHttps = true
        hostWithoutProto = raw.replace(/^https:\/\//, '').replace(/\/$/, '')
      } else if (raw.startsWith('http://')) {
        isHttps = false
        hostWithoutProto = raw.replace(/^http:\/\//, '').replace(/\/$/, '')
      } else {
        isHttps = window.location.protocol === 'https:'
        const port = state.targetPort ? `:${state.targetPort}` : ''
        hostWithoutProto = `${raw}${port}`
      }

      const proto = isHttps ? 'wss:' : 'ws:'
      return `${proto}//${hostWithoutProto}/ws/v1/events`
    },
    isConnected(state): boolean {
      return state.connectionStatus === 'CONNECTED'
    },
    isServerActive(state): boolean {
      return state.hostStatus.is_running
    },
    formattedUptime(state): string {
      const sec = state.hostStatus.uptime_seconds || 0
      const hours = Math.floor(sec / 3600)
      const mins = Math.floor((sec % 3600) / 60)
      const secs = sec % 60
      return `${hours.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
    },
    isServerNode(state): boolean {
      return state.nodeRole === 'SERVER'
    },
    isClientNode(state): boolean {
      return state.nodeRole === 'CLIENT'
    },
  },

  actions: {
    setNodeRole(role: 'SERVER' | 'CLIENT') {
      this.nodeRole = role
      localStorage.setItem('notane_node_role', role)
      if (role === 'SERVER') {
        this.targetHost = '127.0.0.1'
        this.targetPort = 18080
        localStorage.setItem('notane_target_host', '127.0.0.1')
        localStorage.setItem('notane_target_port', '18080')
      }
    },
    // -------------------------------------------------------------
    // CLIENT ACTIONS: CONNECT, DISCONNECT, PING, PAIRING
    // -------------------------------------------------------------
    saveConnectionSettings(
      host: string,
      port?: number | null,
      deviceId: string = 'KASIR-01',
      deviceName: string = 'Terminal Utama',
      deviceType: string = 'CASHIER',
      pairingCode?: string
    ) {
      this.targetHost = host.trim()
      if (port !== undefined && port !== null && !isNaN(Number(port)) && Number(port) > 0) {
        this.targetPort = Number(port)
        localStorage.setItem('notane_target_port', this.targetPort.toString())
      } else {
        this.targetPort = null
        localStorage.removeItem('notane_target_port')
      }
      this.deviceId = deviceId.trim().toUpperCase()
      this.deviceName = deviceName.trim()
      this.deviceType = deviceType as ConnectedDevice['device_type']

      if (pairingCode !== undefined) {
        this.pairingCode = pairingCode.trim()
        localStorage.setItem('notane_pairing_code', this.pairingCode)
      }

      localStorage.setItem('notane_target_host', this.targetHost)
      localStorage.setItem('notane_device_id', this.deviceId)
      localStorage.setItem('notane_device_name', this.deviceName)
      localStorage.setItem('notane_device_type', this.deviceType)
    },

    async pingServer(host?: string, port?: number | null): Promise<{ success: boolean; latency: number; error?: string }> {
      const targetHost = (host ?? this.targetHost).trim()
      const targetPort = port !== undefined ? port : this.targetPort
      const startTime = performance.now()

      let url = ''
      if (targetHost.startsWith('http://') || targetHost.startsWith('https://')) {
        url = `${targetHost.replace(/\/$/, '')}/api/v1/system/health`
      } else {
        const portStr = targetPort ? `:${targetPort}` : ''
        url = `http://${targetHost}${portStr}/api/v1/system/health`
      }

      try {
        const response = await axios.get(url, { timeout: 3500 })
        const latency = Math.round(performance.now() - startTime)
        if (response.data && response.data.status === 'ok') {
          this.latencyMs = latency
          this.lastPingTime = new Date().toLocaleTimeString('id-ID')
          return { success: true, latency }
        }
        return { success: false, latency, error: 'Respons server tidak valid' }
      } catch (err: any) {
        return { success: false, latency: 0, error: err.message || 'Gagal terhubung ke host' }
      }
    },

    async verifyPairingCode(code?: string): Promise<{ valid: boolean; message: string }> {
      const targetCode = (code ?? this.pairingCode).trim()
      try {
        const res = await axios.post(`${this.serverUrl}/api/v1/system/pairing-code/verify`, {
          pairing_code: targetCode,
        }, { timeout: 4000 })
        return { valid: !!res.data?.valid, message: res.data?.message || 'Kode valid' }
      } catch (err: any) {
        const msg = err.response?.data?.message || 'Kode pencocokan salah atau server menolak koneksi'
        return { valid: false, message: msg }
      }
    },

    async connect(options?: {
      host?: string
      port?: number | null
      deviceId?: string
      deviceName?: string
      deviceType?: string
      pairingCode?: string
    }): Promise<boolean> {
      if (options) {
        this.saveConnectionSettings(
          options.host ?? this.targetHost,
          options.port !== undefined ? options.port : this.targetPort,
          options.deviceId ?? this.deviceId,
          options.deviceName ?? this.deviceName,
          options.deviceType ?? this.deviceType,
          options.pairingCode ?? this.pairingCode
        )
      }

      this.connectionStatus = 'CONNECTING'
      this.disconnectReason = ''
      this.connectionIntent = 'CONNECTED'
      localStorage.setItem('notane_connection_intent', 'CONNECTED')

      // Koneksi langsung berbasis WebSocket subscription (Zero HTTP Polling)
      return new Promise<boolean>((resolve) => {
        let isResolved = false
        const timer = setTimeout(() => {
          if (!isResolved) {
            isResolved = true
            if (this.connectionStatus !== 'CONNECTED') {
              this.connectionStatus = 'ERROR'
              this.disconnectReason = 'Batas waktu koneksi ke Host Server habis (Timeout)'
              resolve(false)
            } else {
              resolve(true)
            }
          }
        }, 4000)

        this.initWebSocket((success, errorMsg) => {
          if (!isResolved) {
            isResolved = true
            clearTimeout(timer)
            if (success) {
              this.connectionStatus = 'CONNECTED'
              this.disconnectReason = ''
              resolve(true)
            } else {
              this.connectionStatus = 'ERROR'
              this.disconnectReason = errorMsg || 'Gagal terhubung ke Host Server'
              resolve(false)
            }
          }
        })
      })
    },

    disconnect(reason = 'Koneksi diputuskan oleh pengguna') {
      if (this._watchdogTimer) {
        clearTimeout(this._watchdogTimer)
        this._watchdogTimer = null
      }
      if (this._reconnectTimer) {
        clearTimeout(this._reconnectTimer)
        this._reconnectTimer = null
      }
      if (this._ws) {
        try {
          this._ws.close()
        } catch (_) {}
        this._ws = null
      }

      this.connectionStatus = 'DISCONNECTED'
      this.connectionIntent = 'DISCONNECTED'
      localStorage.setItem('notane_connection_intent', 'DISCONNECTED')
      this.disconnectReason = reason
      this.latencyMs = null
    },

    initWebSocket(onResult?: (success: boolean, error?: string) => void) {
      if (this._ws) {
        try {
          this._ws.close()
        } catch (_) {}
        this._ws = null
      }

      const params = new URLSearchParams({
        device_id: this.deviceId,
        device_name: this.deviceName,
        device_type: this.deviceType,
        pairing_code: this.pairingCode,
      })

      const currentUser = localStorage.getItem('notane_user')
      if (currentUser) {
        try {
          const userObj = JSON.parse(currentUser)
          if (userObj.name) {
            params.append('user_name', userObj.name)
          }
        } catch (_) {}
      }

      const wsTarget = `${this.wsUrl}?${params.toString()}`

      try {
        const ws = new WebSocket(wsTarget)
        this._ws = ws
        let resultReported = false

        ws.onopen = () => {
          this.connectionStatus = 'CONNECTED'
          this.connectionIntent = 'CONNECTED'
          localStorage.setItem('notane_connection_intent', 'CONNECTED')
          this.disconnectReason = ''
          this.resetWatchdog()
          if (onResult && !resultReported) {
            resultReported = true
            onResult(true)
          }
        }

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data)
            this.handleWsMessage(data)
            if (onResult && !resultReported && this.connectionStatus === 'CONNECTED') {
              resultReported = true
              onResult(true)
            }
          } catch (_) {}
        }

        ws.onclose = () => {
          if (this._watchdogTimer) {
            clearTimeout(this._watchdogTimer)
            this._watchdogTimer = null
          }
          if (this.connectionStatus === 'CONNECTED' || this.connectionStatus === 'CONNECTING') {
            this.connectionStatus = 'DISCONNECTED'
            if (!this.disconnectReason) {
              this.disconnectReason = 'Koneksi ke host terputus'
            }
            if (this.connectionIntent === 'CONNECTED' && this.autoReconnect) {
              this.scheduleReconnect()
            }
          }
          if (onResult && !resultReported) {
            resultReported = true
            onResult(false, this.disconnectReason || 'Koneksi ditolak atau ditutup oleh host')
          }
        }

        ws.onerror = () => {
          this.connectionStatus = 'ERROR'
          if (!this.disconnectReason) {
            this.disconnectReason = 'Kesalahan socket jaringan'
          }
          if (onResult && !resultReported) {
            resultReported = true
            onResult(false, this.disconnectReason)
          }
        }
      } catch (err: any) {
        this.connectionStatus = 'ERROR'
        this.disconnectReason = err.message || 'Gagal membuka WebSocket'
        if (onResult) onResult(false, this.disconnectReason)
      }
    },

    // Pengawas Pasif: Klien HANYA mendengarkan sinyal detak broadcast dari Host Server (Zero Polling Overhead)
    resetWatchdog() {
      if (this._watchdogTimer) clearTimeout(this._watchdogTimer)
      // Host Server memancarkan sinyal detak setiap 2 detik.
      // Jika dalam 7 detik tidak ada sinyal dari Host Server, tandai koneksi terputus.
      this._watchdogTimer = setTimeout(() => {
        if (this.connectionStatus === 'CONNECTED') {
          console.warn('[Network Watchdog] Detak sinyal dari Host Server terhenti (> 7 detik).')
          this.connectionStatus = 'DISCONNECTED'
          this.disconnectReason = 'Sinyal Host Server terhenti (Server tidak merespons)'
          this.latencyMs = null
          if (this.connectionIntent === 'CONNECTED' && this.autoReconnect) {
            this.scheduleReconnect()
          }
        }
      }, 7000)
    },

    scheduleReconnect() {
      if (this._reconnectTimer) clearTimeout(this._reconnectTimer)
      this._reconnectTimer = setTimeout(async () => {
        if (this.connectionIntent === 'CONNECTED' && this.autoReconnect && this.connectionStatus !== 'CONNECTED') {
          await this.connect()
        }
      }, 4000)
    },

    handleWsMessage(data: any) {
      if (!data || !data.type) return
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('notane-ws-message', { detail: data }))
      }
      const dialog = useDialogStore()

      // Detak Jantung yang dipancarkan oleh Host Server (Server-Driven Heartbeat)
      if (data.type === 'HOST_HEARTBEAT') {
        if (this.connectionStatus !== 'CONNECTED') {
          this.connectionStatus = 'CONNECTED'
          this.disconnectReason = ''
        }
        this.lastHeartbeatTime = Date.now()
        this.lastPingTime = new Date().toLocaleTimeString('id-ID')
        if (typeof data.connected_devices === 'number') {
          this.hostStatus.total_connected_devices = data.connected_devices
        }
        if (data.timestamp) {
          this.latencyMs = Math.max(0, Math.round(Date.now() - Number(data.timestamp)))
        }
        this.resetWatchdog()
        return
      }

      if (data.type === 'CONNECTION_REJECTED') {
        this.disconnect(data.message || 'Koneksi ditolak oleh server')
        this.connectionStatus = 'ERROR'
        this.connectionIntent = 'DISCONNECTED'
        localStorage.setItem('notane_connection_intent', 'DISCONNECTED')
        dialog.alert(`Penolakan Koneksi Host: ${data.message}`, 'danger')
        return
      }

      if (data.type === 'FORCE_DISCONNECT') {
        this.disconnect(data.message || 'Koneksi Anda diputuskan oleh Host Server')
        this.connectionStatus = 'ERROR'
        this.connectionIntent = 'DISCONNECTED'
        localStorage.setItem('notane_connection_intent', 'DISCONNECTED')
        dialog.alert(`Pemberitahuan Host: ${data.message || 'Terminal kasir telah diputuskan dari server karena kode pairing diubah.'}`, 'warning')
        return
      }

      if (data.type === 'SERVER_STOPPED') {
        this.hostStatus.is_running = false
        this.connectionStatus = 'DISCONNECTED'
        this.disconnectReason = data.message || 'Host Server sedang dihentikan oleh Administrator'
        if (this._ws) {
          try {
            this._ws.close()
          } catch (_) {}
          this._ws = null
        }
        if (this._watchdogTimer) {
          clearTimeout(this._watchdogTimer)
          this._watchdogTimer = null
        }
        this.latencyMs = null
        return
      }

      if (data.type === 'SERVER_STARTED') {
        this.hostStatus.is_running = true
        this.disconnectReason = ''
        if (this.connectionIntent === 'CONNECTED' && this.autoReconnect && this.connectionStatus !== 'CONNECTED') {
          this.connect()
        }
        return
      }

      if (data.type === 'PAIRING_CODE_UPDATED') {
        if (data.pairing_code) {
          this.hostStatus.pairing_code = data.pairing_code
          if (this.nodeRole !== 'SERVER' && this.pairingCode !== data.pairing_code) {
            this.disconnect('Kode pairing telah diubah oleh Host Server. Masukkan kode pairing baru untuk menghubungkan kembali.')
            this.connectionStatus = 'ERROR'
            this.connectionIntent = 'DISCONNECTED'
            localStorage.setItem('notane_connection_intent', 'DISCONNECTED')
          }
        }
        return
      }

      if (data.type === 'DEVICE_CONNECTED' || data.type === 'DEVICE_DISCONNECTED') {
        // Refresh tabel device host
        this.fetchConnectedDevices()
        this.fetchHostStatus()
      }
    },

    // -------------------------------------------------------------
    // HOST SERVER ACTIONS: START, STOP, MONITOR, KICK, PAIRING CODE
    // -------------------------------------------------------------
    getControlUrls(): string[] {
      const urls: string[] = []
      if (typeof window !== 'undefined' && window.location?.origin) {
        urls.push(window.location.origin)
      }
      if (this.serverUrl && !urls.includes(this.serverUrl)) {
        urls.push(this.serverUrl)
      }
      if (this.controlUrl && !urls.includes(this.controlUrl)) {
        urls.push(this.controlUrl)
      }
      return urls
    },

    async fetchHostStatus() {
      for (const base of this.getControlUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/status`, { timeout: 2000 })
          if (res.data) {
            this.hostStatus = res.data
            if (this.nodeRole === 'SERVER' && res.data.port) {
              this.targetPort = res.data.port
              localStorage.setItem('notane_target_port', String(res.data.port))
            }
            if (this.nodeRole === 'SERVER') {
              if (this.hostStatus.is_running) {
                this.connectionStatus = 'CONNECTED'
                this.disconnectReason = ''
              } else {
                this.connectionStatus = 'DISCONNECTED'
                this.disconnectReason = 'Host Server dimatikan'
              }
            }
            return
          }
        } catch (_) {}
      }

      // Jika seluruh endpoint tidak menjawab
      this.hostStatus.is_running = false
      if (this.nodeRole === 'SERVER') {
        this.connectionStatus = 'DISCONNECTED'
        this.disconnectReason = 'Daemon server tidak merespons'
      }
    },

    async fetchPairingCode() {
      for (const base of this.getControlUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/pairing-code`, { timeout: 2000 })
          if (res.data?.pairing_code) {
            this.hostStatus.pairing_code = res.data.pairing_code
            return
          }
        } catch (_) {}
      }
    },

    async regeneratePairingCode(): Promise<string | null> {
      this.isHostLoading = true
      try {
        let res = null
        for (const base of this.getControlUrls()) {
          try {
            res = await axios.post(`${base}/api/v1/system/pairing-code/regenerate`, {}, { timeout: 4000 })
            if (res?.data?.pairing_code) break
          } catch (_) {}
        }
        if (res?.data?.pairing_code) {
          this.hostStatus.pairing_code = res.data.pairing_code
          return res.data.pairing_code
        }
        return null
      } catch (err: any) {
        useDialogStore().alert(`Gagal mengacak kode pencocokan: ${err.message}`, 'danger')
        return null
      } finally {
        this.isHostLoading = false
      }
    },

    async updatePairingCode(newCode: string): Promise<{ success: boolean; message: string; pairing_code?: string }> {
      this.isHostLoading = true
      try {
        const target = newCode.trim().toUpperCase()
        if (target.length < 4 || target.length > 32) {
          return { success: false, message: 'Kode pairing minimal 4 karakter dan maksimal 32 karakter.' }
        }
        let res = null
        for (const base of this.getControlUrls()) {
          try {
            res = await axios.post(`${base}/api/v1/system/pairing-code/update`, { pairing_code: target }, { timeout: 4000 })
            if (res?.data?.pairing_code) break
          } catch (_) {}
        }
        if (res?.data?.pairing_code) {
          this.hostStatus.pairing_code = res.data.pairing_code
          return { success: true, message: res.data.message || 'Kode pairing toko berhasil diubah.', pairing_code: res.data.pairing_code }
        }
        return { success: false, message: 'Gagal mengubah kode pairing di host server. Pastikan server host aktif.' }
      } catch (err: any) {
        return { success: false, message: err?.response?.data?.message || err.message || 'Gagal mengubah kode pairing.' }
      } finally {
        this.isHostLoading = false
      }
    },

    async fetchConnectedDevices() {
      for (const base of this.getControlUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/devices`, { timeout: 2000 })
          if (Array.isArray(res.data)) {
            this.connectedDevices = res.data
            return
          }
        } catch (_) {}
      }
    },

    async startHostServer(): Promise<boolean> {
      this.isHostLoading = true
      try {
        let res = null
        for (const base of this.getControlUrls()) {
          try {
            res = await axios.post(`${base}/api/v1/system/server/start`, {}, { timeout: 25000 })
            if (res?.data?.status === 'success') {
              break
            }
          } catch (_) {}
        }

        if (res?.data?.status === 'success') {
          this.hostStatus.is_running = true
          await this.fetchHostStatus()
          return true
        }
        throw new Error(res?.data?.message || 'Server host tidak dapat dihubungi di port 18081')
      } catch (err: any) {
        useDialogStore().alert(`Gagal menyalakan server: ${err.message || 'Koneksi gagal'}`, 'danger')
        return false
      } finally {
        this.isHostLoading = false
      }
    },

    async stopHostServer(): Promise<boolean> {
      this.isHostLoading = true
      try {
        let res = null
        for (const base of this.getControlUrls()) {
          try {
            res = await axios.post(`${base}/api/v1/system/server/stop`, {}, { timeout: 4000 })
            if (res?.data?.status === 'success') {
              break
            }
          } catch (_) {}
        }

        if (res?.data?.status === 'success') {
          this.hostStatus.is_running = false
          this.connectionStatus = 'DISCONNECTED'
          this.disconnectReason = 'Host Server dimatikan oleh Administrator (Port ditutup)'
          if (this._ws) {
            try { this._ws.close() } catch (_) {}
            this._ws = null
          }
          if (this._watchdogTimer) {
            clearTimeout(this._watchdogTimer)
            this._watchdogTimer = null
          }
          this.latencyMs = null
          await this.fetchHostStatus()
          return true
        }
        return false
      } catch (err: any) {
        useDialogStore().alert(`Gagal menghentikan server: ${err.message || 'Koneksi gagal'}`, 'danger')
        return false
      } finally {
        this.isHostLoading = false
      }
    },

    async restartHostServer(): Promise<boolean> {
      this.isHostLoading = true
      try {
        let res = null
        for (const base of this.getControlUrls()) {
          try {
            res = await axios.post(`${base}/api/v1/system/server/restart`, {}, { timeout: 8000 })
            if (res?.data?.status === 'success') {
              break
            }
          } catch (_) {}
        }

        if (res?.data?.status === 'success') {
          this.hostStatus.is_running = true
          await this.fetchHostStatus()
          await this.fetchConnectedDevices()
          return true
        }
        return false
      } catch (err: any) {
        useDialogStore().alert(`Gagal memuat ulang server: ${err.message || 'Koneksi gagal'}`, 'danger')
        return false
      } finally {
        this.isHostLoading = false
      }
    },

    async kickDevice(deviceId: string): Promise<boolean> {
      try {
        let res = null
        try {
          res = await axios.post(`${this.controlUrl}/api/v1/system/devices/${encodeURIComponent(deviceId)}/disconnect`, {}, { timeout: 3000 })
        } catch (_) {
          res = await axios.post(`${this.serverUrl}/api/v1/system/devices/${encodeURIComponent(deviceId)}/disconnect`, {}, { timeout: 3000 })
        }
        if (res?.data?.status === 'success') {
          await this.fetchConnectedDevices()
          return true
        }
        return false
      } catch (err: any) {
        useDialogStore().alert(`Gagal memutuskan device ${deviceId}: ${err.message}`, 'danger')
        return false
      }
    },

    async checkConnection(): Promise<boolean> {
      if (this.nodeRole === 'SERVER') {
        if (this.connectionStatus === 'CONNECTED') {
          return true
        }
        await this.fetchHostStatus()
        return this.hostStatus.is_running
      } else {
        // CLIENT mode
        // Jika sudah aktif terhubung dan mensubscribe sinyal detak host, langsung true (Zero Polling!)
        if (this.connectionStatus === 'CONNECTED') {
          return true
        }
        // Jika belum terhubung dan intent pengguna ingin terhubung, sambungkan via WebSocket
        if (this.connectionIntent === 'CONNECTED' || this.autoReconnect) {
          return await this.connect()
        }
        return false
      }
    },

    startHostPolling() {
      // Inisialisasi data sekali saat antarmuka dibuka
      this.fetchHostStatus()
      this.fetchConnectedDevices()
      this.fetchPairingCode()
      // Pastikan subscriber event real-time aktif (Zero Interval Polling)
      if (!this._ws || this._ws.readyState !== WebSocket.OPEN) {
        this.initWebSocket()
      }
    },

    stopHostPolling() {
      if (this._statusPollTimer) {
        clearInterval(this._statusPollTimer)
        this._statusPollTimer = null
      }
    },
  },
})
