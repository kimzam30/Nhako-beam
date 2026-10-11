; Nhako Beam installer hooks (bundle.windows.nsis.installerHooks in tauri.windows.conf.json).
; The app is installed for the current user only, so everything here is under HKCU.

!macro NSIS_HOOK_PREUNINSTALL
  ; "Send with Nhako Beam" in Explorer. The app adds it on first run (src/filemenu.rs, same key).
  DeleteRegKey HKCU "Software\Classes\*\shell\NhakoBeam"
  ; "Start when you log in" (tauri-plugin-autostart names it after the product).
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Nhako Beam"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Nhako Beam"
!macroend
