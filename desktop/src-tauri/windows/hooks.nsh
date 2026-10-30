; Nhako Beam installer hooks (bundle.windows.nsis.installerHooks in tauri.windows.conf.json).
; The app is installed for the current user only, so everything here is under HKCU, except the
; firewall rule, which asks for admin rights once.

; Windows Firewall (core/src/firewall.rs, the same rule and commands): without a rule, Windows asks
; when the app first listens and ticks only Private networks, so on a Public network nothing nearby
; can reach it. The rule allows the app in on every profile, after removing the rules Windows made
; from its own prompt (a Block rule there would win). Skipped on silent installs (winget) and when
; it's there already; the app offers the same as Fix it. Declining the prompt still installs.
!define NHAKO_RULE "Nhako Beam (nearby devices)"

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToStack 'netsh advfirewall firewall show rule name="${NHAKO_RULE}"'
  Pop $0
  Pop $1
  ${If} $0 != 0
  ${AndIfNot} ${Silent}
    ClearErrors
    ExecShellWait "runas" "$SYSDIR\cmd.exe" '/c netsh advfirewall firewall delete rule name=all program="$INSTDIR\${MAINBINARYNAME}.exe" & netsh advfirewall firewall add rule name="${NHAKO_RULE}" dir=in action=allow program="$INSTDIR\${MAINBINARYNAME}.exe" enable=yes profile=any' SW_HIDE
    ClearErrors
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; "Send with Nhako Beam" in Explorer, for files and for folders. The app adds them on first run
  ; (src/filemenu.rs, same keys).
  DeleteRegKey HKCU "Software\Classes\*\shell\NhakoBeam"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\NhakoBeam"
  ; "Start when you log in" (tauri-plugin-autostart names it after the product).
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Nhako Beam"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Nhako Beam"
  ; The firewall rule (one admin prompt, only if it's there; declining leaves a rule for a program
  ; that's gone, which does nothing).
  nsExec::ExecToStack 'netsh advfirewall firewall show rule name="${NHAKO_RULE}"'
  Pop $0
  Pop $1
  ${If} $0 == 0
  ${AndIfNot} ${Silent}
    ClearErrors
    ExecShellWait "runas" "$SYSDIR\cmd.exe" '/c netsh advfirewall firewall delete rule name="${NHAKO_RULE}"' SW_HIDE
    ClearErrors
  ${EndIf}
!macroend
