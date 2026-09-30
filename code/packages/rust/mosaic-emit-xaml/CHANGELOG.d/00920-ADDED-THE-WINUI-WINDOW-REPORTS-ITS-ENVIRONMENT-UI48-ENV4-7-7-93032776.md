### Added — the WinUI window reports its environment (UI48 ENV4, §7.7)

The native-complete `MainWindow` with a component root calls
`ObserveEnvironment()` once the runtime has started: it reports the window's
size class, orientation and rendered theme through
`MosaicRuntimeHost.ReportEnvironment`, then again on the content's
`SizeChanged` and `ActualThemeChanged`, each change reported from the
dispatcher queue with one report queued at a time (wired once, so a retried
start does not stack handlers; nothing before the first layout). Only a
failure is written to the status line. Sample shells and dialog-root windows
do not observe.

