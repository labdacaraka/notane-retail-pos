import puppeteer from 'puppeteer-core';
import fs from 'node:fs';
import path from 'node:path';

const CHROME_PATH = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const OUT_DIR = path.resolve('docs/panduan/images');

if (!fs.existsSync(OUT_DIR)) {
  fs.mkdirSync(OUT_DIR, { recursive: true });
}

// Helper highlight function
async function highlightElements(page, annotations) {
  // annotations: array of { selector, text, num, label, color }
  await page.evaluate((items) => {
    // Remove previous badges
    document.querySelectorAll('.manual-guide-annotation').forEach(el => el.remove());

    items.forEach((item, idx) => {
      let targetEl = null;
      if (item.selector) {
        targetEl = document.querySelector(item.selector);
      } else if (item.text) {
        const allElements = Array.from(document.querySelectorAll(item.tag || 'button, a, input, div, span, h3, h2'));
        targetEl = allElements.find(el => el.innerText && el.innerText.trim().toLowerCase().includes(item.text.toLowerCase()));
      }

      if (!targetEl) return;

      const color = item.color || '#ef4444'; // Red-500
      targetEl.style.outline = `3px solid ${color}`;
      targetEl.style.outlineOffset = '3px';
      targetEl.style.borderRadius = item.radius || '8px';
      targetEl.style.boxShadow = `0 0 14px ${color}80`;

      if (getComputedStyle(targetEl).position === 'static') {
        targetEl.style.position = 'relative';
      }

      const num = item.num || (idx + 1);
      const badge = document.createElement('div');
      badge.className = 'manual-guide-annotation';
      badge.innerHTML = `<span>${num}</span>`;
      badge.style.position = 'absolute';
      badge.style.top = '-12px';
      badge.style.right = '-12px';
      badge.style.backgroundColor = color;
      badge.style.color = '#ffffff';
      badge.style.fontSize = '12px';
      badge.style.fontWeight = 'bold';
      badge.style.width = '24px';
      badge.style.height = '24px';
      badge.style.borderRadius = '50%';
      badge.style.display = 'flex';
      badge.style.alignItems = 'center';
      badge.style.justifyContent = 'center';
      badge.style.boxShadow = '0 2px 6px rgba(0,0,0,0.5)';
      badge.style.zIndex = '999999';
      badge.style.pointerEvents = 'none';

      targetEl.appendChild(badge);
    });
  }, annotations);
}

async function loginUser(page, username, password = 'admin123') {
  // First make API call directly to get token
  const res = await fetch("http://127.0.0.1:18080/api/v1/auth/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password, terminal_id: "KASIR-01" })
  });
  const data = await res.json();
  if (!data.success) {
    throw new Error(`Login failed for ${username}: ${data.message}`);
  }

  const token = data.data.token;
  const user = data.data.user;

  // Inject into page localStorage
  await page.evaluate((t, u) => {
    localStorage.setItem('notane_token', t);
    localStorage.setItem('notane_user', JSON.stringify(u));
    localStorage.setItem('notane_server_url', 'http://127.0.0.1:18080');
    localStorage.setItem('notane_pairing_code', 'NOTANE-TEST');
    localStorage.setItem('notane_terminal', 'KASIR-01');
  }, token, user);
}

(async () => {
  console.log('🚀 Menjalankan Chromium Headless untuk Screenshot Panduan...');
  const browser = await puppeteer.launch({
    executablePath: CHROME_PATH,
    headless: true,
    args: ['--no-sandbox', '--disable-gpu', '--window-size=1280,800']
  });

  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 800 });

  // =========================================================================
  // 1. NOTANE SERVER SCREENSHOTS
  // =========================================================================
  console.log('📸 Mengambil screenshot Notane Server...');
  await page.goto('http://localhost:5176', { waitUntil: 'networkidle0' });
  await page.waitForSelector('body');

  // Server 01: Dashboard
  await highlightElements(page, [
    { text: 'Mulai Ulang Server', num: 1 },
    { text: 'Hentikan Sementara', num: 2 },
    { text: 'Cadangan Data', num: 3 },
    { text: 'Pembaruan', num: 4 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'server_01_dashboard.png') });

  // Server 02: Backup Modal
  await page.evaluate(() => {
    const btn = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Cadangan Data'));
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 600));
  await highlightElements(page, [
    { text: 'Buat Cadangan Baru', num: 1 },
    { text: 'Unggah Cadangan', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'server_02_backup_modal.png') });
  await page.keyboard.press('Escape');
  await new Promise(r => setTimeout(r, 400));

  // Server 03: Pairing Code Modal
  await page.evaluate(() => {
    const btn = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Ganti'));
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 600));
  await highlightElements(page, [
    { selector: 'input', num: 1 },
    { text: 'Simpan', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'server_03_pairing_modal.png') });
  await page.keyboard.press('Escape');
  await new Promise(r => setTimeout(r, 400));

  // Server 04: Branding Setup
  await highlightElements(page, [
    { selector: 'input[placeholder*="Nama Toko"], input', num: 1 },
    { text: 'Simpan & Terapkan Branding Toko', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'server_04_branding_setup.png') });

  // Server 05: Update Modal
  await page.evaluate(() => {
    const btn = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Pembaruan'));
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 600));
  await highlightElements(page, [
    { text: 'Periksa Pembaruan', num: 1 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'server_05_update_modal.png') });
  await page.keyboard.press('Escape');
  await new Promise(r => setTimeout(r, 400));


  // =========================================================================
  // 2. NOTANE CLIENT - LOGIN SCREEN
  // =========================================================================
  console.log('📸 Mengambil screenshot Client: Login & Kasir...');
  await page.goto('http://localhost:5175', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));

  // Close any error modal if open
  await page.evaluate(() => {
    const btn = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Mengerti') || b.innerText.includes('Tutup'));
    if (btn) btn.click();
  });
  await new Promise(r => setTimeout(r, 500));

  await highlightElements(page, [
    { selector: 'input[placeholder*="terminal" i], select', num: 1 },
    { selector: 'input[placeholder*="username" i], input[type="text"]', num: 2 },
    { selector: 'input[type="password"]', num: 3 },
    { text: 'Masuk', num: 4 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'client_01_login.png') });


  // =========================================================================
  // 3. ROLE: CASHIER (kasir1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: CASHIER...');
  await loginUser(page, 'kasir1');
  await page.goto('http://localhost:5175/', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  // Cashier POS Layout
  await highlightElements(page, [
    { selector: 'input[placeholder*="scan" i], input[placeholder*="cari" i], input', num: 1 },
    { text: 'Bayar', num: 2 },
    { text: 'Tahan', num: 3 },
    { text: 'Panggil', num: 4 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'cashier_01_pos_layout.png') });

  // Cashier: Open/Close Shift
  await page.evaluate(() => {
    const shiftBtn = Array.from(document.querySelectorAll('button, a')).find(b => b.innerText && (b.innerText.includes('Shift') || b.innerText.includes('Tutup')));
    if (shiftBtn) shiftBtn.click();
  });
  await new Promise(r => setTimeout(r, 800));
  await highlightElements(page, [
    { selector: 'input[type="number"], input', num: 1 },
    { text: 'Tutup Shift', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'cashier_02_shift_settlement.png') });
  await page.keyboard.press('Escape');
  await new Promise(r => setTimeout(r, 500));

  // Cashier: Payment Dialog (add a product to cart first if possible)
  await page.evaluate(() => {
    // Click first product card if any
    const prodCard = document.querySelector('.cursor-pointer, button');
    if (prodCard) prodCard.click();
  });
  await new Promise(r => setTimeout(r, 500));
  await page.evaluate(() => {
    const payBtn = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Bayar') || b.innerText.includes('Checkout'));
    if (payBtn) payBtn.click();
  });
  await new Promise(r => setTimeout(r, 800));
  await highlightElements(page, [
    { text: 'Tunai', num: 1 },
    { text: 'QRIS', num: 2 },
    { text: 'EDC', num: 3 },
    { text: 'Bayar', num: 4 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'cashier_03_payment_modal.png') });
  await page.keyboard.press('Escape');
  await new Promise(r => setTimeout(r, 500));


  // =========================================================================
  // 4. ROLE: SUPERVISOR (spv)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: SUPERVISOR...');
  await loginUser(page, 'spv');
  await page.goto('http://localhost:5175/admin', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  // SPV Dashboard
  await highlightElements(page, [
    { text: 'Penjualan', num: 1 },
    { text: 'Stok', num: 2 },
    { text: 'Shift', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'spv_01_dashboard.png') });

  // SPV: Sales Returns & Approvals
  await page.goto('http://localhost:5175/admin/sales', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Retur', num: 1 },
    { text: 'Detail', num: 2 },
    { text: 'Ekspor', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'spv_02_sales_supervision.png') });

  // SPV: Cash Drops
  await page.goto('http://localhost:5175/admin/shifts', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Setor Kas', num: 1 },
    { text: 'Riwayat', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'spv_03_cash_drop.png') });


  // =========================================================================
  // 5. ROLE: STORE MANAGER (manager1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: STORE_MANAGER...');
  await loginUser(page, 'manager1');
  await page.goto('http://localhost:5175/admin/analytics', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Penjualan', num: 1 },
    { text: 'Margin', num: 2 },
    { text: 'Filter', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'mgr_01_analytics.png') });

  // Store Manager: Products & Outlet Prices
  await page.goto('http://localhost:5175/admin/products', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Tambah Produk', num: 1 },
    { text: 'Harga Khusus', num: 2 },
    { text: 'Barcode', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'mgr_02_product_prices.png') });

  // Store Manager: Accounting Period Closing
  await page.goto('http://localhost:5175/admin/accounting', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Tutup Periode', num: 1 },
    { text: 'Laba Rugi', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'mgr_03_period_closing.png') });


  // =========================================================================
  // 6. ROLE: WAREHOUSE (gudang1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: WAREHOUSE...');
  await loginUser(page, 'gudang1');
  await page.goto('http://localhost:5175/admin/inventory', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Transfer Stok', num: 1 },
    { text: 'Penyesuaian', num: 2 },
    { text: 'Kartu Stok', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'wh_01_inventory_stock.png') });

  // Warehouse: Price Labels & Barcodes
  await page.goto('http://localhost:5175/admin/price-labels', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Cetak', num: 1 },
    { text: 'Format', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'wh_02_barcode_labels.png') });


  // =========================================================================
  // 7. ROLE: PURCHASING (buyer1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: PURCHASING...');
  await loginUser(page, 'buyer1');
  await page.goto('http://localhost:5175/admin/purchases', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Buat Pesanan', num: 1 },
    { text: 'Penerimaan', num: 2 },
    { text: 'Faktur', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'purchasing_01_orders.png') });


  // =========================================================================
  // 8. ROLE: ACCOUNTING (akunting1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: ACCOUNTING...');
  await loginUser(page, 'akunting1');
  await page.goto('http://localhost:5175/admin/accounting', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Jurnal', num: 1 },
    { text: 'COA', num: 2 },
    { text: 'Buku Besar', num: 3 },
    { text: 'Neraca', num: 4 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'acc_01_journal_ledger.png') });

  // Accounting: Expenses & Income
  await page.goto('http://localhost:5175/admin/expenses', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Tambah Biaya', num: 1 },
    { text: 'Kategori', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'acc_02_expenses.png') });


  // =========================================================================
  // 9. ROLE: INTERNAL AUDIT (audit1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: INTERNAL_AUDIT...');
  await loginUser(page, 'audit1');
  await page.goto('http://localhost:5175/admin/audit-logs', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Filter', num: 1 },
    { text: 'Jejak Audit', num: 2 },
    { text: 'Aktivitas', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'audit_01_activity_logs.png') });


  // =========================================================================
  // 10. ROLE: MERCHANDISER (merch1)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: MERCHANDISER...');
  await loginUser(page, 'merch1');
  await page.goto('http://localhost:5175/admin/display-banners', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Unggah', num: 1 },
    { text: 'Banner', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'merch_01_display_banners.png') });

  // Merchandiser: Promotions & Vouchers
  await page.goto('http://localhost:5175/admin/promotions', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1000));
  await highlightElements(page, [
    { text: 'Tambah Promosi', num: 1 },
    { text: 'Flash Sale', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'merch_02_promotions.png') });


  // =========================================================================
  // 11. ROLE: SUPERADMIN (admin)
  // =========================================================================
  console.log('📸 Mengambil screenshot Role: SUPERADMIN...');
  await loginUser(page, 'admin');
  await page.goto('http://localhost:5175/admin/users', { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 1200));

  await highlightElements(page, [
    { text: 'Tambah Pengguna', num: 1 },
    { text: 'Peran', num: 2 },
    { text: 'Hak Akses', num: 3 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'super_01_users_management.png') });

  // Superadmin: Role Permissions Matrix
  await page.evaluate(() => {
    const roleTab = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Hak Akses') || b.innerText.includes('Peran'));
    if (roleTab) roleTab.click();
  });
  await new Promise(r => setTimeout(r, 800));
  await highlightElements(page, [
    { text: 'SUPERADMIN', num: 1 },
    { text: 'Simpan', num: 2 },
  ]);
  await page.screenshot({ path: path.join(OUT_DIR, 'super_02_matrix.png') });

  console.log('🎉 Semua screenshot panduan berhasil diambil dan ditandai!');
  await browser.close();
})();
