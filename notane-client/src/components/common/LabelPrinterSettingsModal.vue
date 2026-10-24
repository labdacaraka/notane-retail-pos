<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import {
  useLabelPrinterStore,
  type LabelConnectionType,
  type LabelSizePreset,
  type LabelSensorType,
  type LabelDarkness,
} from '@/stores/labelPrinterStore'
import { Icon } from '@iconify/vue'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const labelStore = useLabelPrinterStore()
const activeTab = ref<LabelConnectionType>(labelStore.connectionType || 'SYSTEM')
const localBaudRate = ref<number>(labelStore.baudRate || 9600)
const isTesting = ref<boolean>(false)
const testMessage = ref<string>('')
const testError = ref<string>('')

onMounted(() => {
  labelStore.fetchSystemPrinters()
})

watch(() => props.show, (val) => {
  if (val) {
    activeTab.value = labelStore.connectionType || 'SYSTEM'
    labelStore.fetchSystemPrinters()
  }
})

const baudRateOptions = [
  { label: '9600 bps (Standar Xprinter/TSC/Gprinter)', value: 9600 },
  { label: '19200 bps', value: 19200 },
  { label: '38400 bps', value: 38400 },
  { label: '115200 bps (Kecepatan Tinggi)', value: 115200 },
]

const sizePresets: { id: LabelSizePreset; label: string; desc: string; icon: string }[] = [
  { id: '40x30', label: '40 × 30 mm', desc: 'Standar Rak & Barcode Minimarket', icon: 'fluent-color:tag-24' },
  { id: '50x30', label: '50 × 30 mm', desc: 'Label Produk Kompak', icon: 'fluent-color:tag-24' },
  { id: '60x40', label: '60 × 40 mm', desc: 'Display Luas Supermarket', icon: 'fluent-color:tag-multiple-24' },
  { id: '70x50', label: '70 × 50 mm', desc: 'Informasi Lengkap & Rinci', icon: 'fluent-color:tag-multiple-24' },
  { id: '80x50', label: '80 × 50 mm', desc: 'Label Box Dus / Gudang', icon: 'streamline-ultimate-color:warehouse-cart-packages-2' },
  { id: '100x150', label: '100 × 150 mm', desc: 'Stiker Resi / Ekspedisi (A6)', icon: 'streamline-ultimate-color:delivery-truck-cargo' },
  { id: 'a4_grid', label: 'Lembaran A4 Grid', desc: 'Kertas Stiker Tom & Jerry 2 Kolom', icon: 'fluent-color:document-page-break-24' },
  { id: 'custom', label: 'Ukuran Kustom', desc: 'Tentukan Lebar × Tinggi Manual', icon: 'fluent-color:ruler-24' },
]

async function handleConnectSerial() {
  testMessage.value = ''
  testError.value = ''
  try {
    const res = await labelStore.connectSerial(localBaudRate.value)
    testMessage.value = res.message
    setTimeout(() => {
      testMessage.value = ''
    }, 4000)
  } catch (err: any) {
    testError.value = err.message || 'Gagal menyambungkan port serial'
  }
}

async function handleDisconnectSerial() {
  await labelStore.disconnectSerial()
  testMessage.value = 'Port serial printer label berhasil diputuskan'
}

async function handleConnectBluetooth() {
  testMessage.value = ''
  testError.value = ''
  try {
    const res = await labelStore.connectBluetooth()
    testMessage.value = res.message
    setTimeout(() => {
      testMessage.value = ''
    }, 4000)
  } catch (err: any) {
    testError.value = err.message || 'Gagal menyambungkan bluetooth'
  }
}

function handleDisconnectBluetooth() {
  labelStore.disconnectBluetooth()
  testMessage.value = 'Koneksi bluetooth berhasil diputuskan'
}

function handleSetSystem() {
  labelStore.setSystemDriver()
  testMessage.value = 'Mode Driver Sistem (OS Default / Browser Print) diaktifkan'
  testError.value = ''
}

function handleSetVirtual() {
  labelStore.setVirtualSimulator()
  testMessage.value = 'Simulator label diaktifkan untuk pengujian'
  testError.value = ''
}

async function handleTestPrint() {
  isTesting.value = true
  testMessage.value = ''
  testError.value = ''

  try {
    const res = await labelStore.testPrint()
    testMessage.value = res.message
    setTimeout(() => {
      testMessage.value = ''
    }, 4000)
  } catch (err: any) {
    testError.value = err.message || 'Gagal melakukan uji cetak label'
  } finally {
    isTesting.value = false
  }
}

function selectPreset(preset: LabelSizePreset) {
  labelStore.sizePreset = preset
  labelStore.saveConfig()
}

function setSensorType(sensor: LabelSensorType) {
  labelStore.sensorType = sensor
  labelStore.saveConfig()
}

function setDarkness(d: LabelDarkness) {
  labelStore.darkness = d
  labelStore.saveConfig()
}

function setSpeed(s: number) {
  labelStore.printSpeed = s
  labelStore.saveConfig()
}

function handleSave() {
  labelStore.saveConfig()
  emit('close')
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
    @click.self="emit('close')"
  >
    <div
      class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-3xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-teal-100 border border-teal-300 flex items-center justify-center shadow-xs">
            <Icon icon="streamline-ultimate-color:tags-1" class="w-6 h-6" />
          </div>
          <div>
            <h2 class="text-base font-bold text-slate-900 leading-tight">Pengaturan Printer Label Barcode & Rak</h2>
            <p class="text-xs text-slate-500">Konfigurasi koneksi hardware printer sticker, ukuran label, sensor, dan uji cetak</p>
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
            'bg-emerald-50 border-emerald-300 text-emerald-900': labelStore.isConnected,
            'bg-slate-50 border-slate-300 text-slate-700': !labelStore.isConnected
          }"
        >
          <div class="flex items-center gap-3">
            <span
              class="w-3.5 h-3.5 rounded-full flex items-center justify-center"
              :class="labelStore.isConnected ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'"
            >
              <span v-if="labelStore.isConnected" class="w-2 h-2 rounded-full bg-white"></span>
            </span>
            <div>
              <div class="text-xs font-bold font-mono">
                {{ labelStore.isConnected ? 'SIAP MENCETAK LABEL' : 'BELUM TERHUBUNG' }}
              </div>
              <div class="text-xs text-slate-600 font-sans">
                {{ labelStore.printerName }} • {{ labelStore.effectiveWidthMm }}×{{ labelStore.effectiveHeightMm }} mm • Mode {{ labelStore.connectionType }}
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <button
              @click="handleTestPrint"
              :disabled="isTesting"
              class="bg-teal-600 hover:bg-teal-700 text-white text-xs font-bold px-3.5 py-1.5 rounded-lg border border-teal-700 shadow-xs transition active:scale-95 flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <Icon icon="streamline-ultimate-color:print-text" class="w-4 h-4" />
              <span>{{ isTesting ? 'Mencetak...' : 'Uji Cetak Label' }}</span>
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

        <!-- 1. Tipe Koneksi Printer Label -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-2">Pilih Jalur Koneksi Printer Label</label>

          <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
            <!-- Tab Driver Sistem -->
            <button
              type="button"
              @click="activeTab = 'SYSTEM'; handleSetSystem()"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'SYSTEM' ? 'bg-teal-50 border-teal-600 ring-2 ring-teal-500/20 text-teal-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="fluent-color:desktop-speaker-24" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Driver Sistem</span>
              <span class="text-[10px] text-slate-500 font-normal">OS / Browser Print</span>
            </button>

            <!-- Tab Serial Port -->
            <button
              type="button"
              @click="activeTab = 'SERIAL'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'SERIAL' ? 'bg-teal-50 border-teal-600 ring-2 ring-teal-500/20 text-teal-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="streamline-ultimate-color:usb-port-1" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Serial USB Port</span>
              <span class="text-[10px] text-slate-500 font-mono font-normal">Direct TSPL / COM</span>
            </button>

            <!-- Tab Bluetooth -->
            <button
              type="button"
              @click="activeTab = 'BLUETOOTH'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'BLUETOOTH' ? 'bg-teal-50 border-teal-600 ring-2 ring-teal-500/20 text-teal-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="streamline-ultimate-color:bluetooth-transfer" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Bluetooth (BLE)</span>
              <span class="text-[10px] text-slate-500 font-normal">Portable Labeler</span>
            </button>

            <!-- Tab Simulator -->
            <button
              type="button"
              @click="activeTab = 'VIRTUAL'; handleSetVirtual()"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-center transition cursor-pointer"
              :class="activeTab === 'VIRTUAL' ? 'bg-teal-50 border-teal-600 ring-2 ring-teal-500/20 text-teal-900 font-bold' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Icon icon="streamline-ultimate-color:layout-dashboard" class="w-6 h-6 mb-1.5" />
              <span class="text-xs">Simulator Virtual</span>
              <span class="text-[10px] text-slate-500 font-normal">Uji Pratinjau Layar</span>
            </button>
          </div>
        </div>

        <!-- Detail Per-Tab Koneksi -->
        <!-- Tab SYSTEM -->
        <div v-if="activeTab === 'SYSTEM'" class="bg-teal-50/60 rounded-xl p-4 border border-teal-200 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-teal-950 flex items-center gap-1.5">
              <Icon icon="fluent-color:checkmark-circle-24" class="w-4 h-4" />
              Mode Driver Sistem Operasi (Silent Print Windows / macOS / Linux)
            </span>
            <span class="text-[10px] px-2 py-0.5 rounded font-mono font-bold bg-teal-100 text-teal-800">
              Silent Print
            </span>
          </div>

          <p class="text-xs text-slate-600 leading-relaxed">
            Mencetak otomatis langsung ke printer OS (seperti <strong>Xprinter XP-365B, XP-420B, TSC TE200, Zebra ZD220, EPSON, Brother</strong>) tanpa membuka peramban web dan tanpa dialog cetak.
          </p>

          <!-- Dropdown Pemilihan Printer Sistem -->
          <div class="bg-white p-3 rounded-lg border border-teal-200 space-y-2">
            <div class="flex items-center justify-between">
              <label class="text-[11px] font-bold text-slate-700">Pilih Printer Sistem Terpasang:</label>
              <button
                type="button"
                @click="labelStore.fetchSystemPrinters()"
                :disabled="labelStore.isFetchingPrinters"
                class="text-[10px] text-teal-700 hover:text-teal-900 font-bold flex items-center gap-1 cursor-pointer"
              >
                <Icon icon="fluent:arrow-clockwise-16-regular" class="w-3.5 h-3.5" :class="{ 'animate-spin': labelStore.isFetchingPrinters }" />
                <span>Pindai Ulang Printer</span>
              </button>
            </div>

            <div v-if="labelStore.systemPrinters.length > 0">
              <select
                v-model="labelStore.printerName"
                @change="labelStore.saveConfig()"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-2.5 py-2 text-xs font-semibold text-slate-800 focus:outline-none focus:ring-2 focus:ring-teal-500"
              >
                <option
                  v-for="p in labelStore.systemPrinters"
                  :key="p.name"
                  :value="p.name"
                >
                  {{ p.name }} {{ p.is_default ? '(Default OS)' : '' }}
                </option>
              </select>
            </div>
            <div v-else class="flex items-center gap-2">
              <input
                v-model="labelStore.printerName"
                @blur="labelStore.saveConfig()"
                placeholder="Nama Printer Driver OS (contoh: EPSON L3550 / Xprinter XP-365B)"
                class="flex-1 bg-slate-50 border border-slate-300 rounded-lg px-2.5 py-1.5 text-xs text-slate-800 focus:outline-none focus:ring-2 focus:ring-teal-500"
              />
            </div>
            <p class="text-[10px] text-slate-500 italic">
              * Perintah cetak akan dikirimkan otomatis langsung ke printer di atas tanpa memunculkan dialog cetak OS.
            </p>
          </div>
        </div>

        <!-- Tab SERIAL -->
        <div v-if="activeTab === 'SERIAL'" class="bg-slate-50 rounded-xl p-4 border border-slate-200 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-800">Koneksi Direct Serial / USB (Web Serial API)</span>
            <span
              class="text-[10px] px-2 py-0.5 rounded font-mono font-bold"
              :class="labelStore.isWebSerialSupported ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'"
            >
              {{ labelStore.isWebSerialSupported ? '✓ API Didukung' : '⚠️ Perlu Browser Chrome/Edge' }}
            </span>
          </div>
          <p class="text-xs text-slate-600">
            Mengirim instruksi TSPL/CPCL langsung ke port kabel USB tanpa melalui driver cetak OS.
          </p>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
            <div>
              <label class="block text-[11px] font-bold text-slate-600 mb-1">Baud Rate Port</label>
              <select
                v-model="localBaudRate"
                class="w-full bg-white border border-slate-300 rounded-lg px-2.5 py-1.5 text-xs text-slate-800 focus:outline-none focus:ring-2 focus:ring-teal-500"
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
                :disabled="labelStore.isConnecting"
                class="flex-1 bg-teal-600 hover:bg-teal-700 text-white text-xs font-bold py-2 px-3 rounded-lg border border-teal-700 shadow-xs transition active:scale-95 flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-50"
              >
                <Icon icon="streamline-ultimate-color:usb-port-1" class="w-4 h-4" />
                <span>{{ labelStore.isConnecting ? 'Membuka Port...' : 'Pilih Port USB' }}</span>
              </button>

              <button
                v-if="labelStore.connectionType === 'SERIAL' && labelStore.isConnected"
                type="button"
                @click="handleDisconnectSerial"
                class="bg-rose-50 hover:bg-rose-100 text-rose-700 text-xs font-bold py-2 px-3 rounded-lg border border-rose-300 transition active:scale-95 cursor-pointer"
              >
                Putuskan
              </button>
            </div>
          </div>
        </div>

        <!-- Tab BLUETOOTH -->
        <div v-if="activeTab === 'BLUETOOTH'" class="bg-slate-50 rounded-xl p-4 border border-slate-200 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-800">Koneksi Bluetooth Wireless (Web Bluetooth API)</span>
            <span
              class="text-[10px] px-2 py-0.5 rounded font-mono font-bold"
              :class="labelStore.isWebBluetoothSupported ? 'bg-emerald-100 text-emerald-800' : 'bg-amber-100 text-amber-800'"
            >
              {{ labelStore.isWebBluetoothSupported ? '✓ BLE Didukung' : '⚠️ Perlu Browser Chrome/Edge' }}
            </span>
          </div>
          <p class="text-xs text-slate-600">
            Khusus untuk printer label portable mini nirkabel berkemampuan Bluetooth Low Energy (BLE).
          </p>

          <div class="flex items-center gap-2 pt-1">
            <button
              type="button"
              @click="handleConnectBluetooth"
              :disabled="labelStore.isConnecting"
              class="flex-1 bg-teal-600 hover:bg-teal-700 text-white text-xs font-bold py-2 px-3 rounded-lg border border-teal-700 shadow-xs transition active:scale-95 flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <Icon icon="streamline-ultimate-color:bluetooth-transfer" class="w-4 h-4" />
              <span>{{ labelStore.isConnecting ? 'Mencari Bluetooth...' : 'Pindai & Sambungkan Bluetooth' }}</span>
            </button>

            <button
              v-if="labelStore.connectionType === 'BLUETOOTH' && labelStore.isConnected"
              type="button"
              @click="handleDisconnectBluetooth"
              class="bg-rose-50 hover:bg-rose-100 text-rose-700 text-xs font-bold py-2 px-3 rounded-lg border border-rose-300 transition active:scale-95 cursor-pointer"
            >
              Putuskan
            </button>
          </div>
        </div>

        <!-- Tab VIRTUAL -->
        <div v-if="activeTab === 'VIRTUAL'" class="bg-slate-50 rounded-xl p-4 border border-slate-200 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-800">Simulator Internal Notane POS</span>
            <span class="bg-emerald-100 text-emerald-800 text-[10px] px-2 py-0.5 rounded font-mono font-bold">✓ Aktif</span>
          </div>
          <p class="text-xs text-slate-600">
            Simulator digunakan untuk menguji desain tata letak label harga di layar tanpa memerlukan printer fisik.
          </p>
        </div>

        <!-- 2. Ukuran Label / Kertas Sticker -->
        <div class="border-t border-slate-200 pt-5 space-y-3">
          <label class="block text-xs font-bold text-slate-800 uppercase tracking-wider">
            Ukuran Kertas & Dimensi Label Sticker
          </label>

          <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
            <button
              v-for="p in sizePresets"
              :key="p.id"
              type="button"
              @click="selectPreset(p.id)"
              class="p-3 rounded-xl border text-left transition cursor-pointer flex flex-col justify-between"
              :class="labelStore.sizePreset === p.id ? 'bg-teal-50 border-teal-600 ring-2 ring-teal-500/20' : 'bg-slate-50 border-slate-300 hover:bg-slate-100'"
            >
              <div>
                <div class="flex items-center justify-between mb-1">
                  <Icon :icon="p.icon" class="w-5 h-5 text-teal-600" />
                  <span v-if="labelStore.sizePreset === p.id" class="text-teal-700 text-xs font-black">✓</span>
                </div>
                <div class="text-xs font-black text-slate-900">{{ p.label }}</div>
                <div class="text-[10px] text-slate-500 mt-0.5 leading-snug">{{ p.desc }}</div>
              </div>
            </button>
          </div>

          <!-- Input Dimensi Kustom jika opsi custom dipilih -->
          <div
            v-if="labelStore.sizePreset === 'custom'"
            class="bg-amber-50/70 border border-amber-200 rounded-xl p-3.5 grid grid-cols-2 gap-3"
          >
            <div>
              <label class="block text-[11px] font-bold text-amber-900 mb-1">Lebar Label (mm)</label>
              <input
                v-model.number="labelStore.customWidthMm"
                @change="labelStore.saveConfig"
                type="number"
                min="20"
                max="210"
                class="w-full bg-white border border-amber-300 rounded-lg px-2.5 py-1.5 text-xs text-slate-800 font-bold focus:ring-2 focus:ring-teal-500"
              />
            </div>
            <div>
              <label class="block text-[11px] font-bold text-amber-900 mb-1">Tinggi Label (mm)</label>
              <input
                v-model.number="labelStore.customHeightMm"
                @change="labelStore.saveConfig"
                type="number"
                min="10"
                max="297"
                class="w-full bg-white border border-amber-300 rounded-lg px-2.5 py-1.5 text-xs text-slate-800 font-bold focus:ring-2 focus:ring-teal-500"
              />
            </div>
          </div>
        </div>

        <!-- 3. Sensor Sticker, Kepekatan, & Otomasi -->
        <div class="border-t border-slate-200 pt-5 space-y-4">
          <label class="block text-xs font-bold text-slate-800 uppercase tracking-wider">
            Kalibrasi Sensor & Kualitas Cetak
          </label>

          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <!-- Tipe Sensor -->
            <div class="bg-slate-50 p-3 rounded-xl border border-slate-200 space-y-1.5">
              <label class="block text-[11px] font-bold text-slate-700">Tipe Sensor Kertas</label>
              <div class="grid grid-cols-3 gap-1">
                <button
                  type="button"
                  @click="setSensorType('GAP')"
                  class="px-2 py-1 text-[11px] rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.sensorType === 'GAP' ? 'bg-teal-600 text-white border-teal-700' : 'bg-white border-slate-300 text-slate-700'"
                >
                  Gap (Celah)
                </button>
                <button
                  type="button"
                  @click="setSensorType('BLACK_MARK')"
                  class="px-2 py-1 text-[11px] rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.sensorType === 'BLACK_MARK' ? 'bg-teal-600 text-white border-teal-700' : 'bg-white border-slate-300 text-slate-700'"
                >
                  Marking
                </button>
                <button
                  type="button"
                  @click="setSensorType('CONTINUOUS')"
                  class="px-2 py-1 text-[11px] rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.sensorType === 'CONTINUOUS' ? 'bg-teal-600 text-white border-teal-700' : 'bg-white border-slate-300 text-slate-700'"
                >
                  Roll Polos
                </button>
              </div>
              <p class="text-[10px] text-slate-500 mt-1">Celah gap antar stiker biasanya 2mm - 3mm.</p>
            </div>

            <!-- Kepekatan (Darkness / Density) -->
            <div class="bg-slate-50 p-3 rounded-xl border border-slate-200 space-y-1.5">
              <label class="block text-[11px] font-bold text-slate-700">Kepekatan Cetak (Density)</label>
              <div class="grid grid-cols-4 gap-1">
                <button
                  v-for="d in (['LIGHT', 'NORMAL', 'DARK', 'VERY_DARK'] as LabelDarkness[])"
                  :key="d"
                  type="button"
                  @click="setDarkness(d)"
                  class="px-1 py-1 text-[10px] rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.darkness === d ? 'bg-teal-600 text-white border-teal-700' : 'bg-white border-slate-300 text-slate-700'"
                >
                  {{ d === 'LIGHT' ? 'Tipis' : d === 'NORMAL' ? 'Normal' : d === 'DARK' ? 'Pekat' : 'Hitam+' }}
                </button>
              </div>
              <p class="text-[10px] text-slate-500 mt-1">Sesuaikan jika garis barcode kurang terbaca barcode scanner.</p>
            </div>

            <!-- Kecepatan Cetak (Speed) -->
            <div class="bg-slate-50 p-3 rounded-xl border border-slate-200 space-y-1.5">
              <label class="block text-[11px] font-bold text-slate-700">Kecepatan Cetak (Speed)</label>
              <div class="grid grid-cols-3 gap-1">
                <button
                  v-for="s in [2, 3, 5]"
                  :key="s"
                  type="button"
                  @click="setSpeed(s)"
                  class="px-2 py-1 text-[11px] rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.printSpeed === s ? 'bg-teal-600 text-white border-teal-700' : 'bg-white border-slate-300 text-slate-700'"
                >
                  {{ s }} IPS {{ s === 2 ? '(Tajam)' : s === 3 ? '(Std)' : '(Cepat)' }}
                </button>
              </div>
              <p class="text-[10px] text-slate-500 mt-1">IPS = Inch per Second.</p>
            </div>
          </div>

          <!-- Perilaku Otomasi Cetak Label -->
          <div class="bg-slate-50 p-3.5 rounded-xl border border-slate-200 space-y-2.5">
            <div class="flex items-center justify-between">
              <div>
                <div class="text-xs font-bold text-slate-900">Kuantitas Cetak Default Saat Pilih Produk</div>
                <div class="text-[11px] text-slate-500">
                  Tentukan apakah kuantitas awal mencetak 1 stiker atau otomatis mengikuti jumlah sisa stok produk.
                </div>
              </div>
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  @click="labelStore.defaultQtyMode = 'SINGLE'; labelStore.saveConfig()"
                  class="px-2.5 py-1 text-xs rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.defaultQtyMode === 'SINGLE' ? 'bg-teal-600 text-white border-teal-700' : 'bg-white text-slate-700 border-slate-300'"
                >
                  1 Label per Item
                </button>
                <button
                  type="button"
                  @click="labelStore.defaultQtyMode = 'STOCK'; labelStore.saveConfig()"
                  class="px-2.5 py-1 text-xs rounded-lg font-bold border transition cursor-pointer"
                  :class="labelStore.defaultQtyMode === 'STOCK' ? 'bg-teal-600 text-white border-teal-700' : 'bg-white text-slate-700 border-slate-300'"
                >
                  Sesuai Stok Produk
                </button>
              </div>
            </div>

            <div class="border-t border-slate-200 pt-2 flex items-center justify-between">
              <div>
                <div class="text-xs font-bold text-slate-900">Bersihkan Antrean Label Setelah Selesai Mencetak</div>
                <div class="text-[11px] text-slate-500">
                  Otomatis kosongkan daftar stiker yang dicentang setelah dialog/perintah cetak selesai dijalankan.
                </div>
              </div>
              <input
                type="checkbox"
                v-model="labelStore.autoClearAfterPrint"
                @change="labelStore.saveConfig"
                class="w-4 h-4 rounded text-teal-600 focus:ring-teal-500 border-slate-300 cursor-pointer"
              />
            </div>
          </div>
        </div>

        <!-- 4. Log / Pratinjau Uji Cetak Label Terakhir -->
        <div v-if="labelStore.lastPrintedText" class="pt-1">
          <label class="block text-xs font-bold text-slate-700 uppercase tracking-wider mb-1.5">
            Log Uji Cetak Terakhir (Waktu: {{ labelStore.lastPrintTime }})
          </label>
          <div class="bg-slate-900 text-teal-400 p-3 rounded-xl font-mono text-[11px] whitespace-pre-wrap max-h-36 overflow-y-auto border border-slate-800 shadow-inner">
            {{ labelStore.lastPrintedText }}
          </div>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between">
        <div class="text-[11px] text-slate-500 font-mono">
          Status: <span class="font-bold text-slate-700">{{ labelStore.statusMessage }}</span>
        </div>
        <button
          type="button"
          @click="handleSave"
          class="bg-teal-600 hover:bg-teal-700 text-white font-bold text-xs px-5 py-2 rounded-xl border border-teal-700 shadow-xs transition active:scale-95 cursor-pointer"
        >
          Selesai & Simpan Konfigurasi
        </button>
      </div>
    </div>
  </div>
</template>
