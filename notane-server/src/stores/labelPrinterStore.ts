import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'

function checkIsTauri(): boolean {
  try {
    return isTauri()
  } catch {
    return typeof (window as any).__TAURI_INTERNALS__ !== 'undefined'
  }
}

export type LabelConnectionType = 'SYSTEM' | 'SERIAL' | 'BLUETOOTH' | 'VIRTUAL'

export type LabelSizePreset =
  | '40x30'
  | '50x30'
  | '60x40'
  | '70x50'
  | '80x50'
  | '100x150'
  | '40x25'
  | '30x20'
  | '65x35'
  | 'a4_grid'
  | 'custom'

export type LabelSensorType = 'GAP' | 'BLACK_MARK' | 'CONTINUOUS'

export type LabelDarkness = 'LIGHT' | 'NORMAL' | 'DARK' | 'VERY_DARK'

export interface SystemPrinterDevice {
  name: string
  is_default: boolean
}

export interface LabelPrinterConfig {
  connectionType: LabelConnectionType
  printerName: string
  sizePreset: LabelSizePreset
  customWidthMm: number
  customHeightMm: number
  sensorType: LabelSensorType
  gapMm: number
  darkness: LabelDarkness
  printSpeed: number // 2 - 5 ips
  baudRate: number
  defaultQtyMode: 'SINGLE' | 'STOCK' // 1 per item atau sesuai jumlah stok
  autoClearAfterPrint: boolean
  a4Columns: number
  showStoreTitle?: boolean
  showBarcode?: boolean
  showDate?: boolean
  showSku?: boolean
  customStoreName?: string
}

const STORAGE_KEY = 'notane_label_printer_config'

// Daftar service UUID Bluetooth untuk printer label (TSPL / CPCL / ESC-POS / BLE transparent serial)
const BT_LABEL_SERVICES = [
  '000018f0-0000-1000-8000-00805f9b34fb', // Standard printer service
  '49535343-fe7d-4ae5-8fa9-9fafd205e455', // ISSC transparent serial (Xprinter, Panda, TSC)
  'e7810a71-73ae-499d-8c15-faa9aef0c3f2', // Goojprt / PT-210 / MPT-II / Xprinter
  '0000ff00-0000-1000-8000-00805f9b34fb', // Custom VSC / Hoin / Milestone
  '0000fee7-0000-1000-8000-00805f9b34fb', // Tencent / Ali Gen
  '0000ae00-0000-1000-8000-00805f9b34fb', // Zjiang / Iware
  '0000e0ff-0000-1000-8000-00805f9b34fb', // Mini Bluetooth
  '0000ffe0-0000-1000-8000-00805f9b34fb', // HM-10 BLE module
]

export const useLabelPrinterStore = defineStore('labelPrinter', () => {
  // Config state
  const connectionType = ref<LabelConnectionType>('SYSTEM')
  const printerName = ref<string>('Driver Sistem (OS Default / Browser)')
  const sizePreset = ref<LabelSizePreset>('40x30')
  const customWidthMm = ref<number>(40)
  const customHeightMm = ref<number>(30)
  const sensorType = ref<LabelSensorType>('GAP')
  const gapMm = ref<number>(2)
  const darkness = ref<LabelDarkness>('NORMAL')
  const printSpeed = ref<number>(3)
  const baudRate = ref<number>(9600)
  const defaultQtyMode = ref<'SINGLE' | 'STOCK'>('SINGLE')
  const autoClearAfterPrint = ref<boolean>(false)
  const a4Columns = ref<number>(2)
  const showStoreTitle = ref<boolean>(true)
  const showBarcode = ref<boolean>(true)
  const showDate = ref<boolean>(true)
  const showSku = ref<boolean>(true)
  const customStoreName = ref<string>('')

  // Operational state
  const isConnected = ref<boolean>(true) // System and Virtual are ready by default
  const isConnecting = ref<boolean>(false)
  const statusMessage = ref<string>('Siap mencetak')
  const lastPrintedText = ref<string>('')
  const lastPrintTime = ref<string>('')

  // System printers (OS detected in Tauri)
  const systemPrinters = ref<SystemPrinterDevice[]>([])
  const isFetchingPrinters = ref<boolean>(false)

  // Non-reactive hardware handles
  let serialPort: any = null
  let bluetoothDevice: any = null
  let bluetoothCharacteristic: any = null

  // Computed
  const isWebSerialSupported = computed(() => {
    return typeof navigator !== 'undefined' && 'serial' in navigator
  })

  const isWebBluetoothSupported = computed(() => {
    return typeof navigator !== 'undefined' && 'bluetooth' in navigator
  })

  // Efektif ukuran lebar & tinggi dalam mm
  const effectiveWidthMm = computed(() => {
    switch (sizePreset.value) {
      case '40x30': return 40
      case '50x30': return 50
      case '60x40': return 60
      case '70x50': return 70
      case '80x50': return 80
      case '100x150': return 100
      case '40x25': return 40
      case '30x20': return 30
      case '65x35': return 65
      case 'a4_grid': return 210
      case 'custom': return customWidthMm.value || 40
      default: return 40
    }
  })

  const effectiveHeightMm = computed(() => {
    switch (sizePreset.value) {
      case '40x30': return 30
      case '50x30': return 30
      case '60x40': return 40
      case '70x50': return 50
      case '80x50': return 50
      case '100x150': return 150
      case '40x25': return 25
      case '30x20': return 20
      case '65x35': return 35
      case 'a4_grid': return 297
      case 'custom': return customHeightMm.value || 30
      default: return 30
    }
  })

  // Load config from localStorage
  function loadConfig() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      if (saved) {
        const parsed: LabelPrinterConfig = JSON.parse(saved)
        connectionType.value = parsed.connectionType || 'SYSTEM'
        printerName.value = parsed.printerName || 'Driver Sistem (OS Default / Browser)'
        sizePreset.value = parsed.sizePreset || '40x30'
        customWidthMm.value = parsed.customWidthMm || 40
        customHeightMm.value = parsed.customHeightMm || 30
        sensorType.value = parsed.sensorType || 'GAP'
        gapMm.value = parsed.gapMm ?? 2
        darkness.value = parsed.darkness || 'NORMAL'
        printSpeed.value = parsed.printSpeed || 3
        baudRate.value = parsed.baudRate || 9600
        defaultQtyMode.value = parsed.defaultQtyMode || 'SINGLE'
        autoClearAfterPrint.value = parsed.autoClearAfterPrint || false
        a4Columns.value = parsed.a4Columns || 2
        showStoreTitle.value = parsed.showStoreTitle ?? true
        showBarcode.value = parsed.showBarcode ?? true
        showDate.value = parsed.showDate ?? true
        showSku.value = parsed.showSku ?? true
        customStoreName.value = parsed.customStoreName || ''

        if (connectionType.value === 'SYSTEM') {
          isConnected.value = true
          statusMessage.value = 'Driver sistem siap'
        } else if (connectionType.value === 'VIRTUAL') {
          isConnected.value = true
          statusMessage.value = 'Simulator label siap'
        } else {
          isConnected.value = false
          statusMessage.value = 'Perlu disambungkan ulang'
        }
      }
    } catch (e) {
      console.warn('Gagal membaca konfigurasi printer label:', e)
    }
  }

  // Save config to localStorage
  function saveConfig() {
    try {
      const cfg: LabelPrinterConfig = {
        connectionType: connectionType.value,
        printerName: printerName.value,
        sizePreset: sizePreset.value,
        customWidthMm: customWidthMm.value,
        customHeightMm: customHeightMm.value,
        sensorType: sensorType.value,
        gapMm: gapMm.value,
        darkness: darkness.value,
        printSpeed: printSpeed.value,
        baudRate: baudRate.value,
        defaultQtyMode: defaultQtyMode.value,
        autoClearAfterPrint: autoClearAfterPrint.value,
        a4Columns: a4Columns.value,
        showStoreTitle: showStoreTitle.value,
        showBarcode: showBarcode.value,
        showDate: showDate.value,
        showSku: showSku.value,
        customStoreName: customStoreName.value,
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg))
    } catch (e) {
      console.warn('Gagal menyimpan konfigurasi printer label:', e)
    }
  }

  // Set mode Driver Sistem
  function setSystemDriver(name = 'Driver Sistem (OS Default / Browser)') {
    connectionType.value = 'SYSTEM'
    printerName.value = name
    isConnected.value = true
    statusMessage.value = 'Driver sistem siap mencetak'
    saveConfig()
  }

  // Set mode Virtual Simulator
  function setVirtualSimulator(name = 'Simulator Printer Label') {
    connectionType.value = 'VIRTUAL'
    printerName.value = name
    isConnected.value = true
    statusMessage.value = 'Simulator label aktif'
    saveConfig()
  }

  // Connect Serial / USB (Web Serial API)
  async function connectSerial(chosenBaudRate = 9600): Promise<{ success: boolean; message: string }> {
    if (!isWebSerialSupported.value) {
      throw new Error('Browser ini tidak mendukung Web Serial API. Gunakan Chrome, Edge, atau aplikasi desktop resmi.')
    }

    isConnecting.value = true
    statusMessage.value = 'Menghubungkan ke port serial label...'

    try {
      if (serialPort) {
        try {
          await serialPort.close()
        } catch {
          // ignore
        }
      }

      const port = await (navigator as any).serial.requestPort()
      await port.open({ baudRate: chosenBaudRate })

      serialPort = port
      connectionType.value = 'SERIAL'
      baudRate.value = chosenBaudRate

      const info = port.getInfo ? port.getInfo() : {}
      const vid = info.usbVendorId ? `0x${info.usbVendorId.toString(16).toUpperCase()}` : ''
      const pid = info.usbProductId ? `0x${info.usbProductId.toString(16).toUpperCase()}` : ''
      printerName.value = vid ? `Label Printer USB (VID:${vid} PID:${pid})` : 'Label Printer Serial COM'

      isConnected.value = true
      isConnecting.value = false
      statusMessage.value = `Terhubung ke ${printerName.value} (${chosenBaudRate} bps)`

      saveConfig()
      return {
        success: true,
        message: `Berhasil terhubung ke ${printerName.value}`,
      }
    } catch (err: any) {
      isConnecting.value = false
      if (err.name !== 'NotFoundError') {
        isConnected.value = false
        statusMessage.value = `Koneksi serial gagal: ${err.message}`
      }
      throw err
    }
  }

  async function disconnectSerial() {
    if (serialPort) {
      try {
        await serialPort.close()
      } catch {
        // ignore
      }
      serialPort = null
    }
    isConnected.value = false
    statusMessage.value = 'Port serial diputuskan'
  }

  // Connect Bluetooth (Web Bluetooth API)
  async function connectBluetooth(): Promise<{ success: boolean; message: string }> {
    if (!isWebBluetoothSupported.value) {
      throw new Error('Browser ini tidak mendukung Web Bluetooth API. Gunakan Chrome atau Edge.')
    }

    isConnecting.value = true
    statusMessage.value = 'Mencari printer label bluetooth...'

    try {
      const device = await (navigator as any).bluetooth.requestDevice({
        filters: [
          { services: BT_LABEL_SERVICES },
          { namePrefix: 'XP-' },
          { namePrefix: 'Xprinter' },
          { namePrefix: 'TSC' },
          { namePrefix: 'Label' },
          { namePrefix: 'POS' },
          { namePrefix: 'Printer' },
          { namePrefix: 'PT-' },
          { namePrefix: 'HM-' },
        ],
        optionalServices: BT_LABEL_SERVICES,
      })

      device.addEventListener('gattserverdisconnected', () => {
        isConnected.value = false
        statusMessage.value = `Koneksi Bluetooth ${device.name || 'Printer'} terputus`
      })

      const server = await device.gatt.connect()
      let writeChar: any = null

      for (const sUuid of BT_LABEL_SERVICES) {
        try {
          const service = await server.getPrimaryService(sUuid)
          const chars = await service.getCharacteristics()
          for (const ch of chars) {
            if (ch.properties.write || ch.properties.writeWithoutResponse) {
              writeChar = ch
              break
            }
          }
          if (writeChar) break
        } catch {
          // continue
        }
      }

      if (!writeChar) {
        throw new Error('GATT write characteristic tidak ditemukan pada printer label ini.')
      }

      bluetoothDevice = device
      bluetoothCharacteristic = writeChar
      connectionType.value = 'BLUETOOTH'
      printerName.value = device.name || 'Printer Label Bluetooth'
      isConnected.value = true
      isConnecting.value = false
      statusMessage.value = `Terhubung ke ${printerName.value}`

      saveConfig()
      return {
        success: true,
        message: `Berhasil terhubung ke ${printerName.value}`,
      }
    } catch (err: any) {
      isConnecting.value = false
      if (err.name !== 'NotFoundError') {
        isConnected.value = false
        statusMessage.value = `Koneksi bluetooth gagal: ${err.message}`
      }
      throw err
    }
  }

  function disconnectBluetooth() {
    if (bluetoothDevice && bluetoothDevice.gatt.connected) {
      bluetoothDevice.gatt.disconnect()
    }
    bluetoothDevice = null
    bluetoothCharacteristic = null
    isConnected.value = false
    statusMessage.value = 'Koneksi bluetooth diputuskan'
  }

  // Mengambil daftar printer sistem OS via Tauri IPC
  async function fetchSystemPrinters(): Promise<SystemPrinterDevice[]> {
    if (!checkIsTauri()) {
      return []
    }
    isFetchingPrinters.value = true
    try {
      const list = await invoke<SystemPrinterDevice[]>('get_system_printers')
      systemPrinters.value = list || []
      // Jika mode SYSTEM dan printer belum diatur secara spesifik, pilih printer default
      if (connectionType.value === 'SYSTEM') {
        const isGenericName = !printerName.value || printerName.value.includes('Driver Sistem') || printerName.value.includes('OS Default')
        if (isGenericName && systemPrinters.value.length > 0) {
          const def = systemPrinters.value.find((p) => p.is_default) || systemPrinters.value[0]
          printerName.value = def.name
          saveConfig()
        }
      }
      return systemPrinters.value
    } catch (err) {
      console.warn('Gagal membaca printer sistem:', err)
      return []
    } finally {
      isFetchingPrinters.value = false
    }
  }

  // Kirim raw text / binary data ke printer
  async function writeRaw(data: Uint8Array | string): Promise<void> {
    const bytes = typeof data === 'string' ? new TextEncoder().encode(data) : data
    const strData = typeof data === 'string' ? data : new TextDecoder().decode(data)

    if (connectionType.value === 'VIRTUAL') {
      return
    }

    if (connectionType.value === 'SYSTEM') {
      if (checkIsTauri()) {
        await invoke('silent_print_raw', {
          printerName: printerName.value,
          rawData: strData,
        })
      }
      return
    }

    if (connectionType.value === 'SERIAL') {
      if (!serialPort || !serialPort.writable) {
        throw new Error('Port serial printer label belum terbuka atau terputus.')
      }
      const writer = serialPort.writable.getWriter()
      try {
        await writer.write(bytes)
      } finally {
        writer.releaseLock()
      }
      return
    }

    if (connectionType.value === 'BLUETOOTH') {
      if (!bluetoothCharacteristic) {
        throw new Error('Printer label bluetooth belum terhubung.')
      }
      const chunkSize = 100
      for (let i = 0; i < bytes.length; i += chunkSize) {
        const chunk = bytes.slice(i, i + chunkSize)
        if (bluetoothCharacteristic.properties.writeWithoutResponse) {
          await bluetoothCharacteristic.writeValueWithoutResponse(chunk)
        } else {
          await bluetoothCharacteristic.writeValue(chunk)
        }
        await new Promise((r) => setTimeout(r, 20))
      }
    }
  }

  // Bangun perintah TSPL untuk uji cetak
  function buildTestPrintTspl(): string {
    const w = effectiveWidthMm.value
    const h = effectiveHeightMm.value
    const g = gapMm.value
    const d = darkness.value === 'LIGHT' ? 2 : darkness.value === 'DARK' ? 10 : darkness.value === 'VERY_DARK' ? 14 : 6
    const spd = printSpeed.value

    return [
      `SIZE ${w} mm, ${h} mm`,
      `GAP ${g} mm, 0 mm`,
      `SPEED ${spd}`,
      `DENSITY ${d}`,
      'DIRECTION 1',
      'CLS',
      'TEXT 15,10,"3",0,1,1,"NOTANE RETAIL"',
      `TEXT 15,35,"2",0,1,1,"UJI CETAK: ${sizePreset.value.toUpperCase()} (${w}x${h}mm)"`,
      'BARCODE 15,60,"128",40,1,0,2,2,"899275312345"',
      'TEXT 15,115,"4",0,1,1,"Rp 25.000"',
      `TEXT 15,145,"1",0,1,1,"WAKTU: ${new Date().toLocaleTimeString('id-ID')}"`,
      'PRINT 1,1',
      '',
    ].join('\r\n')
  }

  // Uji Cetak Label (Silent Print)
  async function testPrint(): Promise<{ success: boolean; message: string }> {
    lastPrintTime.value = new Date().toLocaleTimeString('id-ID')
    const w = effectiveWidthMm.value
    const h = effectiveHeightMm.value

    lastPrintedText.value = [
      '==================================',
      '       UJI CETAK NOTANE LABEL     ',
      '==================================',
      `Printer: ${printerName.value}`,
      `Koneksi: ${connectionType.value}`,
      `Preset : ${sizePreset.value} (${w}mm x ${h}mm)`,
      `Sensor : ${sensorType.value} (Gap ${gapMm.value}mm)`,
      `Darkness: ${darkness.value} (Speed: ${printSpeed.value} ips)`,
      `Waktu  : ${lastPrintTime.value}`,
      'Produk : Sampel Kopi Gula Aren',
      'Barcode: 899275312345',
      'Harga  : Rp 25.000',
      'STATUS : BERHASIL MENCETAK',
      '==================================',
    ].join('\n')

    if (connectionType.value === 'SERIAL' || connectionType.value === 'BLUETOOTH') {
      const tspl = buildTestPrintTspl()
      await writeRaw(tspl)
      statusMessage.value = `Uji cetak dikirim ke ${printerName.value}`
      return {
        success: true,
        message: `Uji cetak TSPL berhasil dikirim ke ${printerName.value}!`,
      }
    }

    if (connectionType.value === 'SYSTEM') {
      if (checkIsTauri()) {
        const tspl = buildTestPrintTspl()
        await invoke('silent_print_raw', {
          printerName: printerName.value,
          rawData: tspl,
        })
        statusMessage.value = `Uji cetak dikirim ke printer OS: ${printerName.value}`
        return {
          success: true,
          message: `Uji cetak berhasil dikirim ke printer [${printerName.value}] tanpa dialog!`,
        }
      }
      statusMessage.value = `Uji cetak (${w}x${h}mm) disimulasikan`
      return {
        success: true,
        message: `Mode Driver Sistem siap. Uji cetak label ${w}x${h}mm disimulasikan.`,
      }
    }

    // VIRTUAL
    statusMessage.value = `Uji cetak (${w}x${h}mm) berhasil disimulasikan`
    return {
      success: true,
      message: `Uji cetak simulator label ${w}x${h}mm berhasil!`,
    }
  }

  // Cetak daftar label aktual via TSPL / Direct Spooler (Silent Print)
  async function printTsplLabels(
    items: Array<{
      name: string
      barcode: string
      price: number
      sku?: string
      storeTitle?: string
      qty: number
    }>
  ): Promise<{ success: boolean; count: number; message: string }> {
    if (items.length === 0) return { success: true, count: 0, message: 'Tidak ada item label yang dipilih' }

    const w = effectiveWidthMm.value
    const h = effectiveHeightMm.value
    const g = gapMm.value
    const d = darkness.value === 'LIGHT' ? 2 : darkness.value === 'DARK' ? 10 : darkness.value === 'VERY_DARK' ? 14 : 6
    const spd = printSpeed.value

    let totalPrinted = 0

    // 1. Jika mode SYSTEM dan berjalan di Tauri Desktop: kirim langsung batch TSPL via silent_print_raw
    if (connectionType.value === 'SYSTEM' && checkIsTauri()) {
      let batchTspl = ''
      for (const item of items) {
        if (item.qty <= 0) continue
        const cleanName = (item.name || '').replace(/[\r\n"']/g, '').substring(0, 32)
        const cleanStore = (item.storeTitle || 'NOTANE RETAIL').replace(/[\r\n"']/g, '').substring(0, 24)
        const cleanBarcode = (item.barcode || item.sku || '000000').replace(/[^0-9A-Za-z]/g, '')
        const formattedPrice = `Rp ${Number(item.price || 0).toLocaleString('id-ID')}`

        batchTspl += [
          `SIZE ${w} mm, ${h} mm`,
          `GAP ${g} mm, 0 mm`,
          `SPEED ${spd}`,
          `DENSITY ${d}`,
          'DIRECTION 1',
          'CLS',
          `TEXT 15,10,"2",0,1,1,"${cleanStore}"`,
          `TEXT 15,35,"2",0,1,1,"${cleanName}"`,
          cleanBarcode ? `BARCODE 15,65,"128",40,1,0,2,2,"${cleanBarcode}"` : '',
          `TEXT 15,115,"4",0,1,1,"${formattedPrice}"`,
          item.sku ? `TEXT 15,145,"1",0,1,1,"SKU: ${item.sku}"` : '',
          `PRINT ${item.qty},1`,
          '',
        ].filter(Boolean).join('\r\n') + '\r\n'

        totalPrinted += item.qty
      }

      await invoke('silent_print_raw', {
        printerName: printerName.value,
        rawData: batchTspl,
      })

      lastPrintTime.value = new Date().toLocaleTimeString('id-ID')
      lastPrintedText.value = `Sukses mencetak ${totalPrinted} label ke ${printerName.value} pada ${lastPrintTime.value}`
      statusMessage.value = `Berhasil mencetak ${totalPrinted} label`

      return {
        success: true,
        count: totalPrinted,
        message: `Berhasil mencetak ${totalPrinted} label langsung ke ${printerName.value} (Silent Print)`,
      }
    }

    // 2. Jika mode SERIAL, BLUETOOTH, atau VIRTUAL
    for (const item of items) {
      if (item.qty <= 0) continue

      const cleanName = (item.name || '').replace(/[\r\n"']/g, '').substring(0, 32)
      const cleanStore = (item.storeTitle || 'NOTANE RETAIL').replace(/[\r\n"']/g, '').substring(0, 24)
      const cleanBarcode = (item.barcode || item.sku || '000000').replace(/[^0-9A-Za-z]/g, '')
      const formattedPrice = `Rp ${Number(item.price || 0).toLocaleString('id-ID')}`

      const tspl = [
        `SIZE ${w} mm, ${h} mm`,
        `GAP ${g} mm, 0 mm`,
        `SPEED ${spd}`,
        `DENSITY ${d}`,
        'DIRECTION 1',
        'CLS',
        `TEXT 15,10,"2",0,1,1,"${cleanStore}"`,
        `TEXT 15,35,"2",0,1,1,"${cleanName}"`,
        cleanBarcode ? `BARCODE 15,65,"128",40,1,0,2,2,"${cleanBarcode}"` : '',
        `TEXT 15,115,"4",0,1,1,"${formattedPrice}"`,
        item.sku ? `TEXT 15,145,"1",0,1,1,"SKU: ${item.sku}"` : '',
        `PRINT ${item.qty},1`,
        '',
      ].filter(Boolean).join('\r\n')

      await writeRaw(tspl)
      totalPrinted += item.qty
      if (connectionType.value === 'SERIAL' || connectionType.value === 'BLUETOOTH') {
        await new Promise((r) => setTimeout(r, 60))
      }
    }

    lastPrintTime.value = new Date().toLocaleTimeString('id-ID')
    lastPrintedText.value = `Sukses mencetak ${totalPrinted} label ke ${printerName.value} pada ${lastPrintTime.value}`
    statusMessage.value = `Berhasil mencetak ${totalPrinted} label`

    return {
      success: true,
      count: totalPrinted,
      message: `Berhasil mencetak ${totalPrinted} label ke [${printerName.value}]`,
    }
  }

  // Inisialisasi
  loadConfig()

  return {
    // State
    connectionType,
    printerName,
    sizePreset,
    customWidthMm,
    customHeightMm,
    sensorType,
    gapMm,
    darkness,
    printSpeed,
    baudRate,
    defaultQtyMode,
    autoClearAfterPrint,
    a4Columns,
    showStoreTitle,
    showBarcode,
    showDate,
    showSku,
    customStoreName,
    isConnected,
    isConnecting,
    statusMessage,
    lastPrintedText,
    lastPrintTime,
    systemPrinters,
    isFetchingPrinters,
    isWebSerialSupported,
    isWebBluetoothSupported,
    effectiveWidthMm,
    effectiveHeightMm,

    // Actions
    loadConfig,
    saveConfig,
    setSystemDriver,
    setVirtualSimulator,
    connectSerial,
    disconnectSerial,
    connectBluetooth,
    disconnectBluetooth,
    fetchSystemPrinters,
    writeRaw,
    testPrint,
    printTsplLabels,
  }
})
