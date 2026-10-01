### Changed — the WinUI touch layout shares EngramAppEvent and the window switches to it (UI48 §7.11)

`EngramApp.touch.xaml.cs` declares `partial class EngramAppTouch` and raises
the default's `EngramAppEvent`; no `EngramApp.touch.Event.cs` is generated.
The WinUI project carries both controls, and its window selects between them
by the conventional rule (`touch` for a coarse pointer), mounting each root
strictly in the native-complete profile. A new test checks the native-complete
project's roots, selector and package fragment. On a Windows desktop the
pointer is fine, so the touch layout is compiled but not shown. The sample
window looks Engram's own XAML host up by the type of the root showing, so
that host (typed on `EngramApp`) serves the default layout.
