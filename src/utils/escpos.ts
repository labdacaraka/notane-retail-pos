/**
 * ESC/POS Binary Command Generator & Thermal Receipt Formatter
 * Mendukung printer thermal 58mm (32 kolom) dan 80mm (42-48 kolom)
 * Kompatibel dengan koneksi Web Serial API, Web Bluetooth, dan Raw Network/Tauri IPC.
 */

export interface EscPosOptions {
  paperWidth: '58mm' | '80mm'
  autoCut?: boolean
  openDrawer?: boolean
  storeName?: string
  storeTagline?: string
  storeAddress?: string
  storePhone?: string
  storeWebsite?: string
  taxId?: string
  headerNote?: string
  footerNote1?: string
  footerNote2?: string
  socialMedia?: string
  showPhone?: boolean
  showAddress?: boolean
  showCashierName?: boolean
  showCustomerInfo?: boolean
  showLoyaltyPoints?: boolean
  showTaxInfo?: boolean
  showBarcode?: boolean
  showTaxSummary?: boolean
}

export class EscPosBuilder {
  private buffer: number[] = []
  private encoder = new TextEncoder()

  constructor() {
    this.init()
  }

  // 1. Inisialisasi printer
  init(): this {
    this.buffer.push(0x1B, 0x40) // ESC @
    return this
  }

  // 2. Perataan teks
  alignLeft(): this {
    this.buffer.push(0x1B, 0x61, 0x00) // ESC a 0
    return this
  }

  alignCenter(): this {
    this.buffer.push(0x1B, 0x61, 0x01) // ESC a 1
    return this
  }

  alignRight(): this {
    this.buffer.push(0x1B, 0x61, 0x02) // ESC a 2
    return this
  }

  // 3. Gaya Teks
  bold(enable = true): this {
    this.buffer.push(0x1B, 0x45, enable ? 0x01 : 0x00) // ESC E n
    return this
  }

  doubleSize(enable = true): this {
    this.buffer.push(0x1D, 0x21, enable ? 0x11 : 0x00) // GS ! 0x11 / 0x00
    return this
  }

  // 4. Baris Baru & Teks
  text(str: string): this {
    const bytes = this.encoder.encode(str)
    for (let i = 0; i < bytes.length; i++) {
      this.buffer.push(bytes[i])
    }
    return this
  }

  textLine(str = ''): this {
    this.text(str)
    this.buffer.push(0x0A) // LF
    return this
  }

  feed(lines = 1): this {
    this.buffer.push(0x1B, 0x64, lines) // ESC d n
    return this
  }

  // 5. Potong Kertas (Auto-Cut)
  cut(feedLines = 3): this {
    this.feed(feedLines)
    this.buffer.push(0x1D, 0x56, 0x42, 0x00) // GS V 66 0 (Feed & Partial Cut)
    return this
  }

  // 6. Buka Laci Kasir (Cash Drawer Kick)
  kickDrawer(): this {
    this.buffer.push(0x1B, 0x70, 0x00, 0x19, 0xFA) // ESC p 0 25 250
    return this
  }

  // 7. Ambil Buffer Biner
  toUint8Array(): Uint8Array {
    return new Uint8Array(this.buffer)
  }
}

/**
 * Format string 2 kolom (kiri & kanan) sesuai lebar kertas
 */
export function formatTwoColumns(left: string, right: string, width: number): string {
  const rightLen = right.length
  const leftMax = Math.max(0, width - rightLen - 1)
  let leftTrimmed = left
  if (leftTrimmed.length > leftMax) {
    leftTrimmed = leftTrimmed.substring(0, leftMax)
  }
  const spaces = Math.max(1, width - leftTrimmed.length - rightLen)
  return leftTrimmed + ' '.repeat(spaces) + right
}

/**
 * Format separator horizontal (e.g. ------------------)
 */
export function formatSeparator(char = '-', width: number): string {
  return char.repeat(width)
}

/**
 * Format angka ke Rupiah ringkas (e.g. 15.000)
 */
function fmtRupiah(amount: number): string {
  return new Intl.NumberFormat('id-ID').format(Math.round(amount))
}

/**
 * Buat Buffer ESC/POS Biner untuk Struk Transaksi POS
 */
export function buildReceiptEscPos(receiptData: any, options: EscPosOptions): Uint8Array {
  const width = options.paperWidth === '80mm' ? 42 : 32
  const builder = new EscPosBuilder()

  // Buka Laci jika diminta
  if (options.openDrawer) {
    builder.kickDrawer()
  }

  // HEADER TOKO
  builder.alignCenter()
  builder.bold(true)
  builder.doubleSize(true)
  builder.textLine(options.storeName || 'NOTANE RETAIL STORE')
  builder.doubleSize(false)
  builder.bold(false)

  if (options.storeTagline) {
    builder.textLine(options.storeTagline)
  }
  if (options.showAddress !== false && options.storeAddress) {
    builder.textLine(options.storeAddress)
  }
  if (options.showPhone !== false && options.storePhone) {
    builder.textLine(`Telp: ${options.storePhone}`)
  }
  if (options.storeWebsite) {
    builder.textLine(options.storeWebsite)
  }
  if (options.taxId && options.showTaxInfo !== false) {
    builder.textLine(options.taxId)
  }
  if (options.headerNote) {
    builder.textLine(options.headerNote)
  }
  builder.textLine(formatSeparator('=', width))

  // INFO TRANSAKSI
  builder.alignLeft()
  const sale = receiptData.sale || {}
  const dateStr = sale.created_at ? new Date(sale.created_at).toLocaleString('id-ID') : new Date().toLocaleString('id-ID')
  builder.textLine(formatTwoColumns(`No: ${receiptData.invoice_number || sale.invoice_number || 'INV-DRAFT'}`, '', width))
  builder.textLine(formatTwoColumns(`Tgl: ${dateStr}`, '', width))
  
  if (options.showCashierName !== false) {
    builder.textLine(formatTwoColumns(`Kasir: ${receiptData.notes?.replace('Kasir: ', '') || 'Kasir 1'}`, `Term: ${receiptData.terminal_id || 'POS-01'}`, width))
  }
  if (options.showCustomerInfo !== false && receiptData.customer_name) {
    builder.textLine(formatTwoColumns(`Pelanggan: ${receiptData.customer_name}`, '', width))
  }
  builder.textLine(formatSeparator('-', width))

  // DAFTAR ITEM BELANJA
  builder.bold(true)
  builder.textLine(formatTwoColumns('ITEM', 'TOTAL', width))
  builder.bold(false)

  const items = receiptData.items || sale.items || []
  for (const it of items) {
    const name = it.product_name || it.product?.name || 'Produk'
    const qty = Number(it.quantity) || 1
    const unitPrice = Number(it.unit_price) || 0
    const subtotal = Number(it.subtotal) || (qty * unitPrice)
    const unitCode = it.selected_unit?.unit_code || it.unit_code || 'PCS'

    // Baris 1: Nama Produk
    builder.textLine(name)
    
    // Baris 2: Qty x Harga & Total
    const qtyPriceStr = `  ${qty} ${unitCode} x ${fmtRupiah(unitPrice)}`
    const subtotalStr = fmtRupiah(subtotal)
    builder.textLine(formatTwoColumns(qtyPriceStr, subtotalStr, width))

    // Diskon item jika ada
    if (it.discount_amount && Number(it.discount_amount) > 0) {
      builder.textLine(formatTwoColumns(`  (Diskon Item)`, `-${fmtRupiah(it.discount_amount)}`, width))
    }
  }

  builder.textLine(formatSeparator('-', width))

  // RINGKASAN PEMBAYARAN
  const totalAmount = Number(sale.total_amount || receiptData.total_amount || 0)
  const paidAmount = Number(sale.paid_amount || receiptData.paid_amount || 0)
  const changeAmount = Number(receiptData.change_amount || (paidAmount - totalAmount))
  const voucherDisc = Number(sale.discount_amount || 0)

  builder.alignLeft()
  if (voucherDisc > 0) {
    builder.textLine(formatTwoColumns('Diskon Voucher', `-${fmtRupiah(voucherDisc)}`, width))
  }

  builder.bold(true)
  builder.textLine(formatTwoColumns('TOTAL BELANJA', `Rp ${fmtRupiah(totalAmount)}`, width))
  builder.bold(false)

  // Rincian Metode Bayar
  const payments = receiptData.payments || sale.payments || []
  if (payments.length > 0) {
    for (const p of payments) {
      builder.textLine(formatTwoColumns(`Bayar (${p.payment_method})`, `Rp ${fmtRupiah(p.amount)}`, width))
    }
  } else {
    builder.textLine(formatTwoColumns('Bayar (Tunai)', `Rp ${fmtRupiah(paidAmount)}`, width))
  }

  builder.textLine(formatTwoColumns('KEMBALIAN', `Rp ${fmtRupiah(Math.max(0, changeAmount))}`, width))

  // POIN MEMBER
  if (options.showLoyaltyPoints !== false && receiptData.earned_points && receiptData.earned_points > 0) {
    builder.textLine(formatSeparator('.', width))
    builder.alignCenter()
    builder.textLine(`+${receiptData.earned_points} Poin Member Diperoleh!`)
  }

  // FOOTER STRUK
  builder.textLine(formatSeparator('=', width))
  builder.alignCenter()
  builder.textLine(options.footerNote1 || 'Terima Kasih Atas Kunjungan Anda')
  if (options.footerNote2) {
    builder.textLine(options.footerNote2)
  }
  if (options.socialMedia) {
    builder.textLine(options.socialMedia)
  }

  // Auto Cut Paper
  if (options.autoCut !== false) {
    builder.cut(4)
  } else {
    builder.feed(4)
  }

  return builder.toUint8Array()
}

/**
 * Buat Buffer ESC/POS untuk Pengujian Cetak Printer (Test Print)
 */
export function buildTestPrintEscPos(options: EscPosOptions & { connectionType: string; printerName: string }): Uint8Array {
  const width = options.paperWidth === '80mm' ? 42 : 32
  const builder = new EscPosBuilder()

  if (options.openDrawer) {
    builder.kickDrawer()
  }

  builder.alignCenter()
  builder.bold(true)
  builder.doubleSize(true)
  builder.textLine('NOTANE RETAIL POS')
  builder.doubleSize(false)
  builder.textLine('UJI CETAK THERMAL PRINTER')
  builder.bold(false)
  builder.textLine(formatSeparator('=', width))

  builder.alignLeft()
  builder.textLine(formatTwoColumns('Status', 'BERHASIL TERHUBUNG', width))
  builder.textLine(formatTwoColumns('Tipe Koneksi', options.connectionType, width))
  builder.textLine(formatTwoColumns('Nama Printer', options.printerName, width))
  builder.textLine(formatTwoColumns('Lebar Kertas', options.paperWidth, width))
  builder.textLine(formatTwoColumns('Waktu Uji', new Date().toLocaleTimeString('id-ID'), width))
  builder.textLine(formatSeparator('-', width))

  builder.alignCenter()
  builder.textLine('Printer thermal siap digunakan')
  builder.textLine('untuk cetak nota otomatis tanpa dialog.')
  builder.textLine(formatSeparator('=', width))

  if (options.autoCut !== false) {
    builder.cut(4)
  } else {
    builder.feed(4)
  }

  return builder.toUint8Array()
}

/**
 * Generate Teks Polos Struk untuk Preview Simulator
 */
export function generatePlainTextReceipt(receiptData: any, paperWidth: '58mm' | '80mm'): string {
  const width = paperWidth === '80mm' ? 42 : 32
  const lines: string[] = []

  lines.push('       NOTANE RETAIL POS       ')
  lines.push('  Jl. Raya Retail Enterprise 88')
  lines.push('      Telp: 0812-3456-7890     ')
  lines.push(formatSeparator('=', width))

  const sale = receiptData.sale || {}
  const dateStr = sale.created_at ? new Date(sale.created_at).toLocaleString('id-ID') : new Date().toLocaleString('id-ID')
  lines.push(formatTwoColumns(`No: ${receiptData.invoice_number || sale.invoice_number || 'INV-DRAFT'}`, '', width))
  lines.push(formatTwoColumns(`Tgl: ${dateStr}`, '', width))
  lines.push(formatTwoColumns(`Kasir: ${receiptData.notes?.replace('Kasir: ', '') || 'Kasir 1'}`, `Term: ${receiptData.terminal_id || 'POS-01'}`, width))
  lines.push(formatSeparator('-', width))

  const items = receiptData.items || sale.items || []
  for (const it of items) {
    const name = it.product_name || it.product?.name || 'Produk'
    const qty = Number(it.quantity) || 1
    const unitPrice = Number(it.unit_price) || 0
    const subtotal = Number(it.subtotal) || (qty * unitPrice)
    const unitCode = it.selected_unit?.unit_code || it.unit_code || 'PCS'

    lines.push(name)
    lines.push(formatTwoColumns(`  ${qty} ${unitCode} x ${fmtRupiah(unitPrice)}`, fmtRupiah(subtotal), width))
  }

  lines.push(formatSeparator('-', width))
  const totalAmount = Number(sale.total_amount || receiptData.total_amount || 0)
  const paidAmount = Number(sale.paid_amount || receiptData.paid_amount || 0)
  const changeAmount = Number(receiptData.change_amount || (paidAmount - totalAmount))

  lines.push(formatTwoColumns('TOTAL BELANJA', `Rp ${fmtRupiah(totalAmount)}`, width))
  lines.push(formatTwoColumns('BAYAR', `Rp ${fmtRupiah(paidAmount)}`, width))
  lines.push(formatTwoColumns('KEMBALIAN', `Rp ${fmtRupiah(Math.max(0, changeAmount))}`, width))
  lines.push(formatSeparator('=', width))
  lines.push('  Terima Kasih Atas Kunjungan Anda')

  return lines.join('\n')
}
