/**
 * Utility Formatter Terpusat untuk Notane Manager
 * Mengatasi masalah format angka desimal PostgreSQL NUMERIC (seperti 136.0000 -> 136, 22.5000 -> 22,5)
 * serta standardisasi format mata uang Rupiah dan tanggal Indonesia.
 */

/**
 * Parse string atau number berdesimal dari database/input menjadi number murni yang bersih.
 * Menghilangkan trailing zero PostgreSQL NUMERIC:
 * parseDecimal("136.0000") -> 136
 * parseDecimal("22.5000")  -> 22.5
 */
export function parseDecimal(val: number | string | null | undefined): number {
  if (val === null || val === undefined || val === '') return 0
  if (typeof val === 'number') return isNaN(val) ? 0 : val
  const parsed = parseFloat(String(val).replace(/,/g, '.'))
  return isNaN(parsed) ? 0 : parsed
}

/**
 * Format angka desimal rapi untuk kuantitas, berat, atau nilai desimal.
 * Menghilangkan deretan nol trailing yang tidak perlu atau membatasi presisi.
 */
export function formatDecimal(
  val: number | string | null | undefined,
  maxDecimals: number = 2,
  forceDecimal: boolean = false
): string {
  if (val === null || val === undefined || val === '') {
    return forceDecimal ? '0,0' : '0'
  }

  const num = typeof val === 'number' ? val : parseFloat(String(val).replace(/,/g, '.'))
  if (isNaN(num)) return '0'

  if (forceDecimal) {
    return new Intl.NumberFormat('id-ID', {
      minimumFractionDigits: 1,
      maximumFractionDigits: maxDecimals,
    }).format(num)
  }

  return new Intl.NumberFormat('id-ID', {
    minimumFractionDigits: 0,
    maximumFractionDigits: maxDecimals,
  }).format(num)
}

/**
 * Format angka umum tanpa trailing nol.
 */
export function formatNumber(
  val: number | string | null | undefined,
  maxDecimals: number = 2
): string {
  return formatDecimal(val, maxDecimals, false)
}

/**
 * Format kuantitas barang / stok / opname / unit.
 */
export function formatQty(val: number | string | null | undefined, maxDecimals: number = 3): string {
  return formatDecimal(val, maxDecimals, false)
}

/**
 * Format mata uang Rupiah (IDR).
 * Menghilangkan trailing zero pada desimal PostgreSQL NUMERIC.
 */
export function formatCurrency(
  val: number | string | null | undefined,
  withSymbol: boolean = false,
  maxDecimals: number = 2
): string {
  if (val === null || val === undefined || val === '') {
    return withSymbol ? 'Rp 0' : '0'
  }

  const num = typeof val === 'number' ? val : parseFloat(String(val).replace(/,/g, '.'))
  if (isNaN(num)) return withSymbol ? 'Rp 0' : '0'

  const formatted = new Intl.NumberFormat('id-ID', {
    minimumFractionDigits: 0,
    maximumFractionDigits: maxDecimals,
  }).format(num)

  return withSymbol ? `Rp ${formatted}` : formatted
}

/**
 * Format persentase tanpa trailing nol berlebih.
 */
export function formatPercent(val: number | string | null | undefined, maxDecimals: number = 2): string {
  if (val === null || val === undefined || val === '') return '0%'
  const num = typeof val === 'number' ? val : parseFloat(String(val).replace(/,/g, '.'))
  if (isNaN(num)) return '0%'

  const formatted = new Intl.NumberFormat('id-ID', {
    minimumFractionDigits: 0,
    maximumFractionDigits: maxDecimals,
  }).format(num)

  return `${formatted}%`
}

/**
 * Format tanggal dan waktu standar Indonesia.
 */
export function formatDate(val: string | Date | null | undefined, includeTime: boolean = false): string {
  if (!val) return '-'
  const d = typeof val === 'string' ? new Date(val) : val
  if (isNaN(d.getTime())) return '-'

  const dateStr = d.toLocaleDateString('id-ID', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })

  if (!includeTime) return dateStr

  const timeStr = d.toLocaleTimeString('id-ID', {
    hour: '2-digit',
    minute: '2-digit',
  })

  return `${dateStr}, ${timeStr}`
}

