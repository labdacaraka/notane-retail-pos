; ==============================================================================
; Notane Manager - NSIS Installer & Uninstaller Custom Hooks
; ==============================================================================

!macro NSIS_HOOK_PREUNINSTALL
    ; Hentikan proses Notane Manager jika sedang aktif
    nsExec::Exec 'taskkill /F /IM notane-manager.exe /T'
    Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
    ; Hapus basis data SQLite provider dan cache WebView2
    RMDir /r "$LOCALAPPDATA\notane-license-manager"
    RMDir /r "$APPDATA\notane-license-manager"
    RMDir /r "$LOCALAPPDATA\com.notane.licensemanager"
    RMDir /r "$APPDATA\com.notane.licensemanager"
!macroend

