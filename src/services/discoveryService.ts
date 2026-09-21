import axios from 'axios'

export interface DiscoveredServer {
  id: string
  name: string
  host: string
  port: number
  latency: number
  isRunning: boolean
  isLocal: boolean
  pairingCodeHint?: string
  lastSeen: string
}

export const discoveryService = {
  /**
   * Pindai server Notane POS di jaringan lokal (LAN & mDNS candidates)
   */
  async scanLanServers(onProgress?: (scanned: number, total: number) => void): Promise<DiscoveredServer[]> {
    const candidates: { host: string; port: number; isLocal: boolean; name?: string }[] = []

    // 1. Localhost candidate
    candidates.push({ host: '127.0.0.1', port: 8080, isLocal: true, name: 'Local Host Server' })

    // 2. Previously saved host
    const savedHost = localStorage.getItem('notane_target_host')
    const savedPort = Number(localStorage.getItem('notane_target_port')) || 8080
    if (savedHost && savedHost !== '127.0.0.1') {
      candidates.push({ host: savedHost, port: savedPort, isLocal: false, name: 'Server Terakhir Digunakan' })
    }

    // 3. Current window hostname if running in browser
    if (typeof window !== 'undefined' && window.location.hostname && window.location.hostname !== '127.0.0.1' && window.location.hostname !== 'localhost') {
      candidates.push({ host: window.location.hostname, port: 8080, isLocal: false, name: `Gateway (${window.location.hostname})` })
    }

    // 4. Subnet heuristic: Jika savedHost berupa IPv4 (misal 192.168.1.x), buat sampel IP di subnet tersebut
    const baseIpMatch = (savedHost || window.location.hostname || '192.168.1.1').match(/^(\d{1,3}\.\d{1,3}\.\d{1,3})\.\d{1,3}$/)
    if (baseIpMatch) {
      const subnet = baseIpMatch[1]
      const probeIps = [1, 2, 3, 5, 10, 15, 20, 25, 50, 100, 101, 150, 200]
      for (const lastOctet of probeIps) {
        const ip = `${subnet}.${lastOctet}`
        if (!candidates.some(c => c.host === ip)) {
          candidates.push({ host: ip, port: 8080, isLocal: false })
        }
      }
    }

    // Dedup candidates
    const uniqueCandidates = candidates.filter(
      (c, index, self) => index === self.findIndex(t => t.host === c.host && t.port === c.port)
    )

    const discovered: DiscoveredServer[] = []
    let completed = 0

    const probePromises = uniqueCandidates.map(async (cand) => {
      try {
        const startTime = performance.now()
        const url = `http://${cand.host}:${cand.port}/api/v1/system/health`
        const res = await axios.get(url, { timeout: 2000 })
        const latency = Math.round(performance.now() - startTime)

        if (res.data && (res.data.status === 'ok' || res.data.app?.includes('Notane'))) {
          // Ambil detail status
          let serverName = cand.name || `Notane POS Server (${cand.host})`
          try {
            const statusRes = await axios.get(`http://${cand.host}:${cand.port}/api/v1/system/status`, { timeout: 1500 })
            if (statusRes.data?.host_ips?.length) {
              serverName = `PC Server Utama [${cand.host}]`
            }
          } catch (_) {}

          discovered.push({
            id: `${cand.host}:${cand.port}`,
            name: serverName,
            host: cand.host,
            port: cand.port,
            latency,
            isRunning: true,
            isLocal: cand.isLocal,
            lastSeen: new Date().toLocaleTimeString('id-ID'),
          })
        }
      } catch (_) {
        // Unreachable candidate, ignore
      } finally {
        completed++
        if (onProgress) {
          onProgress(completed, uniqueCandidates.length)
        }
      }
    })

    await Promise.all(probePromises)

    // Simpan hasil discovery di localStorage
    try {
      localStorage.setItem('notane_discovered_servers', JSON.stringify(discovered))
    } catch (_) {}

    return discovered
  },

  getCachedDiscoveredServers(): DiscoveredServer[] {
    try {
      const data = localStorage.getItem('notane_discovered_servers')
      return data ? JSON.parse(data) : []
    } catch {
      return []
    }
  },
}
