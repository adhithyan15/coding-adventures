# Changelog

## [0.1.0] - Unreleased

### Added

- Mosaic-authored Venture browser chrome with light and dark themes.
- Typed slots for the address, page title, status, and disabled controls.
- Typed dispatch events for Back, Forward, Home, Reload, address edits, and
  navigation.
- A compile-time ratchet tying the MIL slots and events to Venture's shared
  host-neutral chrome reducer.
- A typed `content-surface` node slot lowered through `HostSurface` for native
  viewport composition across every Mosaic package backend, including Qt,
  SwiftUI, XAML, React/Electron, Flutter, Compose, HTML, and Web Components.
- Runnable project-shell acceptance proving each backend can obtain and mount
  that content surface through its optional `MosaicHost` contract; the macOS
  SwiftUI gate builds with a real host-provided `NSView`.
- POSIX and PowerShell backend-matrix entry points that emit the same Venture
  package for all nine Mosaic targets and directly invoke each available web,
  Qt, SwiftUI, XAML, Flutter, or Compose build toolchain.
- A package-owned SwiftUI `MosaicHost` adapter that loads Venture's Rust
  dynamic bridge, hydrates generated chrome from `BrowserChromeController`,
  dispatches Mosaic events back to it, and mounts the live Metal viewport with
  native scrolling and link activation.
- A package-owned XAML `MosaicHost` adapter and `venture-browser-windows`
  Direct2D bridge that hydrate the same generated chrome contract, mount the
  live browser viewport as a WinUI `UIElement`, and keep scrolling and link
  activation in the shared Rust session.
