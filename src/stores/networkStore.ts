import { defineStore } from 'pinia'
import axios from 'axios'

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
    targetPort: Number(localStorage.getItem('notane_target_port')) || 8080,
    deviceId: localStorage.getItem('notane_device_id') || `KASIR-01`,
    deviceName: localStorage.getItem('notane_device_name') || `Terminal Utama`,
    deviceType: (localStorage.getItem('notane_device_type') || 'CASHIER') as ConnectedDevice['device_type'],
    pairingCode: localStorage.getItem('notane_pairing_code') || 'NOTANE-8899',

    // Connection Intent: 'CONNECTED' jika user pernah klik sambung, 'DISCONNECTED' jika sengaja diputus
    connectionIntent: (localStorage.getItem('notane_connection_intent') || 'DISCONNECTED') as 'CONNECTED' | 'DISCONNECTED',
    
    connectionStatus: 'DISCONNECTED' as 'CONNECTED' | 'CONNECTING' | 'DISCONNECTED' | 'ERROR',
    latencyMs: null as number | null,
    lastPingTime: null as string | null,
    autoReconnect: localStorage.getItem('notane_auto_reconnect') !== 'false',
    disconnectReason: '' as string,

    // Host Server State
    hostStatus: {
      is_running: true,
      port: 8080,
      control_port: 8081,
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
    _heartbeatTimer: null as any,
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
      const raw = (state.targetHost || '127.0.0.1').trim()
      if (raw.startsWith('http://') || raw.startsWith('https://')) {
        return raw.replace(/\/$/, '')
      }
      const port = Number(state.targetPort) || 8080
      const ctrlPort = state.hostStatus.control_port || (port + 1)
      return `http://${raw}:${ctrlPort}`
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
        this.targetPort = 8080
        localStorage.setItem('notane_target_host', '127.0.0.1')
        localStorage.setItem('notane_target_port', '8080')
      }
    },
    // -------------------------------------------------------------
    // CLIENT ACTIONS: CONNECT, DISCONNECT, PING, PAIRING
    // -------------------------------------------------------------
    saveConnectionSettings(
      host: string,
      port: number,
      deviceId: string,
      deviceName: string,
      deviceType: string,
      pairingCode?: string
    ) {
      this.targetHost = host.trim()
      this.targetPort = port
      this.deviceId = deviceId.trim().toUpperCase()
      this.deviceName = deviceName.trim()
      this.deviceType = deviceType

      if (pairingCode !== undefined) {
        this.pairingCode = pairingCode.trim()
        localStorage.setItem('notane_pairing_code', this.pairingCode)
      }

      localStorage.setItem('notane_target_host', this.targetHost)
      localStorage.setItem('notane_target_port', this.targetPort.toString())
      localStorage.setItem('notane_device_id', this.deviceId)
      localStorage.setItem('notane_device_name', this.deviceName)
      localStorage.setItem('notane_device_type', this.deviceType)
    },

    async pingServer(host?: string, port?: number): Promise<{ success: boolean; latency: number; error?: string }> {
      const targetHost = host ?? this.targetHost
      const targetPort = port ?? this.targetPort
      const startTime = performance.now()

      let url = `http://${targetHost}:${targetPort}/api/v1/system/health`
      if (targetHost.startsWith('http://') || targetHost.startsWith('https://')) {
        url = `${targetHost.replace(/\/$/, '')}/api/v1/system/health`
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
      port?: number
      deviceId?: string
      deviceName?: string
      deviceType?: string
      pairingCode?: string
    }): Promise<boolean> {
      if (options) {
        this.saveConnectionSettings(
          options.host ?? this.targetHost,
          options.port ?? this.targetPort,
          options.deviceId ?? this.deviceId,
          options.deviceName ?? this.deviceName,
          options.deviceType ?? this.deviceType,
          options.pairingCode ?? this.pairingCode
        )
      }

      this.connectionStatus = 'CONNECTING'
      this.disconnectReason = ''

      // 1. Lakukan tes HTTP Ping
      const pingRes = await this.pingServer()
      if (!pingRes.success) {
        this.connectionStatus = 'ERROR'
        this.disconnectReason = pingRes.error || 'Server tidak merespons'
        return false
      }

      // 2. Lakukan Verifikasi Kode Pencocokan (Pairing Code)
      const pairRes = await this.verifyPairingCode()
      if (!pairRes.valid) {
        this.connectionStatus = 'ERROR'
        this.disconnectReason = pairRes.message
        return false
      }

      // 3. Hubungkan WebSocket Event & Heartbeat
      this.initWebSocket()
      this.connectionStatus = 'CONNECTED'
      this.connectionIntent = 'CONNECTED'
      localStorage.setItem('notane_connection_intent', 'CONNECTED')
      return true
    },

    disconnect(reason = 'Koneksi diputuskan oleh pengguna') {
      if (this._heartbeatTimer) {
        clearInterval(this._heartbeatTimer)
        this._heartbeatTimer = null
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

    initWebSocket() {
      if (this._ws) {
        try {
          this._ws.close()
        } catch (_) {}
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

        ws.onopen = () => {
          this.connectionStatus = 'CONNECTED'
          this.connectionIntent = 'CONNECTED'
          localStorage.setItem('notane_connection_intent', 'CONNECTED')
          this.disconnectReason = ''
          this.startHeartbeat()
        }

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data)
            this.handleWsMessage(data)
          } catch (_) {}
        }

        ws.onclose = () => {
          if (this.connectionStatus === 'CONNECTED') {
            this.connectionStatus = 'DISCONNECTED'
            if (!this.disconnectReason) {
              this.disconnectReason = 'Koneksi ke host terputus'
            }
            if (this.connectionIntent === 'CONNECTED' && this.autoReconnect) {
              this.scheduleReconnect()
            }
          }
        }

        ws.onerror = () => {
          this.connectionStatus = 'ERROR'
          this.disconnectReason = 'Kesalahan socket jaringan'
        }
      } catch (err: any) {
        this.connectionStatus = 'ERROR'
        this.disconnectReason = err.message || 'Gagal membuka WebSocket'
      }
    },

    startHeartbeat() {
      if (this._heartbeatTimer) clearInterval(this._heartbeatTimer)
      this._heartbeatTimer = setInterval(async () => {
        if (this._ws && this._ws.readyState === WebSocket.OPEN) {
          const start = performance.now()
          this._ws.send(JSON.stringify({ type: 'HEARTBEAT', timestamp: Date.now() }))
          this.latencyMs = Math.round(performance.now() - start)
          this.lastPingTime = new Date().toLocaleTimeString('id-ID')
        }
      }, 5000)
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

      if (data.type === 'CONNECTION_REJECTED') {
        this.disconnect(data.message || 'Koneksi ditolak oleh server')
        alert(`Penolakan Koneksi Host: ${data.message}`)
        return
      }

      if (data.type === 'FORCE_DISCONNECT') {
        this.disconnect(data.message || 'Koneksi Anda diputuskan oleh Host Server')
        alert(`Pemberitahuan Host: ${data.message || 'Terminal kasir telah diputuskan dari server.'}`)
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
        if (this._heartbeatTimer) {
          clearInterval(this._heartbeatTimer)
          this._heartbeatTimer = null
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
    async fetchHostStatus() {
      try {
        const res = await axios.get(`${this.controlUrl}/api/v1/system/status`, { timeout: 2500 })
        if (res.data) {
          this.hostStatus = res.data
          return
        }
      } catch (_) {}

      try {
        const res = await axios.get(`${this.serverUrl}/api/v1/system/status`, { timeout: 2500 })
        if (res.data) {
          this.hostStatus = res.data
        }
      } catch (_) {
        // Jika kedua port tidak menjawab, tandai is_running = false
        this.hostStatus.is_running = false
      }
    },

    async fetchPairingCode() {
      try {
        const res = await axios.get(`${this.controlUrl}/api/v1/system/pairing-code`, { timeout: 2500 })
        if (res.data?.pairing_code) {
          this.hostStatus.pairing_code = res.data.pairing_code
          return
        }
      } catch (_) {}

      try {
        const res = await axios.get(`${this.serverUrl}/api/v1/system/pairing-code`, { timeout: 2500 })
        if (res.data?.pairing_code) {
          this.hostStatus.pairing_code = res.data.pairing_code
        }
      } catch (_) {}
    },

    async regeneratePairingCode(): Promise<string | null> {
      this.isHostLoading = true
      try {
        let res = null
        try {
          res = await axios.post(`${this.controlUrl}/api/v1/system/pairing-code/regenerate`, {}, { timeout: 4000 })
        } catch (_) {
          res = await axios.post(`${this.serverUrl}/api/v1/system/pairing-code/regenerate`, {}, { timeout: 4000 })
        }
        if (res?.data?.pairing_code) {
          this.hostStatus.pairing_code = res.data.pairing_code
          return res.data.pairing_code
        }
        return null
      } catch (err: any) {
        alert(`Gagal mengacak kode pencocokan: ${err.message}`)
        return null
      } finally {
        this.isHostLoading = false
      }
    },

    async fetchConnectedDevices() {
      try {
        const res = await axios.get(`${this.controlUrl}/api/v1/system/devices`, { timeout: 2500 })
        if (Array.isArray(res.data)) {
          this.connectedDevices = res.data
          return
        }
      } catch (_) {}

      try {
        const res = await axios.get(`${this.serverUrl}/api/v1/system/devices`, { timeout: 2500 })
        if (Array.isArray(res.data)) {
          this.connectedDevices = res.data
        }
      } catch (_) {}
    },

    async startHostServer(): Promise<boolean> {
      this.isHostLoading = true
      try {
        // Nyalakan kembali melalui Host Control Channel (port 8081)
        let res = null
        try {
          res = await axios.post(`${this.controlUrl}/api/v1/system/server/start`, {}, { timeout: 5000 })
        } catch (_) {
          res = await axios.post(`${this.serverUrl}/api/v1/system/server/start`, {}, { timeout: 5000 })
        }

        if (res?.data?.status === 'success') {
          this.hostStatus.is_running = true
          await this.fetchHostStatus()
          return true
        }
        return false
      } catch (err: any) {
        alert(`Gagal menyalakan server: ${err.message || 'Koneksi gagal'}`)
        return false
      } finally {
        this.isHostLoading = false
      }
    },

    async stopHostServer(): Promise<boolean> {
      this.isHostLoading = true
      try {
        let res = null
        try {
          res = await axios.post(`${this.controlUrl}/api/v1/system/server/stop`, {}, { timeout: 4000 })
        } catch (_) {
          res = await axios.post(`${this.serverUrl}/api/v1/system/server/stop`, {}, { timeout: 4000 })
        }

        if (res?.data?.status === 'success') {
          this.hostStatus.is_running = false
          this.connectionStatus = 'DISCONNECTED'
          this.disconnectReason = 'Host Server dimatikan oleh Administrator (Port ditutup)'
          if (this._ws) {
            try { this._ws.close() } catch (_) {}
            this._ws = null
          }
          if (this._heartbeatTimer) {
            clearInterval(this._heartbeatTimer)
            this._heartbeatTimer = null
          }
          this.latencyMs = null
          await this.fetchHostStatus()
          return true
        }
        return false
      } catch (err: any) {
        alert(`Gagal menghentikan server: ${err.message || 'Koneksi gagal'}`)
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
        alert(`Gagal memutuskan device ${deviceId}: ${err.message}`)
        return false
      }
    },

    startHostPolling() {
      this.fetchHostStatus()
      this.fetchConnectedDevices()
      this.fetchPairingCode()
      if (this._statusPollTimer) clearInterval(this._statusPollTimer)
      this._statusPollTimer = setInterval(() => {
        this.fetchHostStatus()
        this.fetchConnectedDevices()
      }, 3000)
    },

    stopHostPolling() {
      if (this._statusPollTimer) {
        clearInterval(this._statusPollTimer)
        this._statusPollTimer = null
      }
    },
  },
})
