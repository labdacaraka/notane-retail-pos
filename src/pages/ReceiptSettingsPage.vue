<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { api } from '@/services/api'
import { usePrinterStore } from '@/stores/printerStore'
import { Icon } from '@iconify/vue'
import {
  Save,
  RotateCcw,
  Printer,
  CheckCircle2,
  AlertCircle,
  Store,
  FileText,
  Sliders,
  Eye,
} from 'lucide-vue-next'

const printerStore = usePrinterStore()

interface ReceiptSettingsState {
  id?: string
  store_name: string
  store_tagline: string
  store_address: string
  store_phone: string
  store_website: string
  tax_id: string
  header_note: string
  footer_note_1: string
  footer_note_2: string
  social_media: string
  paper_width: '58mm' | '80mm'
  show_phone: boolean
  show_address: boolean
  show_cashier_name: boolean
  show_barcode: boolean
  show_tax_summary: boolean
}

const form = reactive<ReceiptSettingsState>({
  store_name: 'NOTANE RETAIL ENTERPRISE',
  store_tagline: 'Solusi Ritel Pintar & Modern',
  store_address: 'Jl. Surya Kencana No. 88, Bogor Tengah, Jawa Barat',
  store_phone: '0812-3456-7890',
  store_website: 'www.notaneretail.id',
  tax_id: '01.234.567.8-901.000',
  header_note: 'Selamat Datang & Selamat Berbelanja',
  footer_note_1: 'Barang yang sudah dibeli tidak dapat ditukar/dikembalikan',
  footer_note_2: 'Kritik & Saran WA: 0812-3456-7890 | Terima Kasih',
  social_media: 'IG & TikTok: @notane.retail',
  paper_width: '58mm',
  show_phone: true,
  show_address: true,
  show_cashier_name: true,
  show_barcode: true,
  show_tax_summary: true,
})

const isLoading = ref(false)
const isSaving = ref(false)
const notification = ref<{ type: 'success' | 'error'; message: string } | null>(null)
const activeTab = ref<'identity' | 'notes' | 'display'>('identity')

// Mock sample transaction data for live receipt preview
const mockReceipt = {
  invoiceNumber: 'INV/20260920/0042',
  date: '20-09-2026 11:25:00',
  cashierName: 'Asep Saepul (Kasir-01)',
  customerName: 'Bpk. Hendra Wijaya (MEMBER)',
  items: [
    { name: 'Kopi Susu Gula Aren 250ml', qty: 2, price: 18000, total: 36000 },
    { name: 'Roti Panggang Keju Coklat', qty: 1, price: 22000, total: 22000 },
    { name: 'Mineral Water 600ml', qty: 3, price: 5000, total: 15000 },
  ],
  subtotal: 73000,
  discount: 5000,
  tax: 7480,
  total: 75480,
  paymentMethod: 'CASH',
  paidAmount: 100000,
  change: 24520,
}

onMounted(async () => {
  await fetchSettings()
})

async function fetchSettings() {
  isLoading.value = true
  try {
    const res = await api.get('/settings/receipt')
    if (res.data?.success && res.data?.data) {
      const data = res.data.data
      form.id = data.id
      form.store_name = data.store_name || ''
      form.store_tagline = data.store_tagline || ''
      form.store_address = data.store_address || ''
      form.store_phone = data.store_phone || ''
      form.store_website = data.store_website || ''
      form.tax_id = data.tax_id || ''
      form.header_note = data.header_note || ''
      form.footer_note_1 = data.footer_note_1 || ''
      form.footer_note_2 = data.footer_note_2 || ''
      form.social_media = data.social_media || ''
      form.paper_width = data.paper_width === '80mm' ? '80mm' : '58mm'
      form.show_phone = data.show_phone !== false
      form.show_address = data.show_address !== false
      form.show_cashier_name = data.show_cashier_name !== false
      form.show_barcode = data.show_barcode !== false
      form.show_tax_summary = data.show_tax_summary !== false

      // Sync with printer store
      printerStore.receiptSettings = data
      printerStore.paperWidth = form.paper_width
    }
  } catch (err: any) {
    console.error('Gagal mengambil pengaturan struk:', err)
    showNotification('error', 'Gagal memuat pengaturan struk dari server database.')
  } finally {
    isLoading.value = false
  }
}

async function saveSettings() {
  isSaving.value = true
  try {
    const res = await api.put('/settings/receipt', form)
    if (res.data?.success) {
      showNotification('success', 'Pengaturan struk berhasil disimpan ke database!')
      printerStore.receiptSettings = res.data.data
      printerStore.paperWidth = form.paper_width
      printerStore.saveConfig()
    } else {
      showNotification('error', res.data?.message || 'Gagal menyimpan pengaturan.')
    }
  } catch (err: any) {
    console.error('Error saat menyimpan pengaturan struk:', err)
    showNotification('error', 'Terjadi kesalahan sistem saat menyimpan ke database.')
  } finally {
    isSaving.value = false
  }
}

function resetToDefault() {
  if (confirm('Kembalikan ke pengaturan struk bawaan pabrik?')) {
    form.store_name = 'NOTANE RETAIL ENTERPRISE'
    form.store_tagline = 'Solusi Ritel Pintar & Modern'
    form.store_address = 'Jl. Surya Kencana No. 88, Bogor Tengah, Jawa Barat'
    form.store_phone = '0812-3456-7890'
    form.store_website = 'www.notaneretail.id'
    form.tax_id = '01.234.567.8-901.000'
    form.header_note = 'Selamat Datang & Selamat Berbelanja'
    form.footer_note_1 = 'Barang yang sudah dibeli tidak dapat ditukar/dikembalikan'
    form.footer_note_2 = 'Kritik & Saran WA: 0812-3456-7890 | Terima Kasih'
    form.social_media = 'IG & TikTok: @notane.retail'
    form.paper_width = '58mm'
    form.show_phone = true
    form.show_address = true
    form.show_cashier_name = true
    form.show_barcode = true
    form.show_tax_summary = true
    showNotification('success', 'Formulir telah diisi dengan konfigurasi default. Klik Simpan untuk memperbarui database.')
  }
}

async function handleTestPrint() {
  try {
    // Temporarily update printer store receipt settings with current form value for testing
    printerStore.receiptSettings = { ...form }
    printerStore.paperWidth = form.paper_width

    const testReceiptData = {
      invoiceNumber: mockReceipt.invoiceNumber,
      date: new Date().toLocaleString('id-ID'),
      cashierName: mockReceipt.cashierName,
      customerName: mockReceipt.customerName,
      items: mockReceipt.items,
      subtotal: mockReceipt.subtotal,
      discount: mockReceipt.discount,
      tax: mockReceipt.tax,
      total: mockReceipt.total,
      paymentMethod: mockReceipt.paymentMethod,
      paidAmount: mockReceipt.paidAmount,
      change: mockReceipt.change,
    }

    const res = await printerStore.printReceipt(testReceiptData)
    if (res.success) {
      showNotification('success', `Uji cetak berhasil dikirim ke printer: ${printerStore.printerName}`)
    }
  } catch (err: any) {
    showNotification('error', `Uji cetak gagal: ${err.message || err}`)
  }
}

function showNotification(type: 'success' | 'error', message: string) {
  notification.value = { type, message }
  setTimeout(() => {
    if (notification.value?.message === message) {
      notification.value = null
    }
  }, 4500)
}

function formatRupiah(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
  }).format(val)
}
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 overflow-hidden font-sans">
      <!-- 1. TOP ACTION BAR -->
      <div class="bg-white border-b border-slate-300 px-6 py-3 flex items-center justify-between shadow-xs shrink-0">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-purple-100 border border-purple-200 flex items-center justify-center text-purple-700 shadow-xs">
            <Icon icon="streamline-ultimate-color:print-text" class="w-6 h-6" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h1 class="text-base font-extrabold text-slate-900 tracking-tight">Pengaturan & Desain Struk Belanja</h1>
              <span class="text-[10px] bg-purple-50 text-purple-700 border border-purple-200 font-bold px-2 py-0.5 rounded-full flex items-center gap-1 font-mono">
                <Icon icon="fluent-color:database-24" class="w-3 h-3" />
                Database PostgreSQL
              </span>
            </div>
            <p class="text-xs text-slate-500 mt-0.5">
              Kelola profil toko, kontak, catatan header/footer, dan tata letak cetak thermal printer langsung dari Back-Office.
            </p>
          </div>
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="resetToDefault"
            class="px-3.5 py-2 text-xs font-semibold text-slate-700 bg-slate-50 hover:bg-slate-100 border border-slate-300 rounded-lg transition flex items-center gap-1.5 cursor-pointer shadow-xs active:scale-95"
          >
            <RotateCcw class="w-3.5 h-3.5 text-slate-500" />
            <span>Reset Bawaan</span>
          </button>

          <button
            type="button"
            @click="handleTestPrint"
            class="px-3.5 py-2 text-xs font-semibold text-indigo-700 bg-indigo-50 hover:bg-indigo-100 border border-indigo-200 rounded-lg transition flex items-center gap-1.5 cursor-pointer shadow-xs active:scale-95"
            title="Cetak struk pengujian ke printer thermal kasir saat ini"
          >
            <Printer class="w-3.5 h-3.5 text-indigo-600" />
            <span>Uji Cetak Thermal</span>
          </button>

          <button
            type="button"
            @click="saveSettings"
            :disabled="isSaving || isLoading"
            class="px-5 py-2 text-xs font-bold text-white bg-emerald-600 hover:bg-emerald-700 disabled:bg-slate-400 rounded-lg shadow-sm border border-emerald-700 transition flex items-center gap-2 cursor-pointer active:scale-95"
          >
            <Save class="w-4 h-4" />
            <span>{{ isSaving ? 'Menyimpan ke DB...' : 'Simpan Pengaturan' }}</span>
          </button>
        </div>
      </div>

      <!-- Toast Notification -->
      <transition enter-active-class="transition duration-200 ease-out" enter-from-class="transform -translate-y-2 opacity-0" enter-to-class="transform translate-y-0 opacity-100" leave-active-class="transition duration-150 ease-in" leave-from-class="transform translate-y-0 opacity-100" leave-to-class="transform -translate-y-2 opacity-0">
        <div
          v-if="notification"
          class="mx-6 mt-3 px-4 py-2.5 rounded-lg border text-xs font-medium flex items-center justify-between shadow-sm z-20"
          :class="notification.type === 'success' ? 'bg-emerald-50 text-emerald-800 border-emerald-200' : 'bg-rose-50 text-rose-800 border-rose-200'"
        >
          <div class="flex items-center gap-2">
            <CheckCircle2 v-if="notification.type === 'success'" class="w-4 h-4 text-emerald-600" />
            <AlertCircle v-else class="w-4 h-4 text-rose-600" />
            <span>{{ notification.message }}</span>
          </div>
          <button @click="notification = null" class="text-slate-400 hover:text-slate-600 text-sm font-bold ml-4">✕</button>
        </div>
      </transition>

      <!-- 2. MAIN 2-COLUMN DESKTOP WORKSPACE -->
      <div class="flex-1 overflow-hidden p-6 grid grid-cols-12 gap-6">
        <!-- LEFT COLUMN: SETTINGS FORM (7 Columns) -->
        <div class="col-span-12 lg:col-span-7 flex flex-col bg-white rounded-xl border border-slate-300 shadow-xs overflow-hidden">
          <!-- Form Tabs Navigation -->
          <div class="bg-slate-50 border-b border-slate-200 px-4 pt-3 flex gap-2">
            <button
              @click="activeTab = 'identity'"
              class="px-4 py-2 rounded-t-lg text-xs font-bold transition flex items-center gap-2 border-t border-x cursor-pointer"
              :class="activeTab === 'identity'
                ? 'bg-white text-purple-700 border-slate-300 border-b-white -mb-px shadow-2xs'
                : 'text-slate-500 hover:text-slate-800 border-transparent hover:bg-slate-100'"
            >
              <Store class="w-3.5 h-3.5" />
              <span>1. Profil & Identitas Toko</span>
            </button>

            <button
              @click="activeTab = 'notes'"
              class="px-4 py-2 rounded-t-lg text-xs font-bold transition flex items-center gap-2 border-t border-x cursor-pointer"
              :class="activeTab === 'notes'
                ? 'bg-white text-purple-700 border-slate-300 border-b-white -mb-px shadow-2xs'
                : 'text-slate-500 hover:text-slate-800 border-transparent hover:bg-slate-100'"
            >
              <FileText class="w-3.5 h-3.5" />
              <span>2. Pesan Header & Footer</span>
            </button>

            <button
              @click="activeTab = 'display'"
              class="px-4 py-2 rounded-t-lg text-xs font-bold transition flex items-center gap-2 border-t border-x cursor-pointer"
              :class="activeTab === 'display'
                ? 'bg-white text-purple-700 border-slate-300 border-b-white -mb-px shadow-2xs'
                : 'text-slate-500 hover:text-slate-800 border-transparent hover:bg-slate-100'"
            >
              <Sliders class="w-3.5 h-3.5" />
              <span>3. Format Cetak & Kertas</span>
            </button>
          </div>

          <!-- Tab Content Scrollable Area -->
          <div class="flex-1 overflow-y-auto p-6 space-y-5">
            <!-- TAB 1: IDENTITAS TOKO -->
            <div v-show="activeTab === 'identity'" class="space-y-4">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Nama Toko / Unit Usaha <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="form.store_name"
                  type="text"
                  placeholder="Misal: NOTANE RETAIL ENTERPRISE"
                  class="w-full px-3.5 py-2 text-sm bg-slate-50 border border-slate-300 rounded-lg font-bold text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                />
                <p class="text-[11px] text-slate-400 mt-1">Dicetak paling atas struk dengan format teks tebal (Bold & Double Height).</p>
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Slogan / Tagline Usaha
                </label>
                <input
                  v-model="form.store_tagline"
                  type="text"
                  placeholder="Misal: Pusat Grosir & Eceran Termurah"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                />
                <p class="text-[11px] text-slate-400 mt-1">Muncul tepat di bawah nama toko untuk memperkuat citra brand.</p>
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Alamat Lengkap Toko / Outlet <span class="text-rose-500">*</span>
                </label>
                <textarea
                  v-model="form.store_address"
                  rows="2"
                  placeholder="Jl. Raya Utama No. 88, Kota / Kabupaten..."
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition resize-none"
                ></textarea>
                <p class="text-[11px] text-slate-400 mt-1">Akan dibungkus secara otomatis (word-wrap) sesuai lebar kertas thermal.</p>
              </div>

              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">
                    No. Telepon / WhatsApp Kasir
                  </label>
                  <input
                    v-model="form.store_phone"
                    type="text"
                    placeholder="0812-xxxx-xxxx"
                    class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                  />
                </div>

                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">
                    Website / Toko Online
                  </label>
                  <input
                    v-model="form.store_website"
                    type="text"
                    placeholder="www.tokosaya.com"
                    class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                  />
                </div>
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Nomor Pokok Wajib Pajak (NPWP / Tax ID)
                </label>
                <input
                  v-model="form.tax_id"
                  type="text"
                  placeholder="Misal: 01.234.567.8-901.000 (Kosongkan jika non-PKP)"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition font-mono"
                />
                <p class="text-[11px] text-slate-400 mt-1">Dicetak jika toko Anda terdaftar sebagai Pengusaha Kena Pajak (PKP).</p>
              </div>
            </div>

            <!-- TAB 2: CATATAN HEADER & FOOTER -->
            <div v-show="activeTab === 'notes'" class="space-y-4">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Catatan Pembuka (Header Note)
                </label>
                <input
                  v-model="form.header_note"
                  type="text"
                  placeholder="Misal: Selamat Datang & Selamat Berbelanja!"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                />
                <p class="text-[11px] text-slate-400 mt-1">Dicetak di bawah alamat toko sebelum garis pemisah transaksi.</p>
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Catatan Kaki Baris 1 (Ketentuan Transaksi)
                </label>
                <input
                  v-model="form.footer_note_1"
                  type="text"
                  placeholder="Misal: Barang yang sudah dibeli tidak dapat ditukar/dikembalikan"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                />
                <p class="text-[11px] text-slate-400 mt-1">Ketentuan retur barang, garansi, atau syarat struk.</p>
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Catatan Kaki Baris 2 (Ucapan & Layanan Pelanggan)
                </label>
                <input
                  v-model="form.footer_note_2"
                  type="text"
                  placeholder="Misal: Kritik & Saran Hubungi WA: 0812-xxxx-xxxx"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                />
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Media Sosial Toko
                </label>
                <input
                  v-model="form.social_media"
                  type="text"
                  placeholder="Misal: Follow IG & TikTok: @notane.retail"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-transparent transition"
                />
                <p class="text-[11px] text-slate-400 mt-1">Mendorong pelanggan untuk mengikuti akun promosi media sosial Anda.</p>
              </div>
            </div>

            <!-- TAB 3: OPSI TAMPILAN & KERTAS -->
            <div v-show="activeTab === 'display'" class="space-y-5">
              <!-- Paper Width Selection -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-2">
                  Standar Lebar Kertas Thermal Toko
                </label>
                <div class="grid grid-cols-2 gap-3">
                  <label
                    class="border-2 rounded-xl p-3 flex items-center gap-3 cursor-pointer transition"
                    :class="form.paper_width === '58mm'
                      ? 'border-purple-600 bg-purple-50/50'
                      : 'border-slate-200 hover:border-slate-300 bg-white'"
                  >
                    <input
                      type="radio"
                      v-model="form.paper_width"
                      value="58mm"
                      class="text-purple-600 focus:ring-purple-500 h-4 w-4"
                    />
                    <div>
                      <div class="text-xs font-bold text-slate-800">58mm (Kompak / Standar)</div>
                      <div class="text-[11px] text-slate-500 font-mono">Lebar 32 Kolom Karakter</div>
                    </div>
                  </label>

                  <label
                    class="border-2 rounded-xl p-3 flex items-center gap-3 cursor-pointer transition"
                    :class="form.paper_width === '80mm'
                      ? 'border-purple-600 bg-purple-50/50'
                      : 'border-slate-200 hover:border-slate-300 bg-white'"
                  >
                    <input
                      type="radio"
                      v-model="form.paper_width"
                      value="80mm"
                      class="text-purple-600 focus:ring-purple-500 h-4 w-4"
                    />
                    <div>
                      <div class="text-xs font-bold text-slate-800">80mm (Lebar / Supermarket)</div>
                      <div class="text-[11px] text-slate-500 font-mono">Lebar 48 Kolom Karakter</div>
                    </div>
                  </label>
                </div>
              </div>

              <!-- Display Toggles -->
              <div class="border-t border-slate-200 pt-4 space-y-3">
                <div class="text-xs font-bold text-slate-700 mb-2">Elemen yang Ditampilkan pada Struk:</div>

                <div class="flex items-center justify-between p-2.5 bg-slate-50 rounded-lg border border-slate-200">
                  <div>
                    <div class="text-xs font-bold text-slate-800">Tampilkan Alamat Toko</div>
                    <div class="text-[11px] text-slate-400">Sertakan alamat outlet lengkap di bagian atas struk</div>
                  </div>
                  <input
                    type="checkbox"
                    v-model="form.show_address"
                    class="w-4 h-4 text-purple-600 rounded focus:ring-purple-500 cursor-pointer"
                  />
                </div>

                <div class="flex items-center justify-between p-2.5 bg-slate-50 rounded-lg border border-slate-200">
                  <div>
                    <div class="text-xs font-bold text-slate-800">Tampilkan Nomor Telepon Toko</div>
                    <div class="text-[11px] text-slate-400">Sertakan kontak telepon / WhatsApp di header struk</div>
                  </div>
                  <input
                    type="checkbox"
                    v-model="form.show_phone"
                    class="w-4 h-4 text-purple-600 rounded focus:ring-purple-500 cursor-pointer"
                  />
                </div>

                <div class="flex items-center justify-between p-2.5 bg-slate-50 rounded-lg border border-slate-200">
                  <div>
                    <div class="text-xs font-bold text-slate-800">Tampilkan Nama Operator Kasir</div>
                    <div class="text-[11px] text-slate-400">Cetak nama kasir yang bertugas pada transaksi</div>
                  </div>
                  <input
                    type="checkbox"
                    v-model="form.show_cashier_name"
                    class="w-4 h-4 text-purple-600 rounded focus:ring-purple-500 cursor-pointer"
                  />
                </div>

                <div class="flex items-center justify-between p-2.5 bg-slate-50 rounded-lg border border-slate-200">
                  <div>
                    <div class="text-xs font-bold text-slate-800">Tampilkan Barcode Nomor Transaksi</div>
                    <div class="text-[11px] text-slate-400">Cetak visual barcode / kode invoice di bagian bawah</div>
                  </div>
                  <input
                    type="checkbox"
                    v-model="form.show_barcode"
                    class="w-4 h-4 text-purple-600 rounded focus:ring-purple-500 cursor-pointer"
                  />
                </div>

                <div class="flex items-center justify-between p-2.5 bg-slate-50 rounded-lg border border-slate-200">
                  <div>
                    <div class="text-xs font-bold text-slate-800">Tampilkan Rincian Pajak & Kembalian</div>
                    <div class="text-[11px] text-slate-400">Rincian subtotal, diskon, PPN, bayar tunai & kembalian</div>
                  </div>
                  <input
                    type="checkbox"
                    v-model="form.show_tax_summary"
                    class="w-4 h-4 text-purple-600 rounded focus:ring-purple-500 cursor-pointer"
                  />
                </div>
              </div>
            </div>
          </div>

          <!-- Form Footer Info -->
          <div class="bg-slate-50 border-t border-slate-200 px-6 py-2.5 flex items-center justify-between text-[11px] text-slate-500">
            <div class="flex items-center gap-1.5 font-mono">
              <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
              <span>Status: Terhubung ke Database PostgreSQL</span>
            </div>
            <div v-if="form.id" class="text-slate-400 font-mono text-[10px]">
              ID: {{ form.id }}
            </div>
          </div>
        </div>

        <!-- RIGHT COLUMN: LIVE THERMAL RECEIPT PREVIEW (5 Columns) -->
        <div class="col-span-12 lg:col-span-5 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <Eye class="w-4 h-4 text-purple-600" />
              <h2 class="text-xs font-bold text-slate-800 uppercase tracking-wider">Pratinjau Kertas Struk Real-Time</h2>
            </div>
            <span class="text-[10px] bg-purple-100 text-purple-800 px-2 py-0.5 rounded font-mono font-bold">
              {{ form.paper_width === '58mm' ? '58mm (32 Chars)' : '80mm (48 Chars)' }}
            </span>
          </div>

          <!-- Realistic Thermal Receipt Container -->
          <div class="flex-1 bg-slate-200/80 rounded-xl p-4 overflow-y-auto flex justify-center border border-slate-300 shadow-inner">
            <!-- The Paper Slip -->
            <div
              class="bg-white text-slate-900 shadow-lg font-mono text-[11px] leading-tight p-4 select-none relative transition-all duration-300"
              :class="form.paper_width === '58mm' ? 'w-[280px]' : 'w-[360px]'"
              style="filter: drop-shadow(0 4px 6px rgba(0,0,0,0.12));"
            >
              <!-- Top Jagged Paper Edge -->
              <div class="absolute -top-1.5 left-0 right-0 h-2 bg-repeat-x" style="background: radial-gradient(circle, transparent, transparent 50%, #ffffff 50%, #ffffff 100%); background-size: 8px 8px;"></div>

              <!-- STORE HEADER -->
              <div class="text-center pb-2 border-b border-dashed border-slate-400">
                <div class="font-extrabold text-sm tracking-wide uppercase">{{ form.store_name || 'NOTANE RETAIL' }}</div>
                <div v-if="form.store_tagline" class="text-[10px] text-slate-600 italic mt-0.5">{{ form.store_tagline }}</div>
                <div v-if="form.show_address && form.store_address" class="text-[10px] text-slate-700 mt-1 leading-snug">{{ form.store_address }}</div>
                <div v-if="form.show_phone && form.store_phone" class="text-[10px] text-slate-700 mt-0.5">Telp: {{ form.store_phone }}</div>
                <div v-if="form.store_website" class="text-[10px] text-slate-600">{{ form.store_website }}</div>
                <div v-if="form.tax_id" class="text-[9px] text-slate-500 font-mono mt-0.5">NPWP: {{ form.tax_id }}</div>
                <div v-if="form.header_note" class="text-[10px] font-semibold text-slate-800 mt-1 border-t border-dotted border-slate-300 pt-1">
                  {{ form.header_note }}
                </div>
              </div>

              <!-- TRANSACTION METADATA -->
              <div class="py-2 text-[10px] space-y-0.5 border-b border-dashed border-slate-400">
                <div class="flex justify-between">
                  <span>No: {{ mockReceipt.invoiceNumber }}</span>
                </div>
                <div class="flex justify-between">
                  <span>Tgl: {{ mockReceipt.date }}</span>
                </div>
                <div v-if="form.show_cashier_name" class="flex justify-between">
                  <span>Kasir: {{ mockReceipt.cashierName }}</span>
                </div>
                <div class="flex justify-between">
                  <span>Plg: {{ mockReceipt.customerName }}</span>
                </div>
              </div>

              <!-- ITEMS LIST -->
              <div class="py-2 border-b border-dashed border-slate-400 space-y-1.5">
                <div v-for="(item, idx) in mockReceipt.items" :key="idx" class="text-[10px]">
                  <div class="font-semibold text-slate-900 truncate">{{ item.name }}</div>
                  <div class="flex justify-between text-slate-700">
                    <span>{{ item.qty }} x {{ formatRupiah(item.price) }}</span>
                    <span class="font-bold">{{ formatRupiah(item.total) }}</span>
                  </div>
                </div>
              </div>

              <!-- TOTALS & PAYMENT -->
              <div v-if="form.show_tax_summary" class="py-2 border-b border-dashed border-slate-400 text-[10px] space-y-0.5">
                <div class="flex justify-between">
                  <span>Subtotal:</span>
                  <span>{{ formatRupiah(mockReceipt.subtotal) }}</span>
                </div>
                <div class="flex justify-between text-emerald-700">
                  <span>Diskon Promo:</span>
                  <span>-{{ formatRupiah(mockReceipt.discount) }}</span>
                </div>
                <div class="flex justify-between text-slate-600">
                  <span>PPN (11%):</span>
                  <span>{{ formatRupiah(mockReceipt.tax) }}</span>
                </div>
                <div class="flex justify-between font-bold text-xs pt-1 border-t border-dotted border-slate-300">
                  <span>TOTAL:</span>
                  <span>{{ formatRupiah(mockReceipt.total) }}</span>
                </div>
                <div class="flex justify-between pt-1">
                  <span>BAYAR ({{ mockReceipt.paymentMethod }}):</span>
                  <span>{{ formatRupiah(mockReceipt.paidAmount) }}</span>
                </div>
                <div class="flex justify-between font-semibold">
                  <span>KEMBALIAN:</span>
                  <span>{{ formatRupiah(mockReceipt.change) }}</span>
                </div>
              </div>

              <!-- FOOTER & NOTES -->
              <div class="text-center pt-2.5 pb-1 space-y-1 text-[10px]">
                <div v-if="form.footer_note_1" class="text-slate-700 leading-snug">
                  {{ form.footer_note_1 }}
                </div>
                <div v-if="form.footer_note_2" class="text-slate-700 leading-snug font-medium">
                  {{ form.footer_note_2 }}
                </div>
                <div v-if="form.social_media" class="text-[9px] text-slate-600 italic">
                  {{ form.social_media }}
                </div>

                <!-- Barcode Simulation -->
                <div v-if="form.show_barcode" class="pt-2 flex flex-col items-center">
                  <div class="tracking-widest font-black text-sm text-slate-800 scale-y-125 select-none font-mono">
                    ||| | | |||| | ||| || |||
                  </div>
                  <div class="text-[8px] tracking-wider text-slate-500 font-mono mt-0.5">
                    {{ mockReceipt.invoiceNumber }}
                  </div>
                </div>

                <div class="text-[8px] text-slate-400 font-mono pt-1">
                  *** NOTANE POS CLOUD & LAN ***
                </div>
              </div>

              <!-- Bottom Jagged Paper Cut Edge -->
              <div class="absolute -bottom-1.5 left-0 right-0 h-2 bg-repeat-x" style="background: radial-gradient(circle, transparent, transparent 50%, #ffffff 50%, #ffffff 100%); background-size: 8px 8px; transform: rotate(180deg);"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
