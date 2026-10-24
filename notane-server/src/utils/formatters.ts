/**
 * Utility Formatter Terpusat untuk Notane Retail POS
 * Mengatasi masalah format angka desimal PostgreSQL NUMERIC (seperti 25.00000 -> 25 atau 25,0)
 * serta standardisasi format mata uang Rupiah dan tanggal Indonesia.
 */

/**
 * Format angka desimal rapi untuk kuantitas, berat, atau nilai desimal.
 * Menghilangkan deretan nol trailing yang tidak perlu atau membatasi presisi.
 *
 * Contoh:
 * formatDecimal(25.00000)          -> "25"
 * formatDecimal(25.00000, 1, true) -> "25,0"  (jika forceDecimal aktif)
 * formatDecimal("25.50000", 2)     -> "25,5"
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

  const num = typeof val === 'number' ? val : parseFloat(String(val))
  if (isNaN(num)) return '0'

  if (forceDecimal) {
    return new Intl.NumberFormat('id-ID', {
      minimumFractionDigits: 1,
      maximumFractionDigits: maxDecimals,
    }).format(num)
  }

  // Jika angka bulat sempurna dan tidak dipaksa desimal, tampilkan bulat tanpa trailing zeros
  if (Number.isInteger(num)) {
    return new Intl.NumberFormat('id-ID', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(num)
  }

  return new Intl.NumberFormat('id-ID', {
    minimumFractionDigits: 0,
    maximumFractionDigits: maxDecimals,
  }).format(num)
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
 * Contoh:
 * formatCurrency(25000)        -> "25.000"
 * formatCurrency(25000, true)  -> "Rp 25.000"
 * formatCurrency(25000.5)      -> "25.000,50"
 */
export function formatCurrency(
  val: number | string | null | undefined,
  withSymbol: boolean = false
): string {
  if (val === null || val === undefined || val === '') {
    return withSymbol ? 'Rp 0' : '0'
  }

  const num = typeof val === 'number' ? val : parseFloat(String(val))
  if (isNaN(num)) return withSymbol ? 'Rp 0' : '0'

  // Cek apakah ada nilai pecahan sen/desimal
  const hasFraction = Math.abs(num % 1) > 0.001
  const minDec = hasFraction ? 2 : 0
  const maxDec = hasFraction ? 2 : 0

  const formatted = new Intl.NumberFormat('id-ID', {
    minimumFractionDigits: minDec,
    maximumFractionDigits: maxDec,
  }).format(num)

  return withSymbol ? `Rp ${formatted}` : formatted
}

/**
 * Format persentase.
 * Contoh:
 * formatPercent(11)        -> "11%"
 * formatPercent("11.5000") -> "11,5%"
 */
export function formatPercent(val: number | string | null | undefined, maxDecimals: number = 2): string {
  if (val === null || val === undefined || val === '') return '0%'
  const num = typeof val === 'number' ? val : parseFloat(String(val))
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
