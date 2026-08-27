# venture-browser-windows

This crate is Venture's native Windows content bridge. It reuses
`venture-browser-core` for browser sessions, history, loading, chrome state,
scrolling, and link hit-testing, then renders the current viewport through
`paint-vm-direct2d` as BGRA8 pixels for WinUI's `WriteableBitmap`.

The bridge loads the shared versioned bookmark catalog from
`%LOCALAPPDATA%\Venture\bookmarks.json` and persists generated chrome toggle
commands atomically. Set `VENTURE_BOOKMARKS_PATH` to use an isolated profile.

It does not create browser chrome or a handwritten Win32 window. The shared
`programs/mosaic/venture-browser` MIL/MLL/MSL package emits the WinUI controls
and installs its package-owned `host/xaml/MosaicHost.cs` adapter into the
generated project shell. That adapter calls this crate's C ABI and mounts the
pixel surface in Mosaic's `content-surface` slot.

```powershell
cargo build -p venture-browser-windows
```

The package-level Windows acceptance entry point builds the DLL, copies it
beside the emitted XAML project, and runs the generated x64 WinUI build:

```powershell
code\programs\mosaic\venture-browser\scripts\build-all.ps1
```

The Windows-only integration test compiles the package-owned C# adapter inside
the generated WinUI project, launches its executable against a local
deterministic page, and requires a successful Direct2D content-surface render:

```powershell
cargo test -p venture-browser-windows --test xaml_project_build
```

Launch and interaction are the default and remain required on a local or
self-hosted Windows machine with an interactive desktop. GitHub-hosted Windows
workers run the same project-generation and compiler gate with
`MOSAIC_SKIP_INTERACTIVE_WINDOWS_ACCEPTANCE=1`, because those workers can reject
WinUI initialization before `OnLaunched` with the WinRT stowed-exception status
`0xc000027b`. That build-only accommodation must not be used to claim interactive
Windows acceptance.

Cross-platform unit tests exercise the shared session and chrome event path
without claiming complete browser or HTML conformance.
