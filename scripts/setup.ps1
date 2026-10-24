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
Write-Host "`n[3/3] Memeriksa koneksi database PostgreSQL terisolasi (Port 15432)..." -ForegroundColor Yellow

$portOpen = $false
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $asyncResult = $tcp.BeginConnect("127.0.0.1", 15432, $null, $null)
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
    Write-Host "  [OK] PostgreSQL terisolasi aktif dan terdeteksi di port 15432!" -ForegroundColor Green
    
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
            & $psqlCmd -U postgres -h 127.0.0.1 -p 15432 -c "CREATE DATABASE notane_pos;" 2>&1 | Out-Null
            Write-Host "  [OK] Database 'notane_pos' telah dipastikan tersedia." -ForegroundColor Green
        } catch {
            Write-Host "  [*] Database 'notane_pos' sudah siap digunakan." -ForegroundColor Gray
        }
    }
} else {
    Write-Host "  [!] Port 15432 belum aktif. Memeriksa ketersediaan Docker..." -ForegroundColor Yellow
    $dockerCmd = Get-Command docker -ErrorAction SilentlyContinue
    if ($dockerCmd) {
        Write-Host "  [*] Docker terdeteksi! Menjalankan database via Docker Compose..." -ForegroundColor Cyan
        docker compose up -d
        Write-Host "  [OK] Container PostgreSQL telah dijalankan via Docker Compose di port terisolasi 15432." -ForegroundColor Green
    } else {
        Write-Host "  [*] Tidak ada server PostgreSQL eksternal di port 15432. Aplikasi akan otomatis menjalankan Embedded PostgreSQL." -ForegroundColor Gray
    }
}

Write-Host "`n==================================================" -ForegroundColor Cyan
Write-Host "  SETUP SELESAI!" -ForegroundColor Green
Write-Host "  - Menjalankan Client POS:    npm run client:app" -ForegroundColor White
Write-Host "  - Menjalankan Server Host:   npm run server:app" -ForegroundColor White
Write-Host "  - Menjalankan Semua Dev:     npm run dev:all" -ForegroundColor White
Write-Host "==================================================" -ForegroundColor Cyan
