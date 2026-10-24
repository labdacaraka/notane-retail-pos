use crate::models::bank::{BankAccount, CreateBankAccountRequest, UpdateBankAccountRequest};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_bank_accounts(
    pool: &PgPool,
    outlet_id: Option<Uuid>,
) -> Result<Vec<BankAccount>, String> {
    sqlx::query_as::<_, BankAccount>(
        "SELECT 
            b.id,
            b.bank_name,
            b.account_number,
            b.account_holder,
            b.branch,
            b.outlet_id,
            o.name as outlet_name,
            b.coa_id,
            c.code as coa_code,
            c.name as coa_name,
            b.is_active,
            b.is_default,
            b.notes,
            b.created_at,
            b.updated_at
         FROM bank_accounts b
         LEFT JOIN outlets o ON b.outlet_id = o.id
         LEFT JOIN chart_of_accounts c ON b.coa_id = c.id
         WHERE ($1::uuid IS NULL OR b.outlet_id IS NULL OR b.outlet_id = $1)
         ORDER BY b.is_default DESC, b.bank_name ASC, b.created_at ASC",
    )
    .bind(outlet_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error saat mengambil daftar bank: {}", e))
}

pub async fn get_bank_account_by_id(pool: &PgPool, id: Uuid) -> Result<BankAccount, String> {
    sqlx::query_as::<_, BankAccount>(
        "SELECT 
            b.id,
            b.bank_name,
            b.account_number,
            b.account_holder,
            b.branch,
            b.outlet_id,
            o.name as outlet_name,
            b.coa_id,
            c.code as coa_code,
            c.name as coa_name,
            b.is_active,
            b.is_default,
            b.notes,
            b.created_at,
            b.updated_at
         FROM bank_accounts b
         LEFT JOIN outlets o ON b.outlet_id = o.id
         LEFT JOIN chart_of_accounts c ON b.coa_id = c.id
         WHERE b.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Rekening bank tidak ditemukan.".to_string())
}

pub async fn create_bank_account(
    pool: &PgPool,
    req: CreateBankAccountRequest,
) -> Result<BankAccount, String> {
    let bank_name = req.bank_name.trim().to_string();
    let account_number = req.account_number.trim().to_string();
    let account_holder = req.account_holder.trim().to_string();

    if bank_name.is_empty() {
        return Err("Nama bank tidak boleh kosong.".to_string());
    }
    if account_number.is_empty() {
        return Err("Nomor rekening tidak boleh kosong.".to_string());
    }
    if account_holder.is_empty() {
        return Err("Nama pemilik rekening (atas nama) tidak boleh kosong.".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi database: {}", e))?;

    // 1. Dapatkan akun induk Kas & Bank (1102)
    let parent_coa: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM chart_of_accounts WHERE code = '1102' LIMIT 1"
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    // 2. Buat akun CoA otomatis terintegrasi (dengan is_system = true agar aman terlindungi)
    let mut coa_id: Option<Uuid> = None;
    if let Some(parent_id) = parent_coa {
        let bank_slug = bank_name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        let short_bank = if bank_slug.len() > 6 {
            &bank_slug[..6]
        } else {
            &bank_slug
        };

        let last_4_acc = if account_number.len() > 4 {
            &account_number[account_number.len() - 4..]
        } else {
            &account_number
        };

        let mut candidate_code = format!("1102-{}-{}", short_bank, last_4_acc);
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM chart_of_accounts WHERE code = $1)",
        )
        .bind(&candidate_code)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(false);

        if exists {
            candidate_code = format!("{}-{}", candidate_code, &Uuid::new_v4().to_string()[..3].to_uppercase());
        }

        let new_coa_id = Uuid::new_v4();
        let coa_title = format!("Bank {} ({}) - {}", bank_name, account_number, account_holder);

        let coa_insert = sqlx::query(
            "INSERT INTO chart_of_accounts (id, account_category_id, parent_id, code, name, description, is_header, is_active, is_system, created_at, updated_at)
             VALUES ($1, 1, $2, $3, $4, $5, FALSE, TRUE, TRUE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
        )
        .bind(new_coa_id)
        .bind(parent_id)
        .bind(&candidate_code)
        .bind(&coa_title)
        .bind(format!("Rekening bank penerimaan {} No. {}", bank_name, account_number))
        .execute(&mut *tx)
        .await;

        match coa_insert {
            Ok(_) => {
                coa_id = Some(new_coa_id);
            }
            Err(e) => {
                return Err(format!("Gagal membuat akun CoA untuk rekening bank: {}", e));
            }
        }
    }

    let is_default = req.is_default.unwrap_or(false);
    if is_default {
        // Reset default rekening lain untuk outlet yang sama atau global
        sqlx::query("UPDATE bank_accounts SET is_default = FALSE")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
    }

    let new_bank_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO bank_accounts (
            id, bank_name, account_number, account_holder, branch, outlet_id, coa_id, is_active, is_default, notes, created_at, updated_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(new_bank_id)
    .bind(&bank_name)
    .bind(&account_number)
    .bind(&account_holder)
    .bind(&req.branch)
    .bind(req.outlet_id)
    .bind(coa_id)
    .bind(is_default)
    .bind(&req.notes)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan rekening bank: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Gagal mengonfirmasi transaksi: {}", e))?;

    get_bank_account_by_id(pool, new_bank_id).await
}

pub async fn update_bank_account(
    pool: &PgPool,
    id: Uuid,
    req: UpdateBankAccountRequest,
) -> Result<BankAccount, String> {
    let bank_name = req.bank_name.trim().to_string();
    let account_number = req.account_number.trim().to_string();
    let account_holder = req.account_holder.trim().to_string();

    if bank_name.is_empty() {
        return Err("Nama bank tidak boleh kosong.".to_string());
    }
    if account_number.is_empty() {
        return Err("Nomor rekening tidak boleh kosong.".to_string());
    }
    if account_holder.is_empty() {
        return Err("Nama pemilik rekening (atas nama) tidak boleh kosong.".to_string());
    }

    let existing = get_bank_account_by_id(pool, id).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi: {}", e))?;

    let is_default = req.is_default.unwrap_or(false);
    if is_default && !existing.is_default {
        sqlx::query("UPDATE bank_accounts SET is_default = FALSE WHERE id != $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
    }

    sqlx::query(
        "UPDATE bank_accounts 
         SET bank_name = $1, account_number = $2, account_holder = $3, branch = $4,
             outlet_id = $5, is_active = $6, is_default = $7, notes = $8, updated_at = CURRENT_TIMESTAMP
         WHERE id = $9",
    )
    .bind(&bank_name)
    .bind(&account_number)
    .bind(&account_holder)
    .bind(&req.branch)
    .bind(req.outlet_id)
    .bind(req.is_active)
    .bind(is_default)
    .bind(&req.notes)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memperbarui rekening bank: {}", e))?;

    // Sinkronkan nama dan status aktif ke akun CoA terkait jika ada
    if let Some(coa_id) = existing.coa_id {
        let coa_title = format!("Bank {} ({}) - {}", bank_name, account_number, account_holder);
        sqlx::query(
            "UPDATE chart_of_accounts 
             SET name = $1, is_active = $2, updated_at = CURRENT_TIMESTAMP 
             WHERE id = $3",
        )
        .bind(&coa_title)
        .bind(req.is_active)
        .bind(coa_id)
        .execute(&mut *tx)
        .await
        .ok();
    }

    tx.commit()
        .await
        .map_err(|e| format!("Gagal menyimpan perubahan: {}", e))?;

    get_bank_account_by_id(pool, id).await
}

pub async fn delete_bank_account(pool: &PgPool, id: Uuid) -> Result<(), String> {
    let existing = get_bank_account_by_id(pool, id).await?;

    // Cek apakah akun CoA terkait memiliki jurnal transaksi
    if let Some(coa_id) = existing.coa_id {
        let has_journals: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM journal_lines WHERE account_id = $1)",
        )
        .bind(coa_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if has_journals {
            return Err(
                "Rekening bank ini sudah memiliki riwayat pencatatan jurnal transaksi pembukuan \
                 dan tidak dapat dihapus. Anda dapat menonaktifkan status rekening ini sebagai gantinya."
                    .to_string(),
            );
        }

        // Cek apakah pernah digunakan dalam setoran modal
        let has_capital: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM capital_deposits WHERE payment_account_id = $1)",
        )
        .bind(coa_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if has_capital {
            return Err(
                "Rekening bank ini terhubung ke catatan transaksi Setoran Modal Usaha \
                 dan tidak dapat dihapus. Silakan nonaktifkan status rekening ini."
                    .to_string(),
            );
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi: {}", e))?;

    sqlx::query("DELETE FROM bank_accounts WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menghapus rekening bank: {}", e))?;

    // Hapus akun CoA terkait jika ada dan belum pernah terpakai
    if let Some(coa_id) = existing.coa_id {
        sqlx::query("DELETE FROM chart_of_accounts WHERE id = $1")
            .bind(coa_id)
            .execute(&mut *tx)
            .await
            .ok();
    }

    tx.commit()
        .await
        .map_err(|e| format!("Gagal menyelesaikan penghapusan: {}", e))?;

    Ok(())
}
