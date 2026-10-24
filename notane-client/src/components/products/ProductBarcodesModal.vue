<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  Barcode,
  Plus,
  Trash2,
  Star,
  CheckCircle2,
  AlertCircle,
  HelpCircle,
  Tag,
  Sparkles,
  RefreshCw,
  X
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api, getApiBaseUrl } from '@/services/api'
import { useDialog } from '@/composables/useDialog'
import { useAuthStore } from '@/stores/authStore'

const dialog = useDialog()
const authStore = useAuthStore()

const props = defineProps<{
  show: boolean
  product: any
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

const isLoading = ref<boolean>(false)
const isSubmitting = ref<boolean>(false)
const barcodeData = ref<{
  product_id: string
  product_name: string
  product_sku: string
  primary_barcode: string
  additional_barcodes: Array<{
    id: string
    barcode: string
    description?: string | null
    created_at: string
  }>
} | null>(null)

// Form tambah barcode
const formBarcode = ref<string>('')
const formDescription = ref<string>('')
const formSetAsPrimary = ref<boolean>(false)
const errorMessage = ref<string>('')
const successMessage = ref<string>('')

function resolveImageUrl(url?: string | null): string | null {
  if (!url) return null
  if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('data:')) return url
  const base = getApiBaseUrl().replace('/api/v1', '')
  return `${base}${url}`
}

async function fetchBarcodes() {
  if (!props.product?.id) return
  isLoading.value = true
  errorMessage.value = ''
  try {
    const res = await api.get(`/products/${props.product.id}/barcodes`)
    if (res.data.success) {
      barcodeData.value = res.data.data
    }
  } catch (err: any) {
    console.error('Gagal mengambil daftar barcode:', err)
    errorMessage.value = err.response?.data?.message || 'Gagal memuat daftar barcode'
  } finally {
    isLoading.value = false
  }
}

watch(
  () => props.show,
  (val) => {
    if (val && props.product?.id) {
      formBarcode.value = ''
      formDescription.value = ''
      formSetAsPrimary.value = false
      errorMessage.value = ''
      successMessage.value = ''
      fetchBarcodes()
    }
  }
)

async function handleAddBarcode() {
  if (!authStore.hasPermission(['products:barcodes', 'master:write'])) {
    errorMessage.value = 'Akses ditolak: Anda tidak memiliki wewenang mengelola barcode produk (products:barcodes)'
    return
  }
  const code = formBarcode.value.trim()
  if (!code) {
    errorMessage.value = 'Silakan masukkan atau scan kode barcode'
    return
  }

  isSubmitting.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const res = await api.post(`/products/${props.product.id}/barcodes`, {
      barcode: code,
      description: formDescription.value.trim() || undefined,
      set_as_primary: formSetAsPrimary.value,
    })

    if (res.data.success) {
      successMessage.value = res.data.message
      formBarcode.value = ''
      formDescription.value = ''
      formSetAsPrimary.value = false
      await fetchBarcodes()
      emit('updated')
    }
  } catch (err: any) {
    errorMessage.value = err.response?.data?.message || 'Gagal menambahkan barcode'
  } finally {
    isSubmitting.value = false
  }
}

async function handleDeleteBarcode(barcodeId: string, barcodeStr: string) {
  if (!authStore.hasPermission(['products:barcodes', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengelola barcode produk (products:barcodes)', 'warning')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Hapus Barcode Alias',
    message: `Hapus barcode alias '${barcodeStr}' dari produk ini?`,
    subMessage: 'Barcode ini tidak akan dapat di-scan lagi di kasir POS untuk produk ini.',
    confirmText: 'Ya, Hapus',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return

  isSubmitting.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const res = await api.delete(`/products/${props.product.id}/barcodes/${barcodeId}`)
    if (res.data.success) {
      successMessage.value = `Barcode '${barcodeStr}' berhasil dihapus`
      dialog.toast(`Barcode '${barcodeStr}' berhasil dihapus`, 'success')
      await fetchBarcodes()
      emit('updated')
    }
  } catch (err: any) {
    errorMessage.value = err.response?.data?.message || 'Gagal menghapus barcode'
  } finally {
    isSubmitting.value = false
  }
}

async function handleSetPrimary(barcodeId: string, barcodeStr: string) {
  if (!authStore.hasPermission(['products:barcodes', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengelola barcode produk (products:barcodes)', 'warning')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Jadikan Barcode Utama',
    message: `Jadikan barcode '${barcodeStr}' sebagai Barcode Utama?`,
    subMessage: `Barcode utama saat ini (${barcodeData.value?.primary_barcode}) akan otomatis diarsipkan sebagai barcode alias.`,
    confirmText: 'Ya, Jadikan Utama',
    cancelText: 'Batal',
    type: 'warning',
  })
  if (!confirmed) return

  isSubmitting.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const res = await api.put(`/products/${props.product.id}/barcodes/${barcodeId}/set-primary`)
    if (res.data.success) {
      successMessage.value = res.data.message
      dialog.toast('Barcode utama berhasil diperbarui', 'success')
      await fetchBarcodes()
      emit('updated')
    }
  } catch (err: any) {
    errorMessage.value = err.response?.data?.message || 'Gagal mengubah barcode utama'
  } finally {
    isSubmitting.value = false
  }
}

function formatDate(isoStr: string): string {
  try {
    const d = new Date(isoStr)
    return d.toLocaleDateString('id-ID', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return isoStr
  }
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn"
    @click.self="emit('close')"
  >
    <div
      class="bg-white border border-slate-300 w-full max-w-2xl rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh] text-slate-900 animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- Header -->
      <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-emerald-500/20 border border-emerald-400/30 flex items-center justify-center shrink-0 overflow-hidden">
            <img
              v-if="product?.image_url"
              :src="resolveImageUrl(product?.image_url)!"
              :alt="product?.name"
              class="w-full h-full object-cover"
            />
            <Icon v-else icon="streamline-ultimate-color:barcode" class="w-6 h-6" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h3 class="text-base font-black tracking-tight text-white">Manajemen Multi-Barcode</h3>
              <span class="px-2 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider bg-emerald-500/20 text-emerald-300 border border-emerald-400/30">
                Ganti Kemasan / Alias
              </span>
            </div>
            <p class="text-xs text-slate-300 truncate max-w-md">
              {{ product?.name }} <span class="font-mono text-emerald-300 font-bold">({{ product?.sku }})</span>
            </p>
          </div>
        </div>

        <button
          type="button"
          @click="emit('close')"
          class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Body Container -->
      <div class="p-6 overflow-y-auto space-y-5 text-xs flex-1">
        <!-- Notifikasi Error / Sukses -->
        <div
          v-if="errorMessage"
          class="p-3 bg-rose-50 border border-rose-200 text-rose-800 rounded-xl flex items-center gap-2 animate-in fade-in"
        >
          <AlertCircle class="w-4 h-4 text-rose-600 shrink-0" />
          <span class="font-medium">{{ errorMessage }}</span>
        </div>

        <div
          v-if="successMessage"
          class="p-3 bg-emerald-50 border border-emerald-200 text-emerald-800 rounded-xl flex items-center gap-2 animate-in fade-in"
        >
          <CheckCircle2 class="w-4 h-4 text-emerald-600 shrink-0" />
          <span class="font-medium">{{ successMessage }}</span>
        </div>

        <!-- Edukasi / Penjelasan Sistem -->
        <div class="p-3 bg-indigo-50/70 border border-indigo-200 rounded-xl text-indigo-900 flex items-start gap-2.5">
          <Sparkles class="w-4 h-4 text-indigo-600 shrink-0 mt-0.5" />
          <div class="space-y-0.5">
            <span class="font-bold">Solusi Pergantian Barcode Pabrik & Multi-Kemasan:</span>
            <p class="text-[11px] text-indigo-700 leading-relaxed">
              Ketika produsen mengganti kemasan produk, Anda tidak perlu mengubah stok atau membuat produk baru. Cukup daftarkan barcode baru di sini. Saat di-scan di kasir atau dicatat di penerimaan barang, sistem otomatis mengenali barang yang sama!
            </p>
          </div>
        </div>

        <!-- 1. Barcode Utama Saat Ini -->
        <div class="bg-slate-50 border border-slate-200 p-4 rounded-xl space-y-2">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-1.5 font-bold text-slate-700 uppercase tracking-wider text-[11px]">
              <Star class="w-4 h-4 text-amber-500 fill-amber-500" />
              <span>Barcode Utama (Primary Barcode)</span>
            </div>
            <span class="text-[10px] font-bold bg-amber-100 text-amber-800 px-2 py-0.5 rounded-full border border-amber-200">
              Standar Master
            </span>
          </div>

          <div class="flex items-center justify-between bg-white border border-slate-300 p-3 rounded-lg shadow-2xs">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-lg bg-amber-50 border border-amber-200 flex items-center justify-center">
                <Barcode class="w-4 h-4 text-amber-600" />
              </div>
              <div>
                <div class="font-mono font-black text-sm text-slate-900 tracking-wider">
                  {{ barcodeData?.primary_barcode || product?.barcode || '(Belum diset)' }}
                </div>
                <div class="text-[10px] text-slate-400">Barcode default yang dicetak pada label dan nota</div>
              </div>
            </div>
            <span class="text-emerald-600 font-bold text-[11px] flex items-center gap-1">
              <CheckCircle2 class="w-3.5 h-3.5" />
              Aktif
            </span>
          </div>
        </div>

        <!-- 2. Form Tambah Barcode Baru / Ganti Barcode -->
        <div
          v-if="authStore.hasPermission(['products:barcodes', 'master:write'])"
          class="border border-slate-200 p-4 rounded-xl space-y-3 bg-white shadow-2xs"
        >
          <div class="font-bold text-slate-800 flex items-center gap-1.5 uppercase tracking-wider text-[11px]">
            <Plus class="w-4 h-4 text-emerald-600" />
            <span>Tambah Barcode Alternatif / Baru</span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <label class="block font-bold text-slate-700 mb-1">
                Kode Barcode * <span class="text-[10px] font-normal text-slate-400">(Scan / Ketik)</span>
              </label>
              <div class="relative">
                <input
                  v-model="formBarcode"
                  @keydown.enter.prevent="handleAddBarcode"
                  placeholder="Scan barcode dengan scanner..."
                  class="w-full pl-8 pr-3 py-2 rounded-lg border border-slate-300 font-mono text-xs font-bold text-slate-900 focus:bg-white focus:border-emerald-600 focus:ring-1 focus:ring-emerald-600"
                />
                <Icon icon="streamline-ultimate-color:barcode" class="w-4 h-4 absolute left-2.5 top-2.5 text-slate-400" />
              </div>
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">
                Keterangan / Alasan <span class="text-[10px] font-normal text-slate-400">(Opsional)</span>
              </label>
              <input
                v-model="formDescription"
                @keydown.enter.prevent="handleAddBarcode"
                placeholder="Contoh: Kemasan Baru 2026, Edisi Khusus, Barcode Dus"
                class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs text-slate-900 focus:bg-white focus:border-emerald-600 focus:ring-1 focus:ring-emerald-600"
              />
            </div>
          </div>

          <!-- Pilihan Jadikan Barcode Utama -->
          <div class="flex items-center justify-between pt-1">
            <label class="flex items-center gap-2 cursor-pointer text-slate-700 font-medium">
              <input
                v-model="formSetAsPrimary"
                type="checkbox"
                class="rounded border-slate-300 text-emerald-600 focus:ring-emerald-500 w-4 h-4 cursor-pointer"
              />
              <span>Jadikan langsung sebagai <strong>Barcode Utama</strong> (barcode lama otomatis jadi alias)</span>
            </label>

            <button
              type="button"
              @click="handleAddBarcode"
              :disabled="isSubmitting || !formBarcode.trim()"
              class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white font-bold rounded-lg transition flex items-center gap-1.5 shadow-2xs cursor-pointer active:scale-95"
            >
              <RefreshCw v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
              <Plus v-else class="w-3.5 h-3.5" />
              <span>Simpan Barcode</span>
            </button>
          </div>
        </div>

        <!-- 3. Daftar Barcode Tambahan / Alias -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <div class="font-bold text-slate-700 uppercase tracking-wider text-[11px] flex items-center gap-1.5">
              <Tag class="w-4 h-4 text-slate-500" />
              <span>Daftar Barcode Tambahan / Riwayat ({{ barcodeData?.additional_barcodes?.length || 0 }})</span>
            </div>
            <span class="text-[10px] text-slate-400 font-sans">
              Semua barcode di bawah aktif dan dapat di-scan di kasir
            </span>
          </div>

          <!-- Loading State -->
          <div v-if="isLoading" class="p-8 text-center text-slate-400 flex flex-col items-center gap-2">
            <RefreshCw class="w-5 h-5 animate-spin text-emerald-600" />
            <span>Memuat barcode...</span>
          </div>

          <!-- Empty State -->
          <div
            v-else-if="!barcodeData?.additional_barcodes || barcodeData.additional_barcodes.length === 0"
            class="p-6 text-center bg-slate-50 border border-dashed border-slate-200 rounded-xl space-y-1 text-slate-400"
          >
            <Icon icon="streamline-ultimate-color:barcode" class="w-8 h-8 mx-auto opacity-40 mb-1" />
            <div class="font-bold text-slate-600 text-xs">Belum ada barcode tambahan</div>
            <div class="text-[11px]">
              Produk ini baru memiliki 1 Barcode Utama. Jika kemasan berganti, tambahkan barcode baru pada formulir di atas.
            </div>
          </div>

          <!-- List Table -->
          <div v-else class="border border-slate-200 rounded-xl overflow-hidden shadow-2xs">
            <table class="w-full text-left text-xs">
              <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                <tr>
                  <th class="px-3 py-2">Kode Barcode</th>
                  <th class="px-3 py-2">Keterangan / Riwayat</th>
                  <th class="px-3 py-2 w-36">Tanggal Daftar</th>
                  <th class="px-3 py-2 text-right w-44">Tindakan</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100">
                <tr
                  v-for="b in barcodeData.additional_barcodes"
                  :key="b.id"
                  class="hover:bg-slate-50 transition"
                >
                  <td class="px-3 py-2.5 font-mono font-bold text-slate-900">
                    <div class="flex items-center gap-2">
                      <Barcode class="w-3.5 h-3.5 text-slate-400" />
                      <span>{{ b.barcode }}</span>
                    </div>
                  </td>
                  <td class="px-3 py-2.5 text-slate-600">
                    <span v-if="b.description" class="px-2 py-0.5 rounded bg-slate-100 text-slate-700 text-[11px]">
                      {{ b.description }}
                    </span>
                    <span v-else class="text-slate-400 italic text-[10px]">-</span>
                  </td>
                  <td class="px-3 py-2.5 text-slate-400 text-[11px]">
                    {{ formatDate(b.created_at) }}
                  </td>
                  <td class="px-3 py-2.5 text-right">
                    <div
                      v-if="authStore.hasPermission(['products:barcodes', 'master:write'])"
                      class="flex items-center justify-end gap-1.5"
                    >
                      <button
                        type="button"
                        @click="handleSetPrimary(b.id, b.barcode)"
                        :disabled="isSubmitting"
                        title="Jadikan barcode ini sebagai Barcode Utama"
                        class="px-2 py-1 rounded bg-amber-50 hover:bg-amber-100 text-amber-700 border border-amber-200 font-bold text-[10px] flex items-center gap-1 transition cursor-pointer disabled:opacity-50"
                      >
                        <Star class="w-3 h-3 text-amber-600" />
                        <span>Jadikan Utama</span>
                      </button>
                      <button
                        type="button"
                        @click="handleDeleteBarcode(b.id, b.barcode)"
                        :disabled="isSubmitting"
                        title="Hapus barcode ini"
                        class="p-1 rounded text-rose-500 hover:bg-rose-50 transition cursor-pointer disabled:opacity-50"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                    <span v-else class="text-[10px] text-slate-400 italic">Hanya Baca</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-3 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
        <div class="text-[11px] text-slate-500 flex items-center gap-1.5">
          <HelpCircle class="w-3.5 h-3.5 text-slate-400" />
          <span>Scan salah satu barcode di kasir akan langsung mendeteksi produk ini.</span>
        </div>
        <button
          type="button"
          @click="emit('close')"
          class="px-4 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer shadow-2xs"
        >
          Tutup
        </button>
      </div>
    </div>
  </div>
</template>
