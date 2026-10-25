# Progress Log 03: Frontend Kasir POS (Front-Office) & Keyboard-Driven Interface

- **Tanggal Penyelesaian:** 2026-09-20
- **Status:** Selesai (Completed)
- **Komponen:** Vue 3, Pinia Store, Tailwind CSS, Layout POS, Keyboard Shortcuts

---

## 1. Ringkasan Antarmuka Kasir POS
Antarmuka kasir dirancang dengan prinsip efisiensi milidetik dan operasional padat (*high-density*), meminimalkan interaksi kursor tetikus (*mouse*) dan memaksimalkan *hotkeys keyboard*:

1. **Top Bar Informasi Terminal & Shift:**
   - Menampilkan status shift kasir aktif, modal awal kasir, waktu lokal detik berjalan, dan indikator koneksi jaringan LAN Host.
   - Tombol cepat pembukaan dan penutupan shift kasir (*blind settlement*).
   - Akses cepat ke Layar Hadap Pelanggan (*Customer Display*) dan Dasbor Back-Office.

2. **Input Barcode Autofocus & Hotkey Toolbar (`components/pos/BarcodeBar.vue`):**
   - Mendukung scanner hardware langsung (Enter otomatis).
   - Mendukung *Weight-Embedded Barcode* (timbangan elektrik buah/sayur).
   - Navigasi tombol fungsi:
     - `F1`: Pencarian Produk Manual
     - `F4`: Buka Cash Drawer Manual
     - `F6`: Tahan Transaksi (Hold)
     - `F7`: Panggil Transaksi (Recall)
     - `F12`: Bayar (Quick Pay / Checkout)
     - `Esc`: Batal / Tutup Dialog

3. **Tabel Keranjang Belanja Interaktif (`components/pos/CartTable.vue`):**
   - Menampilkan rincian produk, nomor baris, SKU, barcode, dan badge khusus produk timbangan.
   - Pilihan satuan bertingkat (*multi-unit switcher*): mengubah antara PCS, BOX, atau DUS secara langsung.
   - Pengubah kuantitas interaktif (`-` dan `+`) serta input numerik langsung.
   - Kolom diskon item dan pajak PPN.
   - Pembatalan baris (*Void Item*) dengan otorisasi supervisor.

4. **Otorisasi Supervisor Langsung / In-Place PIN (`components/pos/SupervisorPinModal.vue`):**
   - Popup verifikasi PIN supervisor langsung di layar kasir untuk tindakan pembatalan nota/item (*Void*), diskon manual, atau buka laci.
   - Tidak memaksa kasir logout, menjaga kecepatan antrean kasir.

5. **Split Payment & Thermal Receipt Modal (`components/pos/QuickPayModal.vue`):**
   - Mendukung pembayaran campuran (Tunai + QRIS / Kartu EDC / Poin Member).
   - Tombol cepat uang pecahan (Uang Pas, +10k, +20k, +50k, +100k).
   - Kalkulasi kembalian tunai secara real-time.
   - Generator dan preview struk kasir thermal 80mm standar ritel (dapat dicetak via browser maupun printer POS).

6. **Fitur Tahan & Panggil Transaksi (`components/pos/HoldRecallModal.vue`):**
   - Memungkinkan kasir menunda keranjang belanja konsumen yang tertinggal dompet atau mengambil barang tambahan, lalu melayani antrean berikutnya tanpa membatalkan barang pertama.

7. **Layar Hadap Pelanggan / Customer Display (`pages/CustomerDisplay.vue`):**
   - Dapat dibuka pada layar monitor kedua (*dual-screen/pole display*) untuk menampilkan rincian belanjaan dan total harga kepada pelanggan secara real-time.
