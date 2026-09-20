import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/services/api'
import type { CashierShift } from '@/types'

export const useShiftStore = defineStore('shift', () => {
  const currentShift = ref<CashierShift | null>(null)
  const isShiftOpen = ref<boolean>(false)
  const isLoading = ref<boolean>(false)

  async function checkCurrentShift() {
    isLoading.value = true
    try {
      const res = await api.get('/shifts/current')
      if (res.data.success && res.data.data) {
        currentShift.value = res.data.data
        isShiftOpen.value = res.data.data.status === 'OPEN'
      } else {
        currentShift.value = null
        isShiftOpen.value = false
      }
    } catch {
      currentShift.value = null
      isShiftOpen.value = false
    } finally {
      isLoading.value = false
    }
  }

  async function openShift(terminalId: string, openingCash: number) {
    const res = await api.post('/shifts/open', {
      terminal_id: terminalId,
      opening_cash: openingCash,
    })
    if (res.data.success) {
      currentShift.value = res.data.data
      isShiftOpen.value = true
      return res.data.data
    }
    throw new Error(res.data.message || 'Gagal membuka shift')
  }

  async function closeShift(actualCash: number, supervisorPin?: string) {
    if (!currentShift.value) throw new Error('Tidak ada shift aktif')

    const res = await api.post('/shifts/close', {
      shift_id: currentShift.value.id,
      actual_cash: actualCash,
      supervisor_pin: supervisorPin,
    })

    if (res.data.success) {
      currentShift.value = null
      isShiftOpen.value = false
      return res.data.data
    }
    throw new Error(res.data.message || 'Gagal menutup shift')
  }

  return {
    currentShift,
    isShiftOpen,
    isLoading,
    checkCurrentShift,
    openShift,
    closeShift,
  }
})
