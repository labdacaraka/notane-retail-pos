import { useDialogStore, type ConfirmDialogOptions, type AlertDialogOptions } from '@/stores/dialogStore'

export function useDialog() {
  const store = useDialogStore()

  const confirm = (opts: ConfirmDialogOptions | string): Promise<boolean> => {
    return store.showConfirm(opts)
  }

  const alert = (opts: AlertDialogOptions | string, type: 'danger' | 'warning' | 'info' | 'success' = 'info'): Promise<void> => {
    return store.showAlert(opts, type)
  }

  const toast = (message: string, type: 'success' | 'danger' | 'warning' | 'info' = 'info', duration: number = 3500) => {
    store.showToast(message, type, duration)
  }

  return {
    confirm,
    alert,
    toast,
  }
}
