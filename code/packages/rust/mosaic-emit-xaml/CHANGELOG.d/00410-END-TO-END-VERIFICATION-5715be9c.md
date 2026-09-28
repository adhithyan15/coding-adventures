### End-to-end verification

1. Authored a minimal `HelloDialog.mil`/`.mll`/`.dark.msl` triple
   using HostDialog.
2. Ran `mosaic-compile --backend xaml --emit-project -o
   /tmp/proj-test/HelloDialog` — 11 files written.
3. Ran `powershell ./build.ps1`. Build emitted one cosmetic
   MSB4062 error (documented); the .exe was produced at
   `bin/x64/Debug/.../HelloDialog.exe`.
4. Launched the .exe. Window appeared with title "HelloDialog —
   Mosaic → XAML demo".
5. Clicked "Open the dialog" via UIAutomation. The ContentDialog
   appeared with the stub Message and a Close button.
6. Pressed Close. The dialog dismissed; the status bar updated to
   "Dispatch: Close" — proof the `HelloDialogEvent.Close` event
   round-tripped through the generated wiring to the host's
   `OnComponentDispatch` handler.

End-to-end Mosaic → XAML → on-screen dialog with **zero hand-patches**.

## [Unreleased] — HostDialog runnability fixes (A1–A5 from demo catalog)

