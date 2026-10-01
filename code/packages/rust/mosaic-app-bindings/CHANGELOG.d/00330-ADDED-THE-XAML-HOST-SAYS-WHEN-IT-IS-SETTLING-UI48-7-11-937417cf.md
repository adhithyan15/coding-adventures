### Added — the XAML host says when it is settling (UI48 §7.11)

`MosaicRuntimeHost.IsSettling` is true while the loaded runtime is running its
effect loop (a dispatch, a report or an effect answer), false otherwise and
with no runtime. A WinUI window that switches layout roots checks it when its
queued switch runs, so a root is never swapped out while an effect handler may
be applying props to it. It reads the loop's own counter without the lock, so
the UI thread never waits on another thread's settle. The XAML effect driver
checks it is true inside an effect handler and false before and after the
dispatch. Additive: nothing else in the binding changed.
