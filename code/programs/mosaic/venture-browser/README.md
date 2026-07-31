# Venture browser chrome

This package is the shared Mosaic source of truth for Venture's native browser
chrome. It authors the title, Back, Forward, Home, Reload, address input, Go
action, status line, disabled states, and dispatch contract once in MIL, MLL,
and MSL.

The package intentionally does not draw a web page. `venture-browser-core`
owns navigation and the URL-to-paint pipeline; the macOS host currently mounts
its Metal viewport separately. A future explicit Mosaic native-surface
primitive will replace the empty `content-surface` part with that host-owned
viewport. Until then, this slice proves the shared chrome compiles and lowers
to native SwiftUI and WinUI controls without adding parallel AppKit or Win32
controls.

## Contract

- Slots carry the current address, page title, status text, and host-derived
  disabled flags.
- Emits carry Back, Forward, Home, Reload, address edits, and Navigate.
- Both themes expose the same parts and interaction states.
- `tests/package_compiles.rs` guards the package contract; emitter integration
  tests prove SwiftUI and XAML consume these exact sources.

This package is a browser-wiring milestone, not a claim of complete Venture or
HTML conformance.
