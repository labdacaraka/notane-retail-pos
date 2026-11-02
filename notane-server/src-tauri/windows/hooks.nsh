; ==============================================================================
; Notane Retail Server - NSIS Installer & Uninstaller Custom Hooks
; ==============================================================================

!macro NSIS_HOOK_PREUNINSTALL
    ; 1. Hentikan proses Notane Retail Server jika sedang aktif
    nsExec::Exec 'taskkill /F /IM notane-retail-server.exe /T'

    ; 2. Hentikan proses PostgreSQL Embedded jika sedang berjalan
    nsExec::Exec 'taskkill /F /IM postgres.exe /T'

    ; Berikan jeda agar file lock dilepaskan oleh sistem operasi
    Sleep 1500
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
    ; 1. Hapus seluruh data cluster database PostgreSQL dan file lisensi lokal Notane Retail POS
    ;    Lokasi: %LOCALAPPDATA%\notane-retail-pos (berisi embedded_pg/data, license_keystore.enc, dll)
    RMDir /r "$LOCALAPPDATA\notane-retail-pos"

    ; 2. Hapus WebView2 cache, IndexedDB, dan LocalStorage aplikasi Host Manager
    RMDir /r "$LOCALAPPDATA\com.notane.retailserver"
    RMDir /r "$APPDATA\com.notane.retailserver"

    ; 3. Hapus cache produk dan gambar upload lokal
    RMDir /r "$PROFILE\.notane"
!macroend

