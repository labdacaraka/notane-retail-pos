import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface ConfirmDialogOptions {
  title?: string
  message: string
  subMessage?: string
  confirmText?: string
  cancelText?: string
  type?: 'danger' | 'warning' | 'info' | 'success'
}

export interface AlertDialogOptions {
  title?: string
  message: string
  type?: 'danger' | 'warning' | 'info' | 'success'
  buttonText?: string
}

export interface ToastItem {
  id: string
  message: string
  type: 'success' | 'danger' | 'warning' | 'info'
  timeoutId?: any
}

export const useDialogStore = defineStore('dialog', () => {
  // 1. Confirm Modal State
  const isConfirmOpen = ref<boolean>(false)
  const confirmOptions = ref<ConfirmDialogOptions>({
    title: 'Konfirmasi Tindakan',
    message: '',
    confirmText: 'Ya, Lanjutkan',
    cancelText: 'Batal',
    type: 'warning',
  })
  let confirmResolve: ((value: boolean) => void) | null = null

  function showConfirm(opts: ConfirmDialogOptions | string): Promise<boolean> {
    const options: ConfirmDialogOptions = typeof opts === 'string'
      ? { message: opts, type: 'warning' }
      : opts

    confirmOptions.value = {
      title: options.title || (options.type === 'danger' ? 'Konfirmasi Tindakan Berbahaya' : 'Konfirmasi Tindakan'),
      message: options.message,
      subMessage: options.subMessage,
      confirmText: options.confirmText || 'Ya, Lanjutkan',
      cancelText: options.cancelText || 'Batal',
      type: options.type || 'warning',
    }
    isConfirmOpen.value = true

    return new Promise<boolean>((resolve) => {
      confirmResolve = resolve
    })
  }

  function handleConfirmAction(result: boolean) {
    isConfirmOpen.value = false
    if (confirmResolve) {
      confirmResolve(result)
      confirmResolve = null
    }
  }

  // 2. Alert Modal State
  const isAlertOpen = ref<boolean>(false)
  const alertOptions = ref<AlertDialogOptions>({
    title: 'Pemberitahuan',
    message: '',
    type: 'info',
    buttonText: 'Mengerti',
  })
  let alertResolve: (() => void) | null = null

  function showAlert(opts: AlertDialogOptions | string, type: 'danger' | 'warning' | 'info' | 'success' = 'info'): Promise<void> {
    const options: AlertDialogOptions = typeof opts === 'string'
      ? { message: opts, type }
      : opts

    alertOptions.value = {
      title: options.title || (
        options.type === 'danger' ? 'Terjadi Kesalahan' :
        options.type === 'warning' ? 'Peringatan Sistem' :
        options.type === 'success' ? 'Berhasil' : 'Pemberitahuan'
      ),
      message: options.message,
      type: options.type || type || 'info',
      buttonText: options.buttonText || 'Mengerti',
    }
    isAlertOpen.value = true

    return new Promise<void>((resolve) => {
      alertResolve = resolve
    })
  }

  function handleAlertClose() {
    isAlertOpen.value = false
    if (alertResolve) {
      alertResolve()
      alertResolve = null
    }
  }

  // 3. Toasts
  const toasts = ref<ToastItem[]>([])

  function showToast(message: string, type: 'success' | 'danger' | 'warning' | 'info' = 'info', duration: number = 3500) {
    const id = `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`
    const timeoutId = setTimeout(() => {
      removeToast(id)
    }, duration)

    toasts.value.push({
      id,
      message,
      type,
      timeoutId,
    })
  }

  function removeToast(id: string) {
    const idx = toasts.value.findIndex(t => t.id === id)
    if (idx !== -1) {
      if (toasts.value[idx].timeoutId) clearTimeout(toasts.value[idx].timeoutId)
      toasts.value.splice(idx, 1)
    }
  }

  return {
    isConfirmOpen,
    confirmOptions,
    showConfirm,
    confirm: showConfirm,
    handleConfirmAction,
    isAlertOpen,
    alertOptions,
    showAlert,
    alert: showAlert,
    handleAlertClose,
    toasts,
    showToast,
    toast: showToast,
    removeToast,
  }
})
