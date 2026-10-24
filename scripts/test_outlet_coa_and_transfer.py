import urllib.request
import json
import uuid

BASE_URL = "http://127.0.0.1:18080/api/v1"

def req(method, path, body=None, token=None):
    url = BASE_URL + path
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(body).encode('utf-8') if body else None
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request) as resp:
            return json.loads(resp.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        err_msg = e.read().decode('utf-8')
        try:
            return json.loads(err_msg)
        except:
            return {"error": err_msg, "status": e.code}

# 1. Login as admin
login_res = req("POST", "/auth/login", {"username": "admin", "password": "admin123"})
assert login_res.get("success"), f"Login failed: {login_res}"
token = login_res["data"]["token"]
print("1. Login berhasil. Token didapatkan.")

# 2. Get Outlets
outlets_res = req("GET", "/outlets", token=token)
assert outlets_res.get("success"), f"Get outlets failed: {outlets_res}"
outlets = outlets_res["data"]
print(f"2. Memeriksa {len(outlets)} outlet/gudang:")
for o in outlets:
    print(f"   - {o['code']}: {o['name']} -> CoA: {o.get('inventory_account_code')} ({o.get('inventory_account_name')})")
    assert o.get("inventory_account_id"), f"Outlet {o['code']} tidak memiliki inventory_account_id"

# 3. Get Chart of Accounts
coa_res = req("GET", "/accounting/chart-of-accounts", token=token)
assert coa_res.get("success"), f"Get COA failed: {coa_res}"
coa_list = coa_res["data"]
inv_coas = [c for c in coa_list if c["code"].startswith("1201")]
print(f"\n3. Memeriksa akun CoA Persediaan ({len(inv_coas)} akun):")
for c in inv_coas:
    print(f"   - {c['code']} | {c['name']} | Header: {c['is_header']} | Outlet: {c.get('outlet_name')}")

# 4. Ambil 2 outlet untuk pengujian transfer stok
from_outlet = outlets[0]
to_outlet = outlets[1]
print(f"\n4. Uji Transfer Stok dari '{from_outlet['name']}' ke '{to_outlet['name']}'...")

# Ambil produk yang ada stoknya di from_outlet
products_res = req("GET", f"/products?outlet_id={from_outlet['id']}&page_size=10", token=token)
prods = products_res.get("data", [])
assert len(prods) > 0, "Tidak ada produk"
target_prod = None
for p in prods:
    if float(p.get("stock_qty", 0)) >= 2:
        target_prod = p
        break

if not target_prod:
    # Buat GRN dulu ke from_outlet agar ada stok
    print("   Stok di from_outlet kurang, membuat penerimaan GRN terlebih dahulu...")
    suppliers_res = req("GET", "/suppliers", token=token)
    supp_id = suppliers_res["data"][0]["id"]
    p_id = prods[0]["id"]
    grn_payload = {
        "supplier_id": supp_id,
        "outlet_id": from_outlet["id"],
        "warehouse_id": from_outlet["id"],
        "supplier_invoice_number": f"SUPP-SEED-{uuid.uuid4().hex[:6].upper()}",
        "invoice_date": "2026-09-26",
        "due_date": "2026-10-26",
        "payment_status": "UNPAID",
        "items": [{
            "product_id": p_id,
            "received_qty": 20,
            "unit_price": 50000
        }]
    }
    grn_res = req("POST", "/inventory/grn", grn_payload, token=token)
    assert grn_res.get("success"), f"GRN failed: {grn_res}"
    print("   GRN berhasil dicatat.")
    target_prod = prods[0]

# Catat stok awal
from_stock_before = float(req("GET", f"/products?outlet_id={from_outlet['id']}&search={target_prod['code']}", token=token)["data"][0]["stock_qty"])
to_stock_before = float(req("GET", f"/products?outlet_id={to_outlet['id']}&search={target_prod['code']}", token=token)["data"][0]["stock_qty"])
print(f"   Stok awal '{target_prod['name']}': Di Asal = {from_stock_before}, Di Tujuan = {to_stock_before}")

# Kirim transfer 2 unit
transfer_payload = {
    "from_outlet_id": from_outlet["id"],
    "to_outlet_id": to_outlet["id"],
    "transfer_date": "2026-09-26",
    "notes": "Pengujian otomatis mutasi transfer stok antar outlet",
    "items": [{
        "product_id": target_prod["id"],
        "quantity": 2
    }]
}
transfer_res = req("POST", "/inventory/transfers", transfer_payload, token=token)
print(f"   Hasil transfer: {transfer_res}")
assert transfer_res.get("success"), f"Transfer failed: {transfer_res}"
trf_id = transfer_res["data"]["id"]

# Cek stok sesudah transfer
from_stock_after = float(req("GET", f"/products?outlet_id={from_outlet['id']}&search={target_prod['code']}", token=token)["data"][0]["stock_qty"])
to_stock_after = float(req("GET", f"/products?outlet_id={to_outlet['id']}&search={target_prod['code']}", token=token)["data"][0]["stock_qty"])
print(f"   Stok akhir: Di Asal = {from_stock_after} (turun {from_stock_before - from_stock_after}), Di Tujuan = {to_stock_after} (naik {to_stock_after - to_stock_before})")
assert from_stock_after == from_stock_before - 2, "Stok outlet asal tidak berkurang 2"
assert to_stock_after == to_stock_before + 2, "Stok outlet tujuan tidak bertambah 2"

# 5. Cek Rincian Transfer dan Jurnal Pembukuan Otomatis
trf_detail = req("GET", f"/inventory/transfers/{trf_id}", token=token)
assert trf_detail.get("success"), f"Get transfer detail failed: {trf_detail}"
trf_data = trf_detail["data"]
print(f"\n5. Rincian Transfer {trf_data['transfer_number']}:")
print(f"   - Dari: {trf_data['from_outlet_name']} ({trf_data['from_outlet_code']})")
print(f"   - Ke: {trf_data['to_outlet_name']} ({trf_data['to_outlet_code']})")
print(f"   - Total Nilai HPP: Rp {float(trf_data['total_amount']):,.2f}")
print(f"   - Jurnal Terbentuk: {trf_data.get('journal', {}).get('journal_number')}")
if trf_data.get("journal"):
    for l in trf_data["journal"]["lines"]:
        print(f"     * [{l['account_code']}] {l['account_name']}: Debit={l['debit']}, Kredit={l['credit']} | {l['memo']}")
    assert float(trf_data["journal"]["total_debit"]) == float(trf_data["journal"]["total_credit"]), "Jurnal transfer tidak seimbang!"

# 6. Uji Buat Outlet Baru -> Otomatis Buat Akun CoA
new_code = f"TEST-{uuid.uuid4().hex[:4].upper()}"
new_name = f"Outlet Uji Coba {new_code}"
create_outlet_payload = {
    "code": new_code,
    "name": new_name,
    "address": "Jl. Pengujian Otomatis No. 1",
    "is_active": True,
    "is_warehouse": False
}
print(f"\n6. Uji Buat Outlet Baru: {new_code} ({new_name})...")
new_outlet_res = req("POST", "/outlets", create_outlet_payload, token=token)
assert new_outlet_res.get("success"), f"Create outlet failed: {new_outlet_res}"
new_outlet_id = new_outlet_res["data"]["id"]

# Cek outlet yang baru dibuat apakah memiliki inventory_account_id
created_outlet = req("GET", f"/outlets/{new_outlet_id}", token=token)["data"]
print(f"   Outlet baru berhasil dibuat dengan inventory_account_code: {created_outlet.get('inventory_account_code')}")
assert created_outlet.get("inventory_account_id"), "Outlet baru tidak memiliki inventory_account_id!"
assert created_outlet.get("inventory_account_code") == f"1201-{new_code}", "Kode CoA tidak sesuai format!"

# 7. Uji Hapus Outlet Baru (tanpa transaksi) -> Akun CoA otomatis terhapus
print(f"\n7. Uji Hapus Outlet Baru (belum ada transaksi)...")
del_res = req("DELETE", f"/outlets/{new_outlet_id}", token=token)
print(f"   Hasil hapus: {del_res}")
assert del_res.get("success"), f"Delete outlet failed: {del_res}"

# Pastikan akun CoA-nya juga terhapus
coa_after_del = req("GET", "/accounting/chart-of-accounts", token=token)["data"]
assert not any(c["code"] == f"1201-{new_code}" for c in coa_after_del), "Akun CoA outlet tidak terhapus saat outlet dihapus!"
print("   Akun CoA outlet berhasil terhapus secara otomatis dan bersih!")

# 8. Uji Pencegahan Hapus Outlet yang memiliki riwayat transaksi/jurnal
print(f"\n8. Uji Pencegahan Hapus Outlet yang memiliki transaksi ({from_outlet['name']})...")
del_used_res = req("DELETE", f"/outlets/{from_outlet['id']}", token=token)
print(f"   Hasil (harus ditolak): {del_used_res}")
assert not del_used_res.get("success"), "Outlet dengan transaksi seharusnya tidak boleh dihapus!"
print("   Pencegahan penghapusan outlet dengan transaksi/jurnal berhasil diverifikasi!")

print("\n=== SEMUA PENGUJIAN 100% SUKSES DAN TERVERIFIKASI ===")
