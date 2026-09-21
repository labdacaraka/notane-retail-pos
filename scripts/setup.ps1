# Skrip Otomasi Setup Lingkungan Notane Retail POS (Windows)
$ErrorActionPreference = "Continue"

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "   NOTANE RETAIL POS - AUTOMATED SETUP            " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

# 1. Konfigurasi Environment File (.env)
Write-Host "`n[1/3] Memeriksa konfigurasi .env..." -ForegroundColor Yellow
if (-not (Test-Path ".env")) {
    if (Test-Path ".env.example") {
        Copy-Item ".env.example" ".env"
        Write-Host "  [OK] Berhasil membuat .env dari .env.example" -ForegroundColor Green
    } else {
        Write-Host "  [!] File .env.example tidak ditemukan!" -ForegroundColor Red
    }
} else {
    Write-Host "  [OK] File .env sudah tersedia." -ForegroundColor Green
}

# 2. Instalasi Dependensi Frontend
Write-Host "`n[2/3] Memeriksa dependensi NPM..." -ForegroundColor Yellow
if (-not (Test-Path "node_modules")) {
    Write-Host "  [*] Menjalankan npm install..." -ForegroundColor Cyan
    npm install
    Write-Host "  [OK] Dependensi NPM berhasil dipasang." -ForegroundColor Green
} else {
    Write-Host "  [OK] Folder node_modules sudah tersedia." -ForegroundColor Green
}

# 3. Memeriksa Ketersediaan Database PostgreSQL
Write-Host "`n[3/3] Memeriksa koneksi database PostgreSQL (Port 5432)..." -ForegroundColor Yellow

$portOpen = $false
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $asyncResult = $tcp.BeginConnect("127.0.0.1", 5432, $null, $null)
    $success = $asyncResult.AsyncWaitHandle.WaitOne(1500)
    if ($success -and $tcp.Connected) {
        $tcp.EndConnect($asyncResult)
        $portOpen = $true
    }
    $tcp.Close()
} catch {
    $portOpen = $false
}

if ($portOpen) {
    Write-Host "  [OK] PostgreSQL aktif dan terdeteksi di port 5432!" -ForegroundColor Green
    
    # Coba buat database notane_pos jika psql tersedia
    $psqlCmd = Get-Command psql -ErrorAction SilentlyContinue
    if (-not $psqlCmd) {
        $stdPsql = "C:\Program Files\PostgreSQL\16\bin\psql.exe"
        if (Test-Path $stdPsql) {
            $psqlCmd = $stdPsql
        }
    }
    
    if ($psqlCmd) {
        try {
            $env:PGPASSWORD = "postgres"
            & $psqlCmd -U postgres -h 127.0.0.1 -p 5432 -c "CREATE DATABASE notane_pos;" 2>&1 | Out-Null
            Write-Host "  [OK] Database 'notane_pos' telah dipastikan tersedia." -ForegroundColor Green
        } catch {
            Write-Host "  [*] Database 'notane_pos' sudah siap digunakan." -ForegroundColor Gray
        }
    }
} else {
    Write-Host "  [!] Port 5432 belum aktif. Memeriksa ketersediaan Docker..." -ForegroundColor Yellow
    $dockerCmd = Get-Command docker -ErrorAction SilentlyContinue
    if ($dockerCmd) {
        Write-Host "  [*] Docker terdeteksi! Menjalankan database via Docker Compose..." -ForegroundColor Cyan
        docker compose up -d
        Write-Host "  [OK] Container PostgreSQL telah dijalankan via Docker Compose di port 5432." -ForegroundColor Green
    } else {
        Write-Host "  [!] PostgreSQL lokal belum berjalan dan Docker belum terpasang." -ForegroundColor Red
        Write-Host "  [*] Silakan jalankan Docker Desktop atau instal PostgreSQL 16." -ForegroundColor Gray
    }
}

Write-Host "`n==================================================" -ForegroundColor Cyan
Write-Host "  SETUP SELESAI!" -ForegroundColor Green
Write-Host "  - Menjalankan Web:    npm run dev" -ForegroundColor White
Write-Host "  - Menjalankan Tauri:  npm run tauri dev" -ForegroundColor White
Write-Host "==================================================" -ForegroundColor Cyan
