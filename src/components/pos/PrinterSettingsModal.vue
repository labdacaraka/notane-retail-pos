<script setup lang="ts">
import { ref } from 'vue'
import { usePrinterStore, type PaperWidth } from '@/stores/printerStore'

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const printerStore = usePrinterStore()
const activeTab = ref<'SERIAL' | 'BLUETOOTH' | 'VIRTUAL'>(printerStore.connectionType || 'SERIAL')
const localBaudRate = ref<number>(printerStore.baudRate || 9600)
const isTesting = ref<boolean>(false)
const testMessage = ref<string>('')
const testError = ref<string>('')

const baudRateOptions = [
  { label: '9600 bps (Standar Epson/Xprinter/Panda)', value: 9600 },
  { label: '19200 bps', value: 19200 },
  { label: '38400 bps', value: 38400 },
  { label: '115200 bps (Kecepatan Tinggi)', value: 115200 },
]

async function handleConnectSerial() {
  testMessage.value = ''
  testError.value = ''
  try {
    const res = await printerStore.connectSerial(localBaudRate.value)
    testMessage.value = res.message
    setTimeout(() => {
      testMessage.value = ''
    }, 4000)
  } catch (err: any) {
    testError.value = err.message || 'Gagal menyambungkan port serial'
  }
}

async function handleDisconnectSerial() {
  await printerStore.disconnectSerial()
  testMessage.value = 'Port serial berhasil diputuskan'
}

async function handleConnectBluetooth() {
  testMessage.value = ''
  testError.value = ''
  try {
    const res = await printerStore.connectBluetooth()
    testMessage.value = res.message
    setTimeout(() => {
      testMessage.value = ''
    }, 4000)
  } catch (err: any) {
    testError.value = err.message || 'Gagal menyambungkan bluetooth'
  }
}

function handleDisconnectBluetooth() {
  printerStore.disconnectBluetooth()
  testMessage.value = 'Koneksi bluetooth berhasil diputuskan'
}

function handleSetVirtual(width: PaperWidth = '58mm') {
  printerStore.setVirtualPrinter(`Simulator Thermal ${width}`, width)
  testMessage.value = `Simulator thermal ${width} diaktifkan untuk pengujian`
  testError.value = ''
}

async function handleTestPrint() {
  isTesting.value = true
  testMessage.value = ''
  testError.value = ''

  try {
    const res = await printerStore.testPrint()
    testMessage.value = res.message
    setTimeout(() => {
      testMessage.value = ''
    }, 4000)
  } catch (err: any) {
    testError.value = err.message || 'Gagal melakukan uji cetak'
  } finally {
    isTesting.value = false
  }
}

function selectPaperWidth(width: PaperWidth) {
  printerStore.paperWidth = width
  printerStore.saveConfig()
}

function toggleAutoPrint() {
  printerStore.autoPrint = !printerStore.autoPrint
  printerStore.saveConfig()
}

function toggleAutoCut() {
  printerStore.autoCut = !printerStore.autoCut
  printerStore.saveConfig()
}

function toggleOpenDrawer() {
  printerStore.openDrawer = !printerStore.openDrawer
  printerStore.saveConfig()
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
    @click.self="emit('close')"
  >
    <div
      class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-2xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-emerald-100 border border-emerald-300 flex items-center justify-center shadow-xs">
            <Icon icon="streamline-ultimate-color:print-text" class="w-6 h-6" />
          </div>
          <div>
            <h2 class="text-base font-bold text-slate-900 leading-tight">Pengaturan Printer Thermal Kasir</h2>
            <p class="text-xs text-slate-500">Koneksi Bluetooth, Serial Port (USB), dan cetak struk tanpa dialog</p>
          </div>
        </div>

        <button
          @click="emit('close')"
          class="w-8 h-8 rounded-lg hover:bg-slate-200 text-slate-500 hover:text-slate-800 flex items-center justify-center transition cursor-pointer text-lg font-bold"
        >
          ✕
        </button>
      </div>

      <!-- Modal Body (Scrollable) -->
      <div class="p-6 overflow-y-auto space-y-6 flex-1 text-slate-800">
        <!-- Status Banner -->
        <div
          class="p-3.5 rounded-xl border flex items-center justify-between"
          :class="{
            'bg-emerald-50 border-emerald-300 text-emerald-900': printerStore.isConnected,
            'bg-slate-50 border-slate-300 text-slate-700': !printerStore.isConnected
          }"
        >
          <div class="flex items-center gap-3">
            <span
              class="w-3.5 h-3.5 rounded-full flex items-center justify-center"
              :class="printerStore.isConnected ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'"
            >
              <span v-if="printerStore.isConnected" class="w-2 h-2 rounded-full bg-white"></span>
            </span>
            <div>
              <div class="text-xs font-bold font-mono">
                {{ printerStore.isConnected ? 'TERHUBUNG & SIAP MENCETAK' : 'BELUM TERHUBUNG' }}
              </div>
              <div class="text-xs text-slate-600 font-sans">
                {{ printerStore.printerName }} • {{ printerStore.paperWidth }} • Mode {{ printerStore.connectionType }}
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <button
              @click="handleTestPrint"
              :disabled="isTesting"
              class="bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold px-3 py-1.5 rounded-lg border border-emerald-700 shadow-xs transition active:scale-95 flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <Icon icon="streamline-ultimate-color:print-text" class="w-4 h-4" />
              <span>{{ isTesting ? 'Mencetak...' : 'Uji Cetak' }}</span>
            </button>
          </div>
        </div>

        <!-- Alert Notifikasi -->
        <div v-if="testMessage" class="bg-emerald-100 border border-emerald-300 text-emerald-800 text-xs px-3.5 py-2.5 rounded-lg font-medium flex items-center gap-2">
          <span>✓</span>
          <span>{{ testMessage }}</span>
        </div>
        <div v-if="testError" class="bg-rose-100 border border-rose-300 text-rose-800 text-xs px-3.5 py-2.5 rounded-lg font-medium flex items-center gap-2">
          <span>⚠️</span>
          <span>{{ testError }}</span>
        </div>

        <!-- Pemilihan Tipe Koneksi -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-2">Metode Koneksi Printer Kasir</label>
          
          <!-- Panduan Cepat Pemilihan Koneksi -->
          <div class="mb-3 bg-blue-50 border border-blue-200 rounded-xl p-3 text-xs text-blue-900 flex items-start gap-2.5">
            <Icon icon="streamline-ultimate-color:help-chat-bubble" class="w-5 h-5 shrink-0 mt-0.5" />
            <div class="space-y-1 text-[11.5px]">
              <div class="font-bold text-blue-950">Panduan Memilih Koneksi Printer Thermal:</div>
              <div class="text-slate-700">
                • <strong>Tab Bluetooth (BLE):</strong> Khusus printer portable nirkabel berlabel <em>Bluetooth Low Energy (BLE)</em>.<br />
                • <strong>Tab Serial Port / USB:</strong> Untuk printer kabel USB dan juga <strong>Printer Bluetooth Classic SPP (non-BLE)</strong> setelah di-pair di Windows/Mac.
              </div>
            </div>
          </div>

          <div class="grid grid-cols-3 gap-3">
            <!-- Tab Serial Port -->
            <button
              type="button"
              @click="activeTab = 'SERIAL'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'SERIAL' ? 'bg-emerald-50 border-emerald-600 ring-2 ring-emerald-500/20 text-emerald-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="streamline-ultimate-color:usb-port-1" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Serial Port / USB</span>
              <span class="text-[10px] text-slate-500 font-mono font-normal">Kabel USB & BT Classic</span>
            </button>

            <!-- Tab Bluetooth -->
            <button
              type="button"
              @click="activeTab = 'BLUETOOTH'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'BLUETOOTH' ? 'bg-emerald-50 border-emerald-600 ring-2 ring-emerald-500/20 text-emerald-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="streamline-ultimate-color:bluetooth-transfer" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Bluetooth (BLE)</span>
              <span class="text-[10px] text-slate-500 font-mono font-normal">Portable BLE Modern</span>
            </button>

            <!-- Tab Simulator -->
            <button
              type="button"
              @click="activeTab = 'VIRTUAL'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'VIRTUAL' ? 'bg-emerald-50 border-emerald-600 ring-2 ring-emerald-500/20 text-emerald-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="streamline-ultimate-color:layout-dashboard" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Virtual Simulator</span>
              <span class="text-[10px] text-slate-500 font-mono font-normal">Uji Tanpa Hardware</span>
            </button>
          </div>
        </div>

        <!-- Detail Kontrol Berdasarkan Tab -->
        <!-- Tab 1: Serial Port -->
        <div v-if="activeTab === 'SERIAL'" class="bg-slate-50 rounded-xl p-4 border border-slate-200 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-800">Koneksi USB / Serial Port (Web Serial API)</span>
            <span
              class="text-[10px] px-2 py-0.5 rounded font-mono font-bold"
              :class="printerStore.isWebSerialSupported ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'"
            >
              {{ printerStore.isWebSerialSupported ? '✓ API Didukung' : '⚠️ Perlu Chrome/Edge' }}
            </span>
          </div>
          <p class="text-xs text-slate-600">
            Digunakan untuk <strong>printer berkabel USB</strong> serta <strong>printer Bluetooth Classic SPP</strong> (non-BLE) yang sudah di-pair di Windows/Mac (akan muncul sebagai port COM / Serial virtual).
          </p>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
            <div>
              <label class="block text-[11px] font-bold text-slate-600 mb-1">Baud Rate Port</label>
              <select
                v-model="localBaudRate"
                class="w-full bg-white border border-slate-300 rounded-lg px-2.5 py-1.5 text-xs text-slate-800 focus:outline-none focus:ring-2 focus:ring-emerald-500"
              >
                <option v-for="opt in baudRateOptions" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </option>
              </select>
            </div>

            <div class="flex items-end gap-2">
              <button
                type="button"
                @click="handleConnectSerial"
                :disabled="printerStore.isConnecting"
                class="flex-1 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold py-2 px-3 rounded-lg border border-emerald-700 shadow-xs transition active:scale-95 flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-50"
              >
                <Icon icon="streamline-ultimate-color:usb-port-1" class="w-4 h-4" />
                <span>{{ printerStore.isConnecting ? 'Membuka Port...' : 'Pilih & Hubungkan Port' }}</span>
              </button>

              <button
                v-if="printerStore.connectionType === 'SERIAL' && printerStore.isConnected"
                type="button"
                @click="handleDisconnectSerial"
                class="bg-rose-50 hover:bg-rose-100 text-rose-700 text-xs font-bold py-2 px-3 rounded-lg border border-rose-300 transition active:scale-95 cursor-pointer"
                title="Putuskan sambungan serial"
              >
                Putuskan
              </button>
            </div>
          </div>
        </div>

        <!-- Tab 2: Bluetooth -->
        <div v-if="activeTab === 'BLUETOOTH'" class="bg-slate-50 rounded-xl p-4 border border-slate-200 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-800">Koneksi Bluetooth (Web Bluetooth API)</span>
            <span
              class="text-[10px] px-2 py-0.5 rounded font-mono font-bold"
              :class="printerStore.isWebBluetoothSupported ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'"
            >
              {{ printerStore.isWebBluetoothSupported ? '✓ API Didukung' : '⚠️ Perlu Chrome/Edge' }}
            </span>
          </div>
          <p class="text-xs text-slate-600">
            Digunakan untuk printer thermal nirkabel / portable 58mm atau 80mm yang tersambung via Bluetooth SPP/ESC-POS GATT Service.
          </p>

          <div class="flex items-center gap-2 pt-1">
            <button
              type="button"
              @click="handleConnectBluetooth"
              :disabled="printerStore.isConnecting"
              class="flex-1 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold py-2 px-3 rounded-lg border border-emerald-700 shadow-xs transition active:scale-95 flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <Icon icon="streamline-ultimate-color:bluetooth-transfer" class="w-4 h-4" />
              <span>{{ printerStore.isConnecting ? 'Mencari Bluetooth...' : 'Pindai & Sambungkan Bluetooth' }}</span>
            </button>

            <button
              v-if="printerStore.connectionType === 'BLUETOOTH' && printerStore.isConnected"
              type="button"
              @click="handleDisconnectBluetooth"
              class="bg-rose-50 hover:bg-rose-100 text-rose-700 text-xs font-bold py-2 px-3 rounded-lg border border-rose-300 transition active:scale-95 cursor-pointer"
              title="Putuskan bluetooth"
            >
              Putuskan
            </button>
          </div>

          <!-- Troubleshooting Box -->
          <div class="bg-amber-50/80 border border-amber-200 rounded-lg p-3 text-[11px] text-amber-900 space-y-1">
            <div class="font-bold flex items-center gap-1 text-amber-800">
              <Icon icon="streamline-ultimate-color:alert-triangle" class="w-3.5 h-3.5" />
              <span>Printer terbaca "Unsupported Device" atau Tidak Muncul?</span>
            </div>
            <ul class="list-disc list-inside space-y-0.5 text-slate-700 pl-0.5">
              <li>
                <strong>BLE vs Bluetooth Classic:</strong> Browser (Chrome/Edge) menggunakan <em>Web Bluetooth (BLE)</em>. Printer model lama hanya mendukung <em>Bluetooth Classic SPP (RFCOMM)</em> sehingga browser menandainya sebagai <em>unsupported</em>.
              </li>
              <li>
                <strong>Solusi Printer Classic SPP:</strong> Gunakan tab <strong>Kabel USB / Serial</strong> di atas, atau sambungkan via USB ke komputer/laptop. Port Serial mendukung 100% semua printer thermal tanpa batasan protokol BLE.
              </li>
              <li>
                <strong>Sudah Dipasangkan (Paired) di OS:</strong> Putuskan koneksi dari aplikasi lain (misal Android/laptop lain) karena printer thermal bluetooth hanya bisa melayani 1 perangkat aktif dalam satu waktu.
              </li>
            </ul>
          </div>
        </div>

        <!-- Tab 3: Virtual Simulator -->
        <div v-if="activeTab === 'VIRTUAL'" class="bg-slate-50 rounded-xl p-4 border border-slate-200 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-800">Simulator Printer Thermal Internal</span>
            <span class="bg-emerald-100 text-emerald-800 text-[10px] px-2 py-0.5 rounded font-mono font-bold">
              ✓ Siap Digunakan
            </span>
          </div>
          <p class="text-xs text-slate-600">
            Simulator berguna jika komputer kasir belum disambungkan ke printer fisik. Struk akan diproses langsung secara virtual tanpa memunculkan dialog pop-up browser apapun.
          </p>

          <div class="flex items-center gap-2 pt-1">
            <button
              type="button"
              @click="handleSetVirtual('58mm')"
              class="flex-1 bg-white hover:bg-slate-100 text-slate-800 border border-slate-300 text-xs font-bold py-2 px-3 rounded-lg shadow-xs transition active:scale-95 cursor-pointer"
            >
              Aktifkan Simulator 58mm
            </button>
            <button
              type="button"
              @click="handleSetVirtual('80mm')"
              class="flex-1 bg-white hover:bg-slate-100 text-slate-800 border border-slate-300 text-xs font-bold py-2 px-3 rounded-lg shadow-xs transition active:scale-95 cursor-pointer"
            >
              Aktifkan Simulator 80mm
            </button>
          </div>
        </div>

        <!-- Section 2: Format Kertas & Otomasi -->
        <div class="border-t border-slate-200 pt-4 space-y-4">
          <!-- Format Kertas -->
          <div>
            <label class="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-2">
              Ukuran Lebar Kertas Struk
            </label>
            <div class="grid grid-cols-2 gap-3">
              <button
                type="button"
                @click="selectPaperWidth('58mm')"
                class="flex items-center justify-between p-3 rounded-xl border text-left cursor-pointer transition"
                :class="printerStore.paperWidth === '58mm' ? 'bg-emerald-50 border-emerald-600 ring-2 ring-emerald-500/20' : 'bg-slate-50 border-slate-300 hover:bg-slate-100'"
              >
                <div>
                  <div class="text-xs font-bold text-slate-900">58 mm (Kecil)</div>
                  <div class="text-[10px] text-slate-500 font-mono">32 Karakter per Baris</div>
                </div>
                <span v-if="printerStore.paperWidth === '58mm'" class="text-emerald-700 text-sm font-bold">✓</span>
              </button>

              <button
                type="button"
                @click="selectPaperWidth('80mm')"
                class="flex items-center justify-between p-3 rounded-xl border text-left cursor-pointer transition"
                :class="printerStore.paperWidth === '80mm' ? 'bg-emerald-50 border-emerald-600 ring-2 ring-emerald-500/20' : 'bg-slate-50 border-slate-300 hover:bg-slate-100'"
              >
                <div>
                  <div class="text-xs font-bold text-slate-900">80 mm (Standar)</div>
                  <div class="text-[10px] text-slate-500 font-mono">42 Karakter per Baris</div>
                </div>
                <span v-if="printerStore.paperWidth === '80mm'" class="text-emerald-700 text-sm font-bold">✓</span>
              </button>
            </div>
          </div>

          <!-- Switch Otomasi Kasir -->
          <div>
            <label class="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-2">
              Perilaku Otomatis Pencetakan Struk
            </label>
            <div class="space-y-2.5 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
              <!-- Switch Auto-Print -->
              <label class="flex items-center justify-between cursor-pointer">
                <div>
                  <div class="text-xs font-bold text-slate-900">Cetak Otomatis saat Transaksi Selesai (Tanpa Dialog)</div>
                  <div class="text-[11px] text-slate-500">
                    Struk langsung terkirim ke printer thermal saat kasir menekan tombol Selesai Bayar tanpa membuka popup dialog browser.
                  </div>
                </div>
                <input
                  type="checkbox"
                  :checked="printerStore.autoPrint"
                  @change="toggleAutoPrint"
                  class="w-4 h-4 rounded text-emerald-600 focus:ring-emerald-500 border-slate-300 cursor-pointer"
                />
              </label>

              <div class="border-t border-slate-200 pt-2 flex items-center justify-between">
                <div>
                  <div class="text-xs font-bold text-slate-900">Buka Laci Kasir Otomatis (Cash Drawer Kick)</div>
                  <div class="text-[11px] text-slate-500">Kirim sinyal pulsa pin laci (ESC p) ke laci kasir saat nota dicetak.</div>
                </div>
                <input
                  type="checkbox"
                  :checked="printerStore.openDrawer"
                  @change="toggleOpenDrawer"
                  class="w-4 h-4 rounded text-emerald-600 focus:ring-emerald-500 border-slate-300 cursor-pointer"
                />
              </div>

              <div class="border-t border-slate-200 pt-2 flex items-center justify-between">
                <div>
                  <div class="text-xs font-bold text-slate-900">Potong Kertas Otomatis (Auto-Cut)</div>
                  <div class="text-[11px] text-slate-500">Kirim instruksi autocutter (GS V) setelah rincian nota selesai dicetak.</div>
                </div>
                <input
                  type="checkbox"
                  :checked="printerStore.autoCut"
                  @change="toggleAutoCut"
                  class="w-4 h-4 rounded text-emerald-600 focus:ring-emerald-500 border-slate-300 cursor-pointer"
                />
              </div>
            </div>
          </div>

          <!-- Preview Struk Terakhir (Jika ada) -->
          <div v-if="printerStore.lastPrintedText" class="pt-2">
            <label class="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
              Pratinjau Struk Terakhir (Waktu: {{ printerStore.lastPrintTime }})
            </label>
            <div class="bg-slate-900 text-emerald-400 p-3 rounded-xl font-mono text-[11px] whitespace-pre-wrap max-h-36 overflow-y-auto border border-slate-800 shadow-inner">
              {{ printerStore.lastPrintedText }}
            </div>
          </div>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between">
        <div class="text-[11px] text-slate-500 font-mono">
          Status: <span class="font-bold text-slate-700">{{ printerStore.statusMessage }}</span>
        </div>
        <button
          type="button"
          @click="emit('close')"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-5 py-2 rounded-xl border border-emerald-700 shadow-xs transition active:scale-95 cursor-pointer"
        >
          Selesai & Simpan
        </button>
      </div>
    </div>
  </div>
</template>
