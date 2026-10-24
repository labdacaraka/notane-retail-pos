/**
 * Offline Storage Service
 * Menyediakan penyimpanan lokal (Cache Katalog Produk, Kategori, dan Antrean Transaksi Offline)
 * untuk Terminal Kasir Client jika koneksi LAN ke Server Host sewaktu-waktu terputus.
 */

export interface OfflineTransaction {
  id: string
  created_at: string
  payload: any
  status: 'PENDING' | 'SYNCED' | 'FAILED'
  retry_count: number
  error_message?: string
}

const STORAGE_KEYS = {
  PRODUCTS: 'notane_offline_products',
  CATEGORIES: 'notane_offline_categories',
  QUEUE: 'notane_offline_transactions_queue',
  LAST_SYNC: 'notane_offline_last_sync',
}

export const offlineStorage = {
  // -------------------------------------------------------------
  // KATALOG PRODUK & KATEGORI OFFLINE
  // -------------------------------------------------------------
  saveProducts(products: any[]) {
    try {
      localStorage.setItem(STORAGE_KEYS.PRODUCTS, JSON.stringify(products))
      localStorage.setItem(STORAGE_KEYS.LAST_SYNC, new Date().toISOString())
    } catch (e) {
      console.warn('[OfflineStorage] Gagal menyimpan cache produk:', e)
    }
  },

  getCachedProducts(): any[] {
    try {
      const data = localStorage.getItem(STORAGE_KEYS.PRODUCTS)
      return data ? JSON.parse(data) : []
    } catch {
      return []
    }
  },

  getCachedProductsCount(): number {
    return this.getCachedProducts().length
  },

  saveCategories(categories: any[]) {
    try {
      localStorage.setItem(STORAGE_KEYS.CATEGORIES, JSON.stringify(categories))
    } catch (e) {
      console.warn('[OfflineStorage] Gagal menyimpan cache kategori:', e)
    }
  },

  getCachedCategories(): any[] {
    try {
      const data = localStorage.getItem(STORAGE_KEYS.CATEGORIES)
      return data ? JSON.parse(data) : []
    } catch {
      return []
    }
  },

  getLastSyncTime(): string | null {
    return localStorage.getItem(STORAGE_KEYS.LAST_SYNC)
  },

  // -------------------------------------------------------------
  // ANTREAN TRANSAKSI OFFLINE (OFFLINE QUEUE)
  // -------------------------------------------------------------
  queueTransaction(payload: any): OfflineTransaction {
    const queue = this.getQueuedTransactions()
    const tx: OfflineTransaction = {
      id: `OFFLINE-${Date.now()}-${Math.random().toString(36).substr(2, 6).toUpperCase()}`,
      created_at: new Date().toISOString(),
      payload,
      status: 'PENDING',
      retry_count: 0,
    }
    queue.push(tx)
    try {
      localStorage.setItem(STORAGE_KEYS.QUEUE, JSON.stringify(queue))
    } catch (e) {
      console.warn('[OfflineStorage] Gagal menyimpan antrean transaksi:', e)
    }
    return tx
  },

  getQueuedTransactions(): OfflineTransaction[] {
    try {
      const data = localStorage.getItem(STORAGE_KEYS.QUEUE)
      return data ? JSON.parse(data) : []
    } catch {
      return []
    }
  },

  getPendingQueueCount(): number {
    return this.getQueuedTransactions().filter(t => t.status === 'PENDING').length
  },

  clearSyncedTransactions() {
    const queue = this.getQueuedTransactions().filter(t => t.status === 'PENDING')
    localStorage.setItem(STORAGE_KEYS.QUEUE, JSON.stringify(queue))
  },

  clearAll() {
    localStorage.removeItem(STORAGE_KEYS.PRODUCTS)
    localStorage.removeItem(STORAGE_KEYS.CATEGORIES)
    localStorage.removeItem(STORAGE_KEYS.QUEUE)
    localStorage.removeItem(STORAGE_KEYS.LAST_SYNC)
  },
}
