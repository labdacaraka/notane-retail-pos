/**
 * Utility Formatter Terpusat untuk Notane Retail POS
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
 *
 * Contoh:
 * formatDecimal(25.00000)          -> "25"
 * formatDecimal("136.0000")        -> "136"
 * formatDecimal("22.5000")         -> "22,5"
 * formatDecimal(25.00000, 1, true) -> "25,0"  (jika forceDecimal aktif)
 * formatDecimal(1250.75)           -> "1.250,75"
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
 * Jika bulat tampilkan bulat (misal 25), jika ada desimal tampilkan sampai 3 digit (misal 25,25 atau 0,75).
 */
export function formatQty(val: number | string | null | undefined, maxDecimals: number = 3): string {
  return formatDecimal(val, maxDecimals, false)
}

/**
 * Format mata uang Rupiah (IDR).
 * Menghilangkan trailing zero pada desimal PostgreSQL NUMERIC.
 * Contoh:
 * formatCurrency(25000)          -> "25.000"
 * formatCurrency(25000, true)    -> "Rp 25.000"
 * formatCurrency("136.0000")     -> "136"
 * formatCurrency("136.0000", true) -> "Rp 136"
 * formatCurrency("22.5000")      -> "22,5"
 * formatCurrency("22.5000", true)-> "Rp 22,5"
 * formatCurrency(25000.5)        -> "25.000,5"
 * formatCurrency(25000.75)       -> "25.000,75"
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
 * Contoh:
 * formatPercent(11)        -> "11%"
 * formatPercent("11.5000") -> "11,5%"
 * formatPercent("11.0000") -> "11%"
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
 * Contoh:
 * formatDate("2026-09-26T14:00:00Z")       -> "26 Sep 2026"
 * formatDate("2026-09-26T14:00:00Z", true) -> "26 Sep 2026, 14:00"
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
