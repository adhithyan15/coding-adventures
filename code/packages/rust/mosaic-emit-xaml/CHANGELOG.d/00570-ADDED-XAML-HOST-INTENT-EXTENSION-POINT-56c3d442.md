### Added - XAML host intent extension point

Generated WinUI project shells now preserve structured `HostIntent` values from
optional `MosaicHost.HandleEvent` results and can delegate them to an
app-provided asynchronous `MosaicHost.HandleHostIntent(Window, Component,
HostIntent)` method. This lets app packages implement native file pickers or
other platform-owned workflows without hand-patching generated `MainWindow`
code.

