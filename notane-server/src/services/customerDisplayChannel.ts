import { invoke, isTauri } from '@tauri-apps/api/core'

export interface CustomerDisplayItem {
  id: string
  name: string
  sku?: string
  barcode?: string
  quantity: number
  unit_code: string
  unit_price: number
  discount_amount: number
  subtotal: number
}

export interface CustomerDisplayState {
  status: 'IDLE' | 'ACTIVE' | 'PAYMENT_PENDING' | 'PAYMENT_SUCCESS'
  items: CustomerDisplayItem[]
  selectedCustomer: {
    id?: string
    name?: string
    phone?: string
    customer_group_name?: string
  } | null
  subtotal: number
  itemDiscounts: number
  voucherCode?: string
  voucherDiscount: number
  totalDiscount: number
  totalTax: number
  grandTotal: number
  totalItemsCount: number
  lastScannedItem?: CustomerDisplayItem | null
  paymentResult?: {
    tendered: number
    change: number
    paymentMethod: string
    invoiceNumber: string
  } | null
  outletName?: string
  updatedAt: number
}

const STORAGE_KEY = 'notane_customer_display_state'
const CHANNEL_NAME = 'notane_pos_customer_display_channel'

let channel: BroadcastChannel | null = null

function getChannel(): BroadcastChannel | null {
  if (typeof window === 'undefined') return null
  if (!channel && 'BroadcastChannel' in window) {
    try {
      channel = new BroadcastChannel(CHANNEL_NAME)
    } catch (e) {
      console.warn('BroadcastChannel not supported or error initializing:', e)
    }
  }
  return channel
}

/**
 * Membuka jendela Customer Display baik melalui Tauri IPC (desktop native)
 * ataupun window.open (browser mode).
 */
export async function openCustomerDisplayWindow() {
  if (isTauri()) {
    try {
      await invoke('open_customer_display_window')
      return
    } catch (err) {
      console.warn('Gagal memanggil Tauri IPC open_customer_display_window, fallback window.open:', err)
    }
  }
  window.open('/customer-display', '_blank', 'width=1024,height=600')
}

/**
 * Mengirim pembaruan status keranjang kasir ke Customer Display secara realtime.
 */
export function broadcastCustomerDisplayState(state: CustomerDisplayState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
  } catch {
    // Abaikan quota error
  }

  const ch = getChannel()
  if (ch) {
    ch.postMessage({ type: 'STATE_UPDATE', state })
  }
}

/**
 * Mengirim sinyal pembayaran berhasil dari kasir ke layar pelanggan.
 */
export function broadcastPaymentSuccess(payload: {
  tendered: number
  change: number
  paymentMethod: string
  invoiceNumber: string
  grandTotal: number
  customerName?: string
}) {
  const currentState = getCachedCustomerDisplayState()
  const successState: CustomerDisplayState = {
    ...currentState,
    status: 'PAYMENT_SUCCESS',
    grandTotal: payload.grandTotal,
    paymentResult: {
      tendered: payload.tendered,
      change: payload.change,
      paymentMethod: payload.paymentMethod,
      invoiceNumber: payload.invoiceNumber,
    },
    updatedAt: Date.now(),
  }

  broadcastCustomerDisplayState(successState)
}

/**
 * Membaca data state terakhir dari localStorage sebagai initial state saat layar pelanggan baru dimuat.
 */
export function getCachedCustomerDisplayState(): CustomerDisplayState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      return JSON.parse(raw)
    }
  } catch {
    // fallback
  }

  return {
    status: 'IDLE',
    items: [],
    selectedCustomer: null,
    subtotal: 0,
    itemDiscounts: 0,
    voucherCode: '',
    voucherDiscount: 0,
    totalDiscount: 0,
    totalTax: 0,
    grandTotal: 0,
    totalItemsCount: 0,
    lastScannedItem: null,
    paymentResult: null,
    updatedAt: Date.now(),
  }
}

/**
 * Berlangganan (subscribe) perubahan state dari kasir.
 */
export function subscribeCustomerDisplay(callback: (state: CustomerDisplayState) => void): () => void {
  const ch = getChannel()

  const handleMessage = (event: MessageEvent) => {
    if (event.data?.type === 'STATE_UPDATE' && event.data.state) {
      callback(event.data.state)
    }
  }

  const handleStorage = (event: StorageEvent) => {
    if (event.key === STORAGE_KEY && event.newValue) {
      try {
        const parsed = JSON.parse(event.newValue)
        callback(parsed)
      } catch {
        // Abaikan parse error
      }
    }
  }

  if (ch) {
    ch.addEventListener('message', handleMessage)
  }
  window.addEventListener('storage', handleStorage)

  // Minta state terkini dari kasir
  if (ch) {
    ch.postMessage({ type: 'REQUEST_SYNC' })
  }

  return () => {
    if (ch) {
      ch.removeEventListener('message', handleMessage)
    }
    window.removeEventListener('storage', handleStorage)
  }
}

/**
 * Kasir mendengarkan permintaan sinkronisasi dari Customer Display baru.
 */
export function listenForSyncRequests(getCurrentState: () => CustomerDisplayState): () => void {
  const ch = getChannel()
  if (!ch) return () => {}

  const handleMessage = (event: MessageEvent) => {
    if (event.data?.type === 'REQUEST_SYNC') {
      const state = getCurrentState()
      broadcastCustomerDisplayState(state)
    }
  }

  ch.addEventListener('message', handleMessage)
  return () => {
    ch.removeEventListener('message', handleMessage)
  }
}
