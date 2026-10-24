<#
==============================================================================
 NOTANE RETAIL SERVER - BRAND MANAGER CLI (WINDOWS VPS & POWERSHELL)
 Mempermudah deploy dan manajemen banyak brand toko dalam 1 Windows VPS
==============================================================================
#>

[CmdletBinding()]
param(
    [Parameter(Position=0)]
    [string]$Command = "help",

    [Parameter(Position=1)]
    [string]$Slug = "",

    [Parameter(Position=2)]
    [string]$BrandName = "",

    [Parameter(Position=3)]
    [int]$Port = 0,

    [Parameter(Position=4)]
    [int]$DbPort = 0
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
$BrandsDir = Join-Path $RootDir "brands"

if (-not (Test-Path $BrandsDir)) {
    New-Item -ItemType Directory -Path $BrandsDir -Force | Out-Null
}

function Show-Help {
    Write-Host "==================================================================" -ForegroundColor Cyan
    Write-Host "   NOTANE RETAIL SERVER - MULTI-BRAND MANAGER (WINDOWS VPS)" -ForegroundColor Green
    Write-Host "==================================================================" -ForegroundColor Cyan
    Write-Host "Penggunaan: .\scripts\brand-manager.ps1 [COMMAND] [ARGUMEN...]"
    Write-Host ""
    Write-Host "Perintah:"
    Write-Host "  create <slug> <nama_brand> <port_server> [port_db]"
    Write-Host "         Membuat konfigurasi baru untuk brand toko."
    Write-Host "         Contoh: .\scripts\brand-manager.ps1 create kopisenja `"Kopi Senja`" 18080 15432"
    Write-Host ""
    Write-Host "  start <slug>"
    Write-Host "         Menjalankan server & database untuk brand tersebut."
    Write-Host ""
    Write-Host "  stop <slug>"
    Write-Host "         Menghentikan server & database untuk brand tersebut."
    Write-Host ""
    Write-Host "  restart <slug>"
    Write-Host "         Me-restart container untuk brand tersebut."
    Write-Host ""
    Write-Host "  logs <slug>"
    Write-Host "         Melihat live logs server brand."
    Write-Host ""
    Write-Host "  list"
    Write-Host "         Menampilkan daftar semua brand yang terkonfigurasi & statusnya."
    Write-Host "==================================================================" -ForegroundColor Cyan
}

function Assert-Docker {
    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Host "[!] Perintah 'docker' tidak ditemukan di Windows VPS ini!" -ForegroundColor Red
        Write-Host "    Layanan server brand dijalankan menggunakan Docker Container." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "    Panduan Instalasi Docker di Windows:" -ForegroundColor Cyan
        Write-Host "    - Pasang Docker Desktop atau Docker Engine for Windows." -ForegroundColor White
        Write-Host "    - https://docs.docker.com/desktop/install/windows-install/" -ForegroundColor DarkGray
        exit 1
    }
}

switch ($Command.ToLower()) {
    "create" {
        if ([string]::IsNullOrWhiteSpace($Slug) -or [string]::IsNullOrWhiteSpace($BrandName) -or $Port -eq 0) {
            Write-Host "[!] Parameter tidak lengkap." -ForegroundColor Red
            Write-Host "Format: .\scripts\brand-manager.ps1 create <slug> <nama_brand> <port_server> [port_db]" -ForegroundColor Yellow
            exit 1
        }

        if ($DbPort -eq 0) {
            $DbPort = $Port - 2648
        }
        $ControlPort = $Port + 1
        $EnvFile = Join-Path $BrandsDir "$Slug.env"

        if (Test-Path $EnvFile) {
            Write-Host "[!] Brand '$Slug' sudah ada di $EnvFile." -ForegroundColor Red
            exit 1
        }

        # Generate secure random pairing code & JWT secret
        $randSuffix = (Get-Random -Minimum 1000 -Maximum 9999).ToString()
        $pairingCode = "NOTANE-$randSuffix"
        $jwtSecret = [System.Convert]::ToBase64String([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(24)) -replace '[^a-zA-Z0-9]', 'x'
        $dbPass = [System.Convert]::ToBase64String([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(12)) -replace '[^a-zA-Z0-9]', 'z'

        $EnvContent = @"
# Konfigurasi Brand Toko: $BrandName
COMPOSE_PROJECT_NAME=notane-$Slug
BRAND_NAME=$BrandName
SERVER_PORT=$Port
CONTROL_PORT=$ControlPort
POSTGRES_PORT=$DbPort
POSTGRES_DB=notane_$Slug
POSTGRES_USER=admin_$Slug
POSTGRES_PASSWORD=$dbPass
PAIRING_CODE=$pairingCode
PROVIDER_SERVER_URL=http://host.docker.internal:19090
JWT_SECRET=$jwtSecret
RUST_LOG=notane_retail_server=info,tower_http=info
"@

        Set-Content -Path $EnvFile -Value $EnvContent -Encoding UTF8

        Write-Host "[OK] Berhasil membuat konfigurasi brand '$BrandName'!" -ForegroundColor Green
        Write-Host "  - File Konfigurasi: brands\$Slug.env" -ForegroundColor White
        Write-Host "  - Port Web/API:     $Port" -ForegroundColor White
        Write-Host "  - Port Kontrol:     $ControlPort" -ForegroundColor White
        Write-Host "  - Port Database:    $DbPort" -ForegroundColor White
        Write-Host "  - Kode Pairing:     $pairingCode" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Untuk menyalakan brand ini, jalankan:" -ForegroundColor Cyan
        Write-Host "  .\scripts\brand-manager.ps1 start $Slug" -ForegroundColor White
    }

    "start" {
        Assert-Docker
        if ([string]::IsNullOrWhiteSpace($Slug)) {
            Write-Host "[!] Masukkan slug brand. Contoh: .\scripts\brand-manager.ps1 start kopisenja" -ForegroundColor Red
            exit 1
        }
        $EnvFile = Join-Path $BrandsDir "$Slug.env"
        if (-not (Test-Path $EnvFile)) {
            Write-Host "[!] File konfigurasi $EnvFile tidak ditemukan!" -ForegroundColor Red
            exit 1
        }

        Write-Host "[*] Memulai layanan brand '$Slug'..." -ForegroundColor Cyan
        docker compose --env-file $EnvFile -p "notane-$Slug" up -d
        Write-Host "[OK] Layanan brand '$Slug' aktif!" -ForegroundColor Green
    }

    "stop" {
        Assert-Docker
        if ([string]::IsNullOrWhiteSpace($Slug)) {
            Write-Host "[!] Masukkan slug brand. Contoh: .\scripts\brand-manager.ps1 stop kopisenja" -ForegroundColor Red
            exit 1
        }
        $EnvFile = Join-Path $BrandsDir "$Slug.env"
        if (-not (Test-Path $EnvFile)) {
            Write-Host "[!] File konfigurasi $EnvFile tidak ditemukan!" -ForegroundColor Red
            exit 1
        }

        Write-Host "[*] Menghentikan layanan brand '$Slug'..." -ForegroundColor Yellow
        docker compose --env-file $EnvFile -p "notane-$Slug" down
        Write-Host "[OK] Layanan brand '$Slug' telah dihentikan." -ForegroundColor Green
    }

    "restart" {
        Assert-Docker
        if ([string]::IsNullOrWhiteSpace($Slug)) {
            Write-Host "[!] Masukkan slug brand. Contoh: .\scripts\brand-manager.ps1 restart kopisenja" -ForegroundColor Red
            exit 1
        }
        $EnvFile = Join-Path $BrandsDir "$Slug.env"
        docker compose --env-file $EnvFile -p "notane-$Slug" restart
        Write-Host "[OK] Layanan brand '$Slug' telah direstart." -ForegroundColor Green
    }

    "logs" {
        Assert-Docker
        if ([string]::IsNullOrWhiteSpace($Slug)) {
            Write-Host "[!] Masukkan slug brand. Contoh: .\scripts\brand-manager.ps1 logs kopisenja" -ForegroundColor Red
            exit 1
        }
        $EnvFile = Join-Path $BrandsDir "$Slug.env"
        docker compose --env-file $EnvFile -p "notane-$Slug" logs -f server
    }

    "list" {
        Write-Host "==================================================================" -ForegroundColor Cyan
        Write-Host "   DAFTAR BRAND TOKO TERDAFTAR (WINDOWS VPS)" -ForegroundColor Green
        Write-Host "==================================================================" -ForegroundColor Cyan
        $files = Get-ChildItem -Path $BrandsDir -Filter "*.env" -ErrorAction SilentlyContinue
        if ($null -eq $files -or $files.Count -eq 0) {
            Write-Host "Belum ada brand toko yang dibuat. Buat dengan:" -ForegroundColor Yellow
            Write-Host "  .\scripts\brand-manager.ps1 create <slug> <nama_brand> <port>" -ForegroundColor White
            return
        }

        $idx = 1
        foreach ($file in $files) {
            $curSlug = [System.IO.Path]::GetFileNameWithoutExtension($file.Name)
            $content = Get-Content $file.FullName

            $curName = ($content | Where-Object { $_ -match "^BRAND_NAME=(.*)" } | ForEach-Object { $matches[1] })
            if (-not $curName) { $curName = $curSlug }

            $curPort = ($content | Where-Object { $_ -match "^SERVER_PORT=(.*)" } | ForEach-Object { $matches[1] })
            $curPair = ($content | Where-Object { $_ -match "^PAIRING_CODE=(.*)" } | ForEach-Object { $matches[1] })

            $status = "STOPPED"
            if (Get-Command docker -ErrorAction SilentlyContinue) {
                $dockerStatus = docker ps --filter "name=notane-$curSlug-server" --format "{{.Status}}" 2>$null
                if ($dockerStatus) { $status = $dockerStatus }
            }

            Write-Host "[$idx] Brand: $curName (Slug: $curSlug)" -ForegroundColor White
            Write-Host "    - Port Web/API: http://IP_VPS:$curPort" -ForegroundColor Cyan
            Write-Host "    - Pairing Code: $curPair" -ForegroundColor Yellow
            Write-Host "    - Status:       $status" -ForegroundColor Gray
            Write-Host "------------------------------------------------------------------" -ForegroundColor DarkGray
            $idx++
        }
    }

    default {
        Show-Help
    }
}
