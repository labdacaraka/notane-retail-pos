import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/services/api'
import type { CashierShift } from '@/types'
import { useCartStore } from './cartStore'

export const useShiftStore = defineStore('shift', () => {
  const currentShift = ref<CashierShift | null>(null)
  const isShiftOpen = ref<boolean>(false)
  const isLoading = ref<boolean>(false)
  const showShiftModal = ref<boolean>(false)
  const shiftModalMode = ref<'OPEN' | 'CLOSE'>('OPEN')

  function openShiftModal(mode: 'OPEN' | 'CLOSE' = 'OPEN') {
    shiftModalMode.value = mode
    showShiftModal.value = true
  }

  function closeShiftModal() {
    showShiftModal.value = false
  }

  async function checkCurrentShift() {
    isLoading.value = true
    try {
      const res = await api.get('/shifts/current')
      if (res.data.success && res.data.data && res.data.data.status === 'OPEN') {
        currentShift.value = res.data.data
        isShiftOpen.value = true
        if (res.data.data.outlet_id) {
          const cartStore = useCartStore()
          cartStore.setOutlet(res.data.data.outlet_id)
        }
      } else {
        currentShift.value = null
        isShiftOpen.value = false
        const cartStore = useCartStore()
        cartStore.setOutlet('')
        cartStore.clearCart()
      }
    } catch {
      currentShift.value = null
      isShiftOpen.value = false
      const cartStore = useCartStore()
      cartStore.setOutlet('')
      cartStore.clearCart()
    } finally {
      isLoading.value = false
    }
  }

  async function openShift(
    terminalId: string,
    openingCash: number,
    supervisorId?: string,
    supervisorPin?: string,
    outletId?: string,
    supervisorAuthToken?: string
  ) {
    const res = await api.post('/shifts/open', {
      terminal_id: terminalId,
      opening_cash: openingCash,
      supervisor_id: supervisorId || undefined,
      supervisor_pin: supervisorPin || undefined,
      supervisor_auth_token: supervisorAuthToken || undefined,
      outlet_id: outletId || undefined,
    })
    if (res.data.success) {
      currentShift.value = res.data.data
      isShiftOpen.value = true
      const cartStore = useCartStore()
      cartStore.clearCart()
      if (res.data.data.outlet_id) {
        cartStore.setOutlet(res.data.data.outlet_id)
      }
      return res.data.data
    }
    throw new Error(res.data.message || 'Gagal membuka shift')
  }

  async function closeShift(
    actualCash: number,
    supervisorId?: string,
    supervisorPin?: string,
    supervisorAuthToken?: string
  ) {
    if (!currentShift.value) throw new Error('Tidak ada shift aktif')

    const res = await api.post('/shifts/close', {
      shift_id: currentShift.value.id,
      actual_cash: actualCash,
      supervisor_id: supervisorId || undefined,
      supervisor_pin: supervisorPin || undefined,
      supervisor_auth_token: supervisorAuthToken || undefined,
    })

    if (res.data.success) {
      currentShift.value = null
      isShiftOpen.value = false
      const cartStore = useCartStore()
      cartStore.clearCart()
      cartStore.setOutlet('')
      return res.data.data
    }
    throw new Error(res.data.message || 'Gagal menutup shift')
  }

  return {
    currentShift,
    isShiftOpen,
    isLoading,
    showShiftModal,
    shiftModalMode,
    openShiftModal,
    closeShiftModal,
    checkCurrentShift,
    openShift,
    closeShift,
  }
})
