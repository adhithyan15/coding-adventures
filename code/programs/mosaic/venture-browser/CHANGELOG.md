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
