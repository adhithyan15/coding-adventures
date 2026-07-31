# Venture browser chrome

This package is the shared Mosaic source of truth for Venture's native browser
chrome. It authors the title, Back, Forward, Home, Reload, address input, Go
action, status line, disabled states, and dispatch contract once in MIL, MLL,
and MSL.

The package intentionally does not draw a web page. `venture-browser-core`
owns navigation and the URL-to-paint pipeline. The `content-surface` node slot
now lowers through Mosaic's `HostSurface` primitive on every package backend:
React/Electron, SwiftUI, Qt Quick, Web Components, HTML, XAML, Flutter, and
Compose. Each host supplies its native node/component/widget (for example a
Metal `AnyView`, Qt `Component`, or Direct2D-backed `UIElement`) without
recreating the surrounding chrome in backend-specific UI code.

## Contract

- Slots carry the current address, page title, status text, and host-derived
  disabled flags; the host supplies the native page renderer as a node slot.
- Emits carry Back, Forward, Home, Reload, address edits, and Navigate.
- `venture-browser-core::BrowserChromeController` is the shared reducer and
  slot projection for that exact contract.
- Both themes expose the same parts and interaction states.
- `tests/package_compiles.rs` guards the package contract; the package artifact
  builder compiles these exact sources, emits project shells, and verifies a
  real host-surface mount for every backend in its exhaustive `Backend::ALL`
  list.

This package is a browser-wiring milestone, not a claim of complete Venture or
HTML conformance.
