; ==============================================================================
; Notane Retail POS Client - NSIS Installer & Uninstaller Custom Hooks
; ==============================================================================

!macro NSIS_HOOK_PREUNINSTALL
    ; Hentikan proses Notane Retail POS jika sedang aktif
    nsExec::Exec 'taskkill /F /IM notane-retail-pos.exe /T'
    Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
    ; Hapus cache WebView2 dan data sesi desktop client
    RMDir /r "$LOCALAPPDATA\com.notane.retailpos"
    RMDir /r "$APPDATA\com.notane.retailpos"
!macroend

