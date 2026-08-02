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
- SwiftUI host-driven prop refresh after native Metal link activation, keeping
  the Mosaic-authored address, title, and history controls synchronized with
  the shared Rust browser session just like the generated WinUI host.
- Package-owned SwiftUI and WinUI content surfaces now accept native line,
  page, start/end, and platform history shortcuts. Both adapters translate key
  input into shared Rust scroll commands and existing Mosaic Back/Forward
  events instead of owning navigation or scrolling semantics themselves.
- Package-owned SwiftUI and WinUI content surfaces report native logical size
  changes through matching Rust resize ABIs, reflowing the retained document
  and repainting without refetching the page or duplicating chrome.
