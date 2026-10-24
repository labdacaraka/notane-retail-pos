import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/services/api'
import { useCartStore } from './cartStore'

export interface AccessibleOutlet {
  id: string
  code: string
  name: string
  is_warehouse: boolean
}

export const useLocationStore = defineStore('location', () => {
  const accessibleOutlets = ref<AccessibleOutlet[]>([])
  const currentOutletId = ref<string>(localStorage.getItem('notane_active_outlet_id') || '')
  const isLoading = ref<boolean>(false)

  const currentOutlet = computed<AccessibleOutlet | null>(() => {
    if (!accessibleOutlets.value.length) return null
    return (
      accessibleOutlets.value.find((o) => o.id === currentOutletId.value) ||
      accessibleOutlets.value[0] ||
      null
    )
  })

  // Outlet yang bertipe Toko / Cabang Ritel (biasanya tempat kasir beroperasi)
  const storeOutlets = computed(() => {
    return accessibleOutlets.value.filter((o) => !o.is_warehouse)
  })

  // Outlet yang bertipe Gudang Fisik
  const warehouseOutlets = computed(() => {
    return accessibleOutlets.value.filter((o) => o.is_warehouse)
  })

  async function fetchAccessibleOutlets(): Promise<AccessibleOutlet[]> {
    isLoading.value = true
    try {
      const res = await api.get('/me/outlets')
      if (res.data?.success && Array.isArray(res.data.data)) {
        accessibleOutlets.value = res.data.data

        // Prioritaskan current_outlet_id yang disimpan di database user
        const serverCurrentOutletId = res.data.current_outlet_id
        if (serverCurrentOutletId && accessibleOutlets.value.some((o) => o.id === serverCurrentOutletId)) {
          currentOutletId.value = serverCurrentOutletId
          localStorage.setItem('notane_active_outlet_id', serverCurrentOutletId)
        } else {
          // Sinkronisasi outlet aktif jika belum ada atau tidak valid lagi
          const exists = accessibleOutlets.value.some((o) => o.id === currentOutletId.value)
          if (!exists && accessibleOutlets.value.length > 0) {
            // Pilih toko ritel pertama jika ada, jika tidak ada baru gudang
            const preferStore = accessibleOutlets.value.find((o) => !o.is_warehouse) || accessibleOutlets.value[0]
            await switchLocation(preferStore.id)
          }
        }
      }
      return accessibleOutlets.value
    } catch (err) {
      console.error('Gagal memuat outlet yang dapat diakses:', err)
      return []
    } finally {
      isLoading.value = false
    }
  }

  async function switchLocation(outletId: string) {
    const target = accessibleOutlets.value.find((o) => o.id === outletId)
    if (!target) return

    currentOutletId.value = target.id
    localStorage.setItem('notane_active_outlet_id', target.id)

    // Simpan personalisasi lokasi outlet aktif ke database per user (backend Zero-Trust)
    try {
      await api.post('/me/current-outlet', { outlet_id: target.id })
    } catch (err) {
      console.warn('Gagal menyimpan personalisasi outlet ke database:', err)
    }

    // Sinkronkan ke cartStore HANYA jika pengguna sedang berada di halaman kasir/POS
    if (typeof window !== 'undefined' && window.location.pathname.startsWith('/pos')) {
      const cartStore = useCartStore()
      await cartStore.setOutlet(target.id)
    }

    // Dispatch global event agar komponen yang mendengarkan dapat me-reload data
    window.dispatchEvent(new CustomEvent('notane-location-changed', { detail: target }))
  }

  return {
    accessibleOutlets,
    currentOutletId,
    currentOutlet,
    storeOutlets,
    warehouseOutlets,
    isLoading,
    fetchAccessibleOutlets,
    switchLocation,
  }
})
