!macro NSIS_HOOK_PREUNINSTALL
  ; auto-launch uses the product name as the current-user Run value.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Glimtra"
!macroend
