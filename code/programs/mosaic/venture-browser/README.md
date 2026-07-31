# Venture browser chrome

This package is the shared Mosaic source of truth for Venture's native browser
chrome. It authors the title, Back, Forward, Home, Reload, address input, Go
action, status line, disabled states, and dispatch contract once in MIL, MLL,
and MSL.

The package intentionally does not draw a web page. `venture-browser-core`
owns navigation and the URL-to-paint pipeline. The `content-surface` node slot
now lowers through Mosaic's `HostSurface` primitive, so SwiftUI hosts can pass
an `AnyView` backed by Metal and WinUI hosts can pass a `UIElement` backed by
Direct2D without recreating the surrounding chrome in AppKit or Win32.

## Contract

- Slots carry the current address, page title, status text, and host-derived
  disabled flags; the host supplies the native page renderer as a node slot.
- Emits carry Back, Forward, Home, Reload, address edits, and Navigate.
- `venture-browser-core::BrowserChromeController` is the shared reducer and
  slot projection for that exact contract.
- Both themes expose the same parts and interaction states.
- `tests/package_compiles.rs` guards the package contract; emitter integration
  tests prove SwiftUI and XAML consume these exact sources.

This package is a browser-wiring milestone, not a claim of complete Venture or
HTML conformance.
