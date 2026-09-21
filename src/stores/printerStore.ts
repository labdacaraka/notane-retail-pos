import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/services/api'
import {
  buildReceiptEscPos,
  buildTestPrintEscPos,
  generatePlainTextReceipt,
  type EscPosOptions,
} from '@/utils/escpos'

export type PrinterConnectionType = 'BLUETOOTH' | 'SERIAL' | 'VIRTUAL'
export type PaperWidth = '58mm' | '80mm'

export interface PrinterConfig {
  connectionType: PrinterConnectionType
  printerName: string
  paperWidth: PaperWidth
  baudRate: number
  autoPrint: boolean
  autoCut: boolean
  openDrawer: boolean
}

// Common Bluetooth Thermal Printer ESC/POS GATT Service UUIDs
const BT_ESC_POS_SERVICES = [
  '000018f0-0000-1000-8000-00805f9b34fb', // Standard ESC/POS Service
  'e7810a71-73ae-499d-8c15-faa9aef0c3f2', // Goojprt / PT-210 / MPT-II / Xprinter
  '49535343-fe7d-4ae5-8fa9-9fafd205e455', // ISSC Transparent Serial (Panda, Zjiang, dsb)
  '0000e0ff-0000-1000-8000-00805f9b34fb', // Common Mini Bluetooth Printer (MPT / POS-58)
  '0000ff00-0000-1000-8000-00805f9b34fb', // Custom VSC / Hoin / Milestone
  '0000fee7-0000-1000-8000-00805f9b34fb', // Tencent / Ali Gen Printer Service
  '0000ae00-0000-1000-8000-00805f9b34fb', // Zjiang ZJ-5802 / Iware
  '0000ae30-0000-1000-8000-00805f9b34fb', // ZJ series extended
  '0000ffff-0000-1000-8000-00805f9b34fb', // Generic OEM Mini Printer
  '0000ffe0-0000-1000-8000-00805f9b34fb', // HM-10 / CC2541 BLE modules
  '0000fff0-0000-1000-8000-00805f9b34fb', // Silicon Labs BLE Printer
  'bef8d6c9-9c21-4c9e-b632-bd58c1009f9f', // Custom ESC/POS vendor service
]

const STORAGE_KEY = 'notane_printer_config'

export const usePrinterStore = defineStore('printer', () => {
  // Config state
  const connectionType = ref<PrinterConnectionType>('VIRTUAL')
  const printerName = ref<string>('Simulator Thermal 58mm')
  const paperWidth = ref<PaperWidth>('58mm')
  const baudRate = ref<number>(9600)
  const autoPrint = ref<boolean>(true)
  const autoCut = ref<boolean>(true)
  const openDrawer = ref<boolean>(true)

  // Database-backed Receipt Settings
  const receiptSettings = ref<any>(null)
  const isSettingsLoading = ref<boolean>(false)

  // Operational state
  const isConnected = ref<boolean>(true) // Virtual is connected by default
  const isConnecting = ref<boolean>(false)
  const statusMessage = ref<string>('Siap mencetak')
  const lastPrintedText = ref<string>('')
  const lastPrintTime = ref<string>('')

  // Hardware handles (non-reactive)
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

  // Load saved config on initialization
  function loadConfig() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      if (saved) {
        const parsed: PrinterConfig = JSON.parse(saved)
        connectionType.value = parsed.connectionType || 'VIRTUAL'
        printerName.value = parsed.printerName || 'Simulator Thermal 58mm'
        paperWidth.value = parsed.paperWidth || '58mm'
        baudRate.value = parsed.baudRate || 9600
        autoPrint.value = parsed.autoPrint !== false
        autoCut.value = parsed.autoCut !== false
        openDrawer.value = parsed.openDrawer !== false

        // If Virtual, it's always ready
        if (connectionType.value === 'VIRTUAL') {
          isConnected.value = true
          statusMessage.value = 'Simulator siap'
        } else {
          isConnected.value = false
          statusMessage.value = 'Perlu disambungkan ulang'
        }
      }
    } catch (e) {
      console.warn('Gagal membaca konfigurasi printer:', e)
    }
  }

  // Save current config
  function saveConfig() {
    try {
      const cfg: PrinterConfig = {
        connectionType: connectionType.value,
        printerName: printerName.value,
        paperWidth: paperWidth.value,
        baudRate: baudRate.value,
        autoPrint: autoPrint.value,
        autoCut: autoCut.value,
        openDrawer: openDrawer.value,
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg))
    } catch (e) {
      console.warn('Gagal menyimpan konfigurasi printer:', e)
    }
  }

  // 1. Connect Serial / USB Printer
  async function connectSerial(chosenBaudRate = 9600): Promise<{ success: boolean; message: string }> {
    if (!isWebSerialSupported.value) {
      throw new Error('Browser ini tidak mendukung Web Serial API. Gunakan Google Chrome, Microsoft Edge, atau Opera.')
    }

    isConnecting.value = true
    statusMessage.value = 'Menghubungkan ke port serial...'

    try {
      // Disconnect previous if open
      if (serialPort) {
        try {
          await serialPort.close()
        } catch {
          // ignore
        }
      }

      // Request port from user
      const port = await (navigator as any).serial.requestPort()
      await port.open({ baudRate: chosenBaudRate })

      serialPort = port
      connectionType.value = 'SERIAL'
      baudRate.value = chosenBaudRate

      // Get info if available
      const info = port.getInfo ? port.getInfo() : {}
      const usbVendor = info.usbVendorId ? `USB:${info.usbVendorId.toString(16)}` : ''
      printerName.value = usbVendor ? `Thermal Printer (${usbVendor})` : `Serial Printer (${chosenBaudRate} bps)`

      isConnected.value = true
      isConnecting.value = false
      statusMessage.value = `Terhubung ke ${printerName.value}`
      saveConfig()

      return { success: true, message: `Berhasil terhubung ke ${printerName.value}` }
    } catch (err: any) {
      isConnecting.value = false
      if (err.name === 'NotFoundError') {
        statusMessage.value = 'Pemilihan port serial dibatalkan'
        throw new Error('Pemilihan port serial dibatalkan oleh pengguna.')
      }
      statusMessage.value = err.message || 'Gagal membuka port serial'
      throw err
    }
  }

  // Disconnect Serial
  async function disconnectSerial() {
    if (serialPort) {
      try {
        await serialPort.close()
      } catch (e) {
        console.warn('Error saat menutup port serial:', e)
      }
      serialPort = null
    }
    if (connectionType.value === 'SERIAL') {
      isConnected.value = false
      statusMessage.value = 'Port serial terputus'
    }
  }

  // 2. Connect Bluetooth Printer
  async function connectBluetooth(): Promise<{ success: boolean; message: string }> {
    if (!isWebBluetoothSupported.value) {
      throw new Error('Browser ini tidak mendukung Web Bluetooth API. Gunakan Chrome di Windows, Mac, Linux, atau Android.')
    }

    isConnecting.value = true
    statusMessage.value = 'Mencari printer bluetooth...'

    try {
      // Disconnect previous
      if (bluetoothDevice && bluetoothDevice.gatt && bluetoothDevice.gatt.connected) {
        bluetoothDevice.gatt.disconnect()
      }

      const device = await (navigator as any).bluetooth.requestDevice({
        acceptAllDevices: true,
        optionalServices: BT_ESC_POS_SERVICES,
      })

      statusMessage.value = `Menyambungkan ke ${device.name || 'Bluetooth Printer'}...`
      const server = await device.gatt.connect()

      // Find primary service and writable characteristic
      let writableChar: any = null
      for (const sUuid of BT_ESC_POS_SERVICES) {
        try {
          const service = await server.getPrimaryService(sUuid)
          const characteristics = await service.getCharacteristics()
          for (const char of characteristics) {
            if (char.properties.write || char.properties.writeWithoutResponse) {
              writableChar = char
              break
            }
          }
          if (writableChar) break
        } catch {
          // try next service UUID
        }
      }

      // If specific service not found, query all services
      if (!writableChar) {
        const services = await server.getPrimaryServices()
        for (const service of services) {
          try {
            const characteristics = await service.getCharacteristics()
            for (const char of characteristics) {
              if (char.properties.write || char.properties.writeWithoutResponse) {
                writableChar = char
                break
              }
            }
            if (writableChar) break
          } catch {
            // continue
          }
        }
      }

      if (!writableChar) {
        throw new Error('Tidak ditemukan karakteristik write ESC/POS pada printer bluetooth ini.')
      }

      bluetoothDevice = device
      bluetoothCharacteristic = writableChar
      connectionType.value = 'BLUETOOTH'
      printerName.value = device.name || 'Bluetooth Thermal Printer'
      isConnected.value = true
      isConnecting.value = false
      statusMessage.value = `Terhubung via Bluetooth (${printerName.value})`
      saveConfig()

      // Listen for disconnect
      device.addEventListener('gattserverdisconnected', () => {
        isConnected.value = false
        statusMessage.value = 'Koneksi bluetooth terputus'
      })

      return { success: true, message: `Berhasil terhubung ke ${printerName.value}` }
    } catch (err: any) {
      isConnecting.value = false
      if (err.name === 'NotFoundError') {
        statusMessage.value = 'Pencarian bluetooth dibatalkan'
        throw new Error('Pencarian bluetooth dibatalkan oleh pengguna.')
      }
      statusMessage.value = err.message || 'Gagal menyambung bluetooth'
      throw err
    }
  }

  // Disconnect Bluetooth
  function disconnectBluetooth() {
    if (bluetoothDevice && bluetoothDevice.gatt && bluetoothDevice.gatt.connected) {
      bluetoothDevice.gatt.disconnect()
    }
    bluetoothDevice = null
    bluetoothCharacteristic = null
    if (connectionType.value === 'BLUETOOTH') {
      isConnected.value = false
      statusMessage.value = 'Bluetooth terputus'
    }
  }

  // 3. Switch to Virtual Thermal Simulator
  function setVirtualPrinter(name = 'Simulator Thermal 58mm', width: PaperWidth = '58mm') {
    disconnectSerial()
    disconnectBluetooth()

    connectionType.value = 'VIRTUAL'
    printerName.value = name
    paperWidth.value = width
    isConnected.value = true
    statusMessage.value = 'Simulator aktif (Uji cetak tanpa hardware fisik)'
    saveConfig()
  }

  // Raw Write ESC/POS Binary Buffer
  async function writeRaw(buffer: Uint8Array): Promise<void> {
    if (connectionType.value === 'SERIAL') {
      if (!serialPort || !serialPort.writable) {
        throw new Error('Port serial belum terbuka atau terputus.')
      }
      const writer = serialPort.writable.getWriter()
      try {
        await writer.write(buffer)
      } finally {
        writer.releaseLock()
      }
    } else if (connectionType.value === 'BLUETOOTH') {
      if (!bluetoothCharacteristic) {
        throw new Error('Koneksi printer bluetooth tidak siap.')
      }
      // Kirim dalam potongan kecil (100 bytes) dengan micro-delay agar modul BLE printer mini tidak mengalami buffer overrun
      const chunkSize = 100
      for (let i = 0; i < buffer.length; i += chunkSize) {
        const chunk = buffer.slice(i, i + chunkSize)
        if (bluetoothCharacteristic.writeValueWithoutResponse) {
          await bluetoothCharacteristic.writeValueWithoutResponse(chunk)
        } else {
          await bluetoothCharacteristic.writeValue(chunk)
        }
        // Jeda 20ms antar paket data biner
        await new Promise((resolve) => setTimeout(resolve, 20))
      }
    } else {
      // VIRTUAL PRINTER: Simulate printing
      console.log('%c[NOTANE THERMAL PRINTER SIMULATOR]', 'color: #059669; font-weight: bold; font-size: 13px;')
      console.log(`Mencetak ${buffer.length} bytes data ESC/POS ke [${printerName.value}] (${paperWidth.value})`)
    }
  }

  // Fetch Receipt Settings from Database Backoffice
  async function fetchReceiptSettings() {
    try {
      isSettingsLoading.value = true
      const res = await api.get('/settings/receipt')
      if (res.data?.success && res.data?.data) {
        receiptSettings.value = res.data.data
        if (receiptSettings.value.paper_width) {
          paperWidth.value = receiptSettings.value.paper_width as PaperWidth
        }
      }
    } catch (e) {
      console.warn('Gagal memuat pengaturan struk dari database:', e)
    } finally {
      isSettingsLoading.value = false
    }
  }

  // Core Action: Print Receipt Directly Without Dialog
  async function printReceipt(receiptData: any): Promise<{ success: boolean; message: string }> {
    // If settings haven't been fetched yet, try fetching them
    if (!receiptSettings.value) {
      await fetchReceiptSettings()
    }

    const settings = receiptSettings.value || {}
    const options: EscPosOptions = {
      paperWidth: (settings.paper_width as PaperWidth) || paperWidth.value,
      autoCut: autoCut.value,
      openDrawer: openDrawer.value,
      storeName: settings.store_name || 'NOTANE RETAIL',
      storeTagline: settings.store_tagline || '',
      storeAddress: settings.store_address || 'Jl. Raya Retail Enterprise No. 88',
      storePhone: settings.store_phone || '0812-3456-7890',
      storeWebsite: settings.store_website || '',
      taxId: settings.tax_id || '',
      headerNote: settings.header_note || '',
      footerNote1: settings.footer_note_1 || 'Terima kasih atas kunjungan Anda!',
      footerNote2: settings.footer_note_2 || 'Barang yang dibeli tidak dapat ditukar',
      socialMedia: settings.social_media || '',
      showPhone: settings.show_phone !== false,
      showAddress: settings.show_address !== false,
      showCashierName: settings.show_cashier_name !== false,
      showBarcode: settings.show_barcode !== false,
      showTaxSummary: settings.show_tax_summary !== false,
    }

    // Build binary buffer
    const buffer = buildReceiptEscPos(receiptData, options)

    // Also build plain text for visual preview & simulator inspection
    const plainText = generatePlainTextReceipt(receiptData, options.paperWidth)
    lastPrintedText.value = plainText
    lastPrintTime.value = new Date().toLocaleTimeString('id-ID')

    // Send binary to hardware or simulator
    await writeRaw(buffer)

    statusMessage.value = `Struk berhasil dicetak ke ${printerName.value}`
    return {
      success: true,
      message: `Struk berhasil dicetak ke ${printerName.value} tanpa dialog browser.`,
    }
  }

  // Core Action: Test Print
  async function testPrint(): Promise<{ success: boolean; message: string }> {
    const options = {
      paperWidth: paperWidth.value,
      autoCut: autoCut.value,
      openDrawer: openDrawer.value,
      connectionType: connectionType.value,
      printerName: printerName.value,
    }

    const buffer = buildTestPrintEscPos(options)
    lastPrintTime.value = new Date().toLocaleTimeString('id-ID')
    lastPrintedText.value = `=== UJI CETAK NOTANE POS ===\nPrinter: ${printerName.value}\nTipe: ${connectionType.value}\nLebar: ${paperWidth.value}\nWaktu: ${lastPrintTime.value}\nSTATUS: BERHASIL TERHUBUNG`

    await writeRaw(buffer)

    statusMessage.value = `Uji cetak berhasil ke ${printerName.value}`
    return {
      success: true,
      message: `Uji cetak berhasil dikirim ke ${printerName.value}!`,
    }
  }

  // Initialize
  loadConfig()
  fetchReceiptSettings()

  return {
    // State
    connectionType,
    printerName,
    paperWidth,
    baudRate,
    autoPrint,
    autoCut,
    openDrawer,
    isConnected,
    isConnecting,
    statusMessage,
    lastPrintedText,
    lastPrintTime,
    isWebSerialSupported,
    isWebBluetoothSupported,
    receiptSettings,
    isSettingsLoading,

    // Actions
    loadConfig,
    saveConfig,
    fetchReceiptSettings,
    connectSerial,
    disconnectSerial,
    connectBluetooth,
    disconnectBluetooth,
    setVirtualPrinter,
    writeRaw,
    printReceipt,
    testPrint,
  }
})
