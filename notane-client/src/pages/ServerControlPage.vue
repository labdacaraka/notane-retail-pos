<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { useNetworkStore } from '@/stores/networkStore'
import { useLicenseStore } from '@/stores/licenseStore'
import ConfirmModal from '@/components/common/ConfirmModal.vue'
import LicenseUpdateModal from '@/components/common/LicenseUpdateModal.vue'

const networkStore = useNetworkStore()
const licenseStore = useLicenseStore()
const copiedNotice = ref(false)
const copiedPairingNotice = ref(false)

// State Modal Konfirmasi In-App
const showStopConfirm = ref(false)
const showRestartConfirm = ref(false)
const showKickConfirm = ref(false)
const deviceToKick = ref('')
const showRegenConfirm = ref(false)
const showLicenseModal = ref(false)

onMounted(() => {
  networkStore.startHostPolling()
})

onUnmounted(() => {
  networkStore.stopHostPolling()
})

const isServerRunning = computed(() => networkStore.hostStatus.is_running)

const copyHostUrl = (ip: string) => {
  const url = `http://${ip}:${networkStore.hostStatus.port}`
  navigator.clipboard.writeText(url)
  copiedNotice.value = true
  setTimeout(() => {
    copiedNotice.value = false
  }, 2500)
}

const copyPairingCode = () => {
  const code = networkStore.hostStatus.pairing_code || 'NOTANE-8899'
  navigator.clipboard.writeText(code)
  copiedPairingNotice.value = true
  setTimeout(() => {
    copiedPairingNotice.value = false
  }, 2000)
}

const handleRegenerateCode = () => {
  showRegenConfirm.value = true
}

const executeRegenerateCode = async () => {
  showRegenConfirm.value = false
  await networkStore.regeneratePairingCode()
}

const handleStartServer = async () => {
  await networkStore.startHostServer()
}

const handleStopServer = () => {
  showStopConfirm.value = true
}

const executeStopServer = async () => {
  showStopConfirm.value = false
  await networkStore.stopHostServer()
}

const handleRestartServer = () => {
  showRestartConfirm.value = true
}

const executeRestartServer = async () => {
  showRestartConfirm.value = false
  await networkStore.restartHostServer()
}

const handleKick = (deviceId: string) => {
  deviceToKick.value = deviceId
  showKickConfirm.value = true
}

const executeKick = async () => {
  if (deviceToKick.value) {
    const id = deviceToKick.value
    showKickConfirm.value = false
    deviceToKick.value = ''
    await networkStore.kickDevice(id)
  }
}

const formatDateTime = (isoStr: string) => {
  try {
    const d = new Date(isoStr)
    return d.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch (_) {
    return isoStr
  }
}

const getRoleBadgeClass = (type: string) => {
  switch (type) {
    case 'CASHIER':
      return 'bg-blue-100 text-blue-800 border-blue-300'
    case 'CUSTOMER_DISPLAY':
      return 'bg-purple-100 text-purple-800 border-purple-300'
    case 'TABLET':
      return 'bg-amber-100 text-amber-800 border-amber-300'
    default:
      return 'bg-slate-100 text-slate-800 border-slate-300'
  }
}

const getRoleLabel = (type: string) => {
  switch (type) {
    case 'CASHIER':
      return 'Kasir POS'
    case 'CUSTOMER_DISPLAY':
      return 'Layar Pelanggan'
    case 'TABLET':
      return 'Tablet Pelayan'
    default:
      return 'Back-Office'
  }
}
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 text-slate-800 text-xs overflow-y-auto p-4 space-y-4 font-sans">
      <!-- Top Bar Title -->
      <div class="bg-white p-3.5 border border-slate-300 rounded-sm shadow-xs flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:server-server-exchange" class="w-6 h-6 drop-shadow-xs" />
            <h1 class="text-sm font-bold uppercase tracking-wider text-slate-800">
              Pengendali Server Host & Pemantau Jaringan LAN
            </h1>
            <span class="text-[10px] bg-purple-50 text-purple-700 border border-purple-200 font-bold px-2 py-0.5 rounded-full flex items-center gap-1 font-mono">
              Mode Server Host (Master)
            </span>
          </div>
          <p class="text-[11px] text-slate-500 mt-0.5">
            Kelola background daemon server, pantau koneksi kasir, dan distribusikan alamat server ke terminal kasir.
          </p>
        </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          @click="showLicenseModal = true"
          class="px-3 py-1.5 bg-purple-50 hover:bg-purple-100 text-purple-900 border border-purple-300 rounded-sm font-semibold transition-colors flex items-center gap-1.5 cursor-pointer text-xs"
          title="Buka status lisensi dan pembaruan sistem"
        >
          <Icon icon="fluent-color:lock-shield-20" class="w-4 h-4" />
          <span>Lisensi & Pembaruan</span>
          <span
            v-if="licenseStore.updateInfo?.has_update"
            class="px-1.5 py-0.2 bg-emerald-500 text-white text-[9px] rounded-full font-black animate-pulse"
          >
            Ada Pembaruan
          </span>
        </button>

        <button
          type="button"
          @click="networkStore.fetchConnectedDevices(); networkStore.fetchHostStatus()"
          class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 border border-slate-300 rounded-sm font-semibold transition-colors flex items-center gap-1.5 cursor-pointer text-xs"
        >
          <Icon icon="streamline-ultimate-color:server-refresh-1" class="w-4 h-4" />
          <span>Segarkan Status</span>
        </button>
      </div>
    </div>

    <!-- Banner Kontrol Server -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- Panel Sakelar Utama Server -->
      <div class="lg:col-span-2 bg-white p-4 border border-slate-300 rounded-sm shadow-xs flex flex-col justify-between space-y-4">
        <div class="flex items-start justify-between">
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider mb-1">
              Status Operasional Daemon Server
            </div>
            <div class="flex items-center gap-3">
              <span
                class="px-3 py-1 rounded-sm text-xs font-bold uppercase tracking-wide border shadow-xs flex items-center gap-2"
                :class="isServerRunning ? 'bg-emerald-100 text-emerald-800 border-emerald-300' : 'bg-rose-100 text-rose-800 border-rose-300'"
              >
                <span class="w-2.5 h-2.5 rounded-full" :class="isServerRunning ? 'bg-emerald-500 animate-ping' : 'bg-rose-500'"></span>
                {{ isServerRunning ? 'Server Berjalan (Active)' : 'Server Dihentikan (Stopped)' }}
              </span>
              <span class="text-slate-500 text-[11px]">
                Port: <strong class="text-slate-800 font-mono">:{{ networkStore.hostStatus.port }}</strong>
              </span>
            </div>
          </div>

          <!-- Tombol Start / Stop / Restart -->
          <div class="flex items-center gap-2">
            <button
              type="button"
              @click="handleStartServer"
              :disabled="isServerRunning || networkStore.isHostLoading"
              class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white font-bold rounded-sm border border-emerald-700 shadow-xs transition-colors flex items-center gap-1.5 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed text-xs"
            >
              <Icon icon="streamline-ultimate-color:button-play-1" class="w-4 h-4" />
              <span>Mulai (Start)</span>
            </button>

            <button
              type="button"
              @click="handleRestartServer"
              :disabled="networkStore.isHostLoading"
              class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white font-bold rounded-sm border border-amber-600 shadow-xs transition-colors flex items-center gap-1.5 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed text-xs"
              title="Muat ulang layanan POS dan sinkronisasi skema database"
            >
              <Icon icon="streamline-ultimate-color:server-refresh-1" class="w-4 h-4" />
              <span>Muat Ulang (Restart)</span>
            </button>

            <button
              type="button"
              @click="handleStopServer"
              :disabled="!isServerRunning || networkStore.isHostLoading"
              class="px-4 py-2 bg-rose-600 hover:bg-rose-700 text-white font-bold rounded-sm border border-rose-700 shadow-xs transition-colors flex items-center gap-1.5 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed text-xs"
            >
              <Icon icon="streamline-ultimate-color:controls-pause" class="w-4 h-4" />
              <span>Hentikan (Stop)</span>
            </button>
          </div>
        </div>

        <!-- Alamat IP Server untuk Kasir Lain -->
        <div class="bg-slate-50 p-3 border border-slate-200 rounded-sm">
          <div class="font-bold text-slate-700 uppercase text-[11px] mb-1.5 flex items-center justify-between">
            <span>Alamat URL Host untuk Terminal Kasir (LAN):</span>
            <span v-if="copiedNotice" class="text-emerald-700 font-semibold text-[10px]">
              ✓ Alamat URL berhasil disalin!
            </span>
          </div>

          <div class="flex flex-wrap gap-2">
            <div
              v-for="ip in networkStore.hostStatus.host_ips"
              :key="ip"
              class="flex items-center bg-white border border-slate-300 rounded-sm px-2.5 py-1 gap-2 shadow-2xs font-mono text-xs"
            >
              <span class="text-slate-500">http://</span>
              <strong class="text-slate-800">{{ ip }}:{{ networkStore.hostStatus.port }}</strong>
              <button
                type="button"
                @click="copyHostUrl(ip)"
                class="px-1.5 py-0.5 bg-slate-100 hover:bg-emerald-100 text-slate-700 hover:text-emerald-800 rounded-xs border border-slate-300 text-[10px] font-sans font-semibold cursor-pointer"
                title="Salin untuk diinput pada terminal kasir"
              >
                Salin
              </button>
            </div>
          </div>
          <p class="text-[10px] text-slate-500 mt-2">
            * Masukkan IP di atas pada menu pengaturan jaringan di komputer/tablet kasir (KASIR-02, Display, dll).
          </p>
        </div>
      </div>

      <!-- Metrik Kartu Stat Ringkas -->
      <div class="bg-white p-4 border border-slate-300 rounded-sm shadow-xs flex flex-col justify-between space-y-3">
        <div class="font-bold text-slate-700 uppercase tracking-wider text-[11px] border-b border-slate-200 pb-1.5">
          Statistik Server Host
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="bg-slate-50 p-3 border border-slate-200 rounded-sm">
            <div class="text-[10px] text-slate-500 font-bold uppercase">Terminal Terhubung</div>
            <div class="text-2xl font-black text-emerald-700 mt-0.5">
              {{ networkStore.connectedDevices.length }}
              <span class="text-xs font-normal text-slate-500">Unit</span>
            </div>
          </div>

          <div class="bg-slate-50 p-3 border border-slate-200 rounded-sm">
            <div class="text-[10px] text-slate-500 font-bold uppercase">Waktu Aktif (Uptime)</div>
            <div class="text-xl font-black font-mono text-slate-800 mt-1">
              {{ networkStore.formattedUptime }}
            </div>
          </div>
        </div>

        <div class="bg-slate-50 p-2.5 border border-slate-200 rounded-sm text-[11px] text-slate-600 space-y-1">
          <div class="flex justify-between">
            <span>Protokol:</span>
            <span class="font-semibold text-slate-800">REST API + WebSocket Axum</span>
          </div>
          <div class="flex justify-between">
            <span>Sinkronisasi Data:</span>
            <span class="font-semibold text-slate-800">Real-time LAN Broadcast</span>
          </div>
          <div class="flex justify-between">
            <span>Deteksi Timeout:</span>
            <span class="font-semibold text-slate-800">5 detik (Heartbeat Ping)</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Banner Kode Pencocokan (Pairing Code) Keamanan Host -->
    <div class="bg-amber-50/80 p-3.5 border border-amber-300 rounded-sm shadow-xs flex flex-wrap items-center justify-between gap-3">
      <div>
        <div class="font-bold text-amber-950 uppercase text-[11px] flex items-center gap-1.5">
          <span>🔐</span>
          <span>Kode Pencocokan Perangkat (Pairing Code Keamanan Host)</span>
        </div>
        <p class="text-[10px] text-amber-800 mt-0.5 max-w-xl">
          Hanya terminal kasir yang memasukkan kode pencocokan ini yang diizinkan menyambung ke server. Mencegah perangkat asing yang tidak dikenal mengakses database jika server ditaruh di VPS atau jaringan terbuka.
        </p>
      </div>

      <div class="flex items-center gap-2">
        <div class="bg-white border-2 border-amber-400 px-3 py-1.5 rounded-sm font-mono font-black text-sm tracking-widest text-slate-900 shadow-2xs">
          {{ networkStore.hostStatus.pairing_code || 'NOTANE-8899' }}
        </div>
        <button
          type="button"
          @click="copyPairingCode"
          class="px-3 py-1.5 bg-white hover:bg-amber-100 text-amber-900 font-bold border border-amber-400 rounded-sm text-xs cursor-pointer shadow-2xs transition-colors"
        >
          {{ copiedPairingNotice ? '✓ Tersalin!' : 'Salin Kode' }}
        </button>
        <button
          type="button"
          @click="handleRegenerateCode"
          :disabled="networkStore.isHostLoading"
          class="px-3 py-1.5 bg-amber-600 hover:bg-amber-700 text-white font-bold border border-amber-700 rounded-sm text-xs cursor-pointer shadow-2xs flex items-center gap-1 transition-colors disabled:opacity-50"
        >
          <span>🎲</span>
          <span>Acak Kode Baru</span>
        </button>
      </div>
    </div>

    <!-- Tabel Monitor Perangkat Terhubung -->
    <div class="bg-white border border-slate-300 rounded-sm shadow-xs flex flex-col flex-1 overflow-hidden">
      <div class="bg-slate-50 px-4 py-2.5 border-b border-slate-300 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="font-bold text-xs uppercase tracking-wider text-slate-800">
            Monitor Perangkat & Terminal Kasir Aktif
          </span>
          <span class="bg-emerald-600 text-white font-bold px-2 py-0.5 rounded-full text-[10px]">
            {{ networkStore.connectedDevices.length }} Online
          </span>
        </div>
        <div class="text-[11px] text-slate-500">
          Setiap kasir yang terkoneksi tercatat otomatis di bawah ini.
        </div>
      </div>

      <div class="overflow-x-auto flex-1">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="bg-slate-100 text-slate-600 uppercase text-[10px] tracking-wider border-b border-slate-300 font-bold">
              <th class="py-2 px-3 w-10 text-center">No</th>
              <th class="py-2 px-3">Terminal ID</th>
              <th class="py-2 px-3">Nama Perangkat</th>
              <th class="py-2 px-3">Kategori</th>
              <th class="py-2 px-3">IP Address</th>
              <th class="py-2 px-3">Kasir / User</th>
              <th class="py-2 px-3">Waktu Konek</th>
              <th class="py-2 px-3">Heartbeat Terakhir</th>
              <th class="py-2 px-3 text-center">Status</th>
              <th class="py-2 px-3 text-center w-28">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200">
            <tr
              v-for="(dev, idx) in networkStore.connectedDevices"
              :key="dev.device_id"
              class="hover:bg-slate-50 transition-colors"
            >
              <td class="py-2.5 px-3 text-center text-slate-400 font-mono">{{ idx + 1 }}</td>
              <td class="py-2.5 px-3 font-mono font-bold text-slate-900">
                <span class="px-2 py-0.5 bg-slate-100 border border-slate-300 rounded-xs">
                  {{ dev.device_id }}
                </span>
              </td>
              <td class="py-2.5 px-3 font-semibold text-slate-800">{{ dev.device_name }}</td>
              <td class="py-2.5 px-3">
                <span class="px-2 py-0.5 rounded-xs border text-[10px] font-semibold" :class="getRoleBadgeClass(dev.device_type)">
                  {{ getRoleLabel(dev.device_type) }}
                </span>
              </td>
              <td class="py-2.5 px-3 font-mono text-slate-600">{{ dev.client_ip }}</td>
              <td class="py-2.5 px-3 font-medium text-slate-700">{{ dev.user_name || '-' }}</td>
              <td class="py-2.5 px-3 text-slate-500 font-mono">{{ formatDateTime(dev.connected_at) }}</td>
              <td class="py-2.5 px-3 text-slate-500 font-mono">{{ formatDateTime(dev.last_heartbeat) }}</td>
              <td class="py-2.5 px-3 text-center">
                <div class="flex flex-col items-center gap-0.5">
                  <span class="px-2 py-0.5 rounded-xs bg-emerald-100 text-emerald-800 border border-emerald-300 font-bold text-[10px]">
                    ● ONLINE
                  </span>
                  <span class="text-[9px] text-emerald-700 font-semibold flex items-center gap-0.5">
                    ✓ Terverifikasi
                  </span>
                </div>
              </td>
              <td class="py-2.5 px-3 text-center">
                <button
                  type="button"
                  @click="handleKick(dev.device_id)"
                  class="px-2 py-1 bg-rose-50 hover:bg-rose-100 text-rose-700 hover:text-rose-900 font-semibold border border-rose-300 rounded-sm transition-colors text-[10px] cursor-pointer"
                  title="Putuskan perangkat ini secara paksa"
                >
                  ✕ Putuskan
                </button>
              </td>
            </tr>

            <!-- Baris Kosong Jika Belum Ada Device -->
            <tr v-if="networkStore.connectedDevices.length === 0">
              <td colspan="10" class="py-10 text-center text-slate-400 bg-slate-50/50">
                <div class="text-2xl mb-1">📡</div>
                <div class="font-bold text-slate-600 text-xs">Belum Ada Perangkat Terminal Kasir yang Terhubung</div>
                <div class="text-[11px] text-slate-400 max-w-md mx-auto mt-1">
                  Buka layar kasir di perangkat lain atau tab browser baru, lalu hubungkan menggunakan IP host di atas.
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Modal Konfirmasi Hentikan Server -->
    <ConfirmModal
      :show="showStopConfirm"
      title="Konfirmasi Penghentian Server Host"
      message="Apakah Anda yakin ingin menghentikan Host Server daemon?"
      sub-message="Menghentikan server akan menolak permintaan transaksi baru dan memutuskan seluruh terminal kasir yang sedang aktif terhubung."
      confirm-text="Ya, Hentikan Server"
      cancel-text="Batal / Tetap Aktif"
      type="danger"
      @confirm="executeStopServer"
      @close="showStopConfirm = false"
    />

    <!-- Modal Konfirmasi Muat Ulang (Restart) Server -->
    <ConfirmModal
      :show="showRestartConfirm"
      title="Konfirmasi Muat Ulang (Restart) Server"
      message="Apakah Anda yakin ingin memuat ulang (Restart) Host Server?"
      sub-message="Server akan me-reload layanan POS, menjalankan migrasi database otomatis jika ada yang baru, dan menyegarkan koneksi kasir."
      confirm-text="Ya, Muat Ulang Sekarang"
      cancel-text="Batal"
      type="warning"
      @confirm="executeRestartServer"
      @close="showRestartConfirm = false"
    />

    <!-- Modal Konfirmasi Putuskan (Kick) Perangkat -->
    <ConfirmModal
      :show="showKickConfirm"
      title="Konfirmasi Putuskan Terminal"
      :message="`Yakin ingin memutuskan koneksi terminal [${deviceToKick}] secara paksa?`"
      sub-message="Terminal kasir tersebut akan langsung terputus dari jaringan dan harus menyambung kembali."
      confirm-text="Ya, Putuskan Sekarang"
      cancel-text="Batal"
      type="danger"
      @confirm="executeKick"
      @close="showKickConfirm = false"
    />

    <!-- Modal Konfirmasi Acak Kode Baru -->
    <ConfirmModal
      :show="showRegenConfirm"
      title="Konfirmasi Acak Kode Pencocokan Baru"
      message="Apakah Anda yakin ingin membuat Kode Pencocokan baru?"
      sub-message="Terminal kasir yang baru akan wajib memasukkan kode baru ini untuk dapat tersambung. Terminal yang sudah terhubung tidak akan terputus."
      confirm-text="Ya, Acak Kode Baru"
      cancel-text="Batal"
      type="warning"
      @confirm="executeRegenerateCode"
      @close="showRegenConfirm = false"
    />
    <!-- Modal Lisensi & Update -->
    <LicenseUpdateModal :is-open="showLicenseModal" @close="showLicenseModal = false" />
    </div>
  </AdminLayout>
</template>
