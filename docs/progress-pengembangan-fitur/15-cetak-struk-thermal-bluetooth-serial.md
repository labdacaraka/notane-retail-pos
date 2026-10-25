# Progress Log 15: Cetak Struk Thermal Otomatis (Direct Silent Printing) & Pengaturan Printer Bluetooth / Serial Port

**Tanggal**: 20 September 2026  
**Status**: Selesai (Completed & Verified)  
**Fitur**:
1. **Cetak Struk Otomatis Tanpa Dialog Browser (*Direct Silent Printing*)**:
   - Menghilangkan dialog pop-up cetak sistem/browser (`window.print()`) saat transaksi kasir selesai.
   - Perintah biner ESC/POS langsung dikirimkan ke printer thermal yang terhubung melalui Web Serial API, Web Bluetooth API, atau virtual simulator.
2. **Dukungan Dua Koneksi Perangkat Utama (Bluetooth & Serial Port / USB)**:
   - **Koneksi Serial Port / USB (Web Serial API)**: Untuk printer thermal kasir desktop berkabel (Epson TM-T82, Xprinter XP-58/XP-80, Panda POS, dll) dengan opsi Baud Rate (9600, 19200, 38400, 115200 bps).
   - **Koneksi Bluetooth (Web Bluetooth API)**: Untuk printer thermal nirkabel / portable (Panda PRJ-58B, VSC, Mini POS Bluetooth 58mm/80mm, ZJ-5802) via Bluetooth GATT ESC/POS SPP characteristics.
   - **Virtual Thermal Simulator**: Mode pengujian internal workstation tanpa perlu perangkat keras printer fisik terhubung.
3. **Modal Pengaturan Printer Thermal Kasir (`PrinterSettingsModal.vue`)**:
   - Pemilihan lebar kertas: **58 mm** (32 karakter/baris) vs **80 mm** (42-48 karakter/baris).
   - Sakelar otomasi: *Cetak Otomatis saat Transaksi Selesai*, *Buka Laci Kasir Otomatis (Cash Drawer Kick)*, dan *Potong Kertas Otomatis (Auto-Cut)*.
   - Tombol **Uji Cetak (Test Print)** dengan feedback status instan dan pratinjau slip monospace struk terakhir.
4. **Lencana Status Printer Interaktif di Header Kasir (`PosLayout.vue`)**:
   - Menampilkan status koneksi real-time: `🟢 Sim: Simulator Thermal ... (58mm)` atau `🟢 Printer: [Nama] (Serial/BT)`.
   - Mengklik lencana langsung membuka modal pengaturan printer.

---

## 1. Latar Belakang & Permintaan Pengguna

Pengguna meminta penyempurnaan alur pencetakan struk belanja di kasir:
> *"Pada bagian cetak struk harusnya otomatis memilih printer thermal yang sudah terhubung tidak muncul dialog. Harus ada settingan untuk memilih printer thermal baik menggunakan koneksi bluetooth atau serial port!"*

### Analisis Kebutuhan Bisnis Kasir:
1. **Efisiensi & Kecepatan Layanan (*Zero-Dialog Checkout*)**:
   - Penggunaan `window.print()` standar browser memunculkan dialog pratinjau sistem operasi yang mengharuskan kasir menekan tombol "Print" atau Enter sekali lagi.
   - Dalam lingkungan retail berkecepatan tinggi, printer thermal harus mencetak struk secara senyap (*silent direct printing*) seketika saat kasir menyelesaikan pembayaran.
2. **Fleksibilitas Perangkat Keras Kasir**:
   - Toko retail dapat menggunakan printer kasir desktop berkabel USB/Serial COM (seperti Epson TM-T82) atau printer portable Bluetooth yang hemat ruang. Sistem harus mendukung kedua protokol ini secara terintegrasi.

---

## 2. Rincian Implementasi Teknis

### A. Binary ESC/POS Encoder Engine (`src/utils/escpos.ts`)
- Menghasilkan byte biner `Uint8Array` berbasis protokol standar ESC/POS:
  - Inisialisasi printer (`ESC @` -> `0x1B, 0x40`).
  - Perataan teks: kiri (`ESC a 0`), tengah (`ESC a 1`), kanan (`ESC a 2`).
  - Format teks: bold (`ESC E 1`), double size (`GS ! 0x11`).
  - Feed baris (`ESC d n`).
  - Potong kertas otomatis (*Auto-Cut* `GS V 66 0` -> `0x1D, 0x56, 0x42, 0x00`).
  - Buka laci kasir (*Cash Drawer Kick* `ESC p 0 25 250` -> `0x1B, 0x70, 0x00, 0x19, 0xFA`).
  - Layout nota retail presisi 2 kolom sesuai format kertas 58mm (32 kolom) atau 80mm (42 kolom).

### B. Thermal Printer Driver & Pinia Store (`src/stores/printerStore.ts`)
- **Web Serial API**: Membuka port via `navigator.serial.requestPort()` dan menulis biner via `port.writable.getWriter().write(buffer)`.
- **Web Bluetooth API**: Menghubungkan perangkat via `navigator.bluetooth.requestDevice()` dengan service UUIDs standar ESC/POS (`000018f0`, `49535343`, `0000e0ff`, dll) dan menulis data chunk 512-byte ke GATT characteristic.
- **Direct Silent Print Execution**: Fungsi `printReceipt(receiptData)` merakit buffer ESC/POS dan menulis data langsung ke koneksi aktif tanpa membuka dialog browser.
- **Persistensi Konfigurasi**: Pengaturan disimpan otomatis di `localStorage` (`notane_printer_config`).

### C. Modal Pengaturan Printer (`src/components/pos/PrinterSettingsModal.vue`)
- Desain antarmuka desktop modular dengan icon colorful (Streamline Ultimate Color via Iconify):
  - Banner status koneksi aktif.
  - Tab koneksi: Serial Port / USB (dengan pilihan Baud Rate 9600 - 115200 bps), Bluetooth, dan Virtual Simulator.
  - Pilihan lebar kertas 58mm vs 80mm.
  - Sakelar cetak otomatis, buka laci kasir, dan potong kertas.
  - Tombol Uji Cetak (*Test Print*) dan panel pratinjau struk terakhir.

### D. Integrasi Header POS & Checkout Kasir
- **Header Kasir (`src/layouts/PosLayout.vue`)**:
  - Lencana interaktif di samping status jaringan LAN Host yang menampilkan status printer aktif dan membuka modal pengaturan saat diklik.
- **Pembayaran Kasir (`src/components/pos/QuickPayModal.vue`)**:
  - Saat pembayaran selesai (`checkout success`), jika `autoPrint` aktif dan printer terhubung, sistem otomatis mencetak nota via `printerStore.printReceipt(res.data.data)` tanpa dialog `window.print()`.
  - Notifikasi sukses langsung ditampilkan pada lencana nota: `✓ Struk berhasil dicetak ke [Nama Printer] tanpa dialog browser`.
  - Tombol "Cetak Ulang Struk" tersedia untuk mencetak salinan nota jika diperlukan.

---

## 3. Hasil Pengujian & Verifikasi

### A. TypeScript & Vite Build
```bash
npm run build
# vue-tsc -b && vite build
# Exit code: 0 (Passed, 4.53s)
```

### B. Pengujian End-to-End dengan Browser Subagent
- **Skenario 1: Lencana Status Printer di Header POS**:
  - Lencana printer muncul di header kasir dengan status: `Sim: Simulator Thermal ... (58mm)`.
- **Skenario 2: Modal Pengaturan Printer**:
  - Mengklik lencana membuka modal **"Pengaturan Printer Thermal Kasir"**.
  - Berpindah antar tab *Serial Port / USB*, *Bluetooth*, dan *Virtual Simulator* berjalan mulus.
- **Skenario 3: Uji Cetak (Test Print)**:
  - Tombol *Uji Cetak* diklik.
  - Notifikasi `Status: Uji cetak berhasil ke Simulator Thermal 58mm` muncul seketika tanpa popup dialog browser.
- **Skenario 4: Pembayaran & Silent Auto-Print Struk Kasir**:
  - Produk dimasukkan ke keranjang belanja kasir.
  - Tombol `BAYAR / F12` ditekan, nominal bayar diisi, dan kasir menekan `SELESAIKAN PEMBAYARAN`.
  - Transaksi berhasil, struk transaksi terbentuk, dan notifikasi cetak langsung muncul: `✓ Struk berhasil dicetak ke Simulator Thermal 58mm tanpa dialog browser`.
  - **100% Zero-Dialog**: Tidak ada dialog cetak sistem operasi atau browser yang menghalangi kasir.
