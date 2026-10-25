# Dokumentasi Fitur: Audio Feedback & Sound Synthesizer POS Kasir

## 1. Latar Belakang & Kebutuhan
Dalam operasional kasir retail bertempo cepat (*fast-paced checkout*), kasir sangat membutuhkan konfirmasi pendengaran (*audio cue*) instan tanpa harus selalu mengalihkan pandangan penuh ke monitor:
1. Peringatan suara tegas ketika barang yang dipindai/dipilih ternyata **stoknya habis** atau kuantitas telah **mencapai batas maksimal stok**.
2. Suara 'pip' reaktif (*scanner beep*) saat produk berhasil dipindai dan masuk ke keranjang.
3. Suara akord melodi (*payment success chime*) saat pembayaran lunas dan transaksi selesai.
4. Suara peringatan rendah (*error buzz*) jika barcode tidak ditemukan.
5. Suara klik/turun (*blip*) saat item dihapus atau keranjang dikosongkan.
6. Suara penahanan/pemanggilan antrean (*hold/recall*).
7. Tombol kendali suara (*Sound Toggle ON / Mute*) pada navigasi atas kasir.

---

## 2. Arsitektur & Implementasi Teknis

### A. Web Audio API Synthesizer ([`src/services/soundService.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/services/soundService.ts))
Menggunakan Web Audio API bawaan browser/WebView:
- **Zero External Audio Files**: Tidak memerlukan file mp3/wav eksternal, sehingga tidak pernah gagal karena 404, latensi jaringan, atau kendala path lokal di Tauri vs Web.
- **Zero Latency (< 1ms)**: Suara disintesis secara real-time menggunakan osilator (*OscillatorNode*) dan selubung amplitudo (*GainNode*).
- **Persistent Settings**: Preferensi Mute dan Volume disimpan di `localStorage` (`notane_pos_sound_enabled` & `notane_pos_sound_volume`).

### Variasi Suara yang Disediakan:
1. **`playOutOfStock()` (Peringatan Stok Habis / Maksimal Stok)**:
   - Pola: *Dual-tone alert* (frekuensi 340 Hz meluncur ke 220 Hz, diikuti nada buzz sekunder 260 Hz ke 160 Hz pada gelombang *sawtooth*).
   - Memberikan sinyal audio peringatan yang tegas namun nyaman di telinga kasir.
2. **`playScanSuccess()` (Pemindaian Barcode Berhasil)**:
   - Pola: Nada pip tajam 1850 Hz -> 1920 Hz (*sine wave*) selama 70ms standar scanner retail modern.
3. **`playError()` (Error / Barang Tidak Ditemukan)**:
   - Pola: *Double-buzz* frekuensi 180 Hz (*square wave*).
4. **`playPaymentSuccess()` (Pembayaran Sukses / Checkout Selesai)**:
   - Pola: Akord arpeggio C5 -> E5 -> G5 -> C6 (*triangle wave*) yang merdu dengan decay lembut.
5. **`playRemoveItem()` (Hapus Barang / Kosongkan Keranjang)**:
   - Pola: Nada frekuensi turun halus 440 Hz -> 220 Hz.
6. **`playHoldRecall()` (Tahan / Panggil Antrean)**:
   - Pola: Nada *ping* frekuensi naik 600 Hz -> 800 Hz.

---

## 3. Titik Integrasi Antarmuka

1. **State Store Keranjang ([`src/stores/cartStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/stores/cartStore.ts))**:
   - `addProductToCart` & `addProduct`: Memanggil `playOutOfStock()` saat stok habis atau melebihi stok, dan `playScanSuccess()` saat item berhasil ditambahkan.
   - `updateQuantity`: Memanggil `playOutOfStock()` saat kuantitas mencoba melebihi batas stok.
   - `removeItem` & `clearCart`: Memanggil `playRemoveItem()`.
   - `holdCurrentOrder` & `recallOrder`: Memanggil `playHoldRecall()`.
2. **Tabel Keranjang POS ([`src/components/pos/CartTable.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/CartTable.vue))**:
   - Menekan tombol `+` saat kuantitas sudah di batas stok memicu `playOutOfStock()`.
   - Mengetik angka melebihi batas stok memicu `playOutOfStock()`.
3. **Bilah Scanner & Autocomplete ([`src/components/pos/BarcodeBar.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/BarcodeBar.vue))**:
   - Memilih produk berstok 0 memicu `playOutOfStock()`.
   - Barcode tidak ditemukan memicu `playError()`.
4. **Katalog Modal Pencarian ([`src/components/pos/ProductSearchModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/ProductSearchModal.vue))**:
   - Mengklik produk habis memicu `playOutOfStock()`.
5. **Modal Pembayaran ([`src/components/pos/QuickPayModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/QuickPayModal.vue))**:
   - Transaksi sukses memicu `playPaymentSuccess()`.
   - Transaksi gagal memicu `playError()`.
6. **Header Navigasi POS ([`src/layouts/PosLayout.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/layouts/PosLayout.vue))**:
   - Menambahkan tombol interaktif **`Suara ON / Mute`** di bagian atas kanan. Kasir dapat mematikan atau mengaktifkan suara kapan saja dengan satu klik.

---

## 4. Hasil Verifikasi
1. **Build Verifikasi**: `npm run build` sukses (1762 modul tertransformasi tanpa peringatan maupun error).
2. **Pengujian Browser E2E**:
   - Tombol toggle suara berpindah antara `Suara ON` dan `Mute` dengan feedback pesan toast instan.
   - Memasukkan kode barcode tidak valid memicu peringatan visual dan efek audio error.
