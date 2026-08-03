# Changelog

## [0.1.0] - Unreleased

### Added

- Add shared-link hover acceptance for generated SwiftUI and WinUI surfaces:
  the Rust session resolves the URL, Mosaic's status slot displays it, and each
  native host selects its platform pointing-hand cursor.
- Extend generated SwiftUI and WinUI direct interaction acceptance through a
  failing native address commit, requiring the shared Rust transaction to
  retain the current page and history while Mosaic reprojects the failure
  status and attempted address.
- Strengthen generated SwiftUI direct interaction acceptance to require both
  cold-start history controls to expose their disabled Mosaic state and
  suppress native dispatch before the first navigation.
- Require generated SwiftUI and WinUI interaction acceptance to navigate the
  shared browser session through the Mosaic-authored address field's native
  Return/Enter `onCommit` path while retaining the existing Go-button gate.
- Require generated SwiftUI and WinUI interaction acceptance to prove native
  focus transfer from Mosaic-authored chrome into the hosted content surface.
- Strengthen generated SwiftUI and WinUI resize acceptance to require a
  successful post-reflow native frame with changed pixel dimensions.
- Extend generated SwiftUI and WinUI interaction acceptance through the native
  content surface's history shortcuts in both directions, requiring the shared
  Mosaic reducer to reproject each retained page.
- Extend generated SwiftUI and WinUI interaction acceptance through native
  wheel scrolling, requiring the production adapters to update and repaint the
  shared Rust viewport before the keyboard, link, and resize gates continue.
- Extend generated SwiftUI and WinUI interaction acceptance through a real
  native content-surface resize, requiring the production adapters to reflow
  and repaint the retained shared Rust browser session.
- Extend generated SwiftUI and WinUI acceptance through native content-surface
  link activation after the End-key scroll gate, requiring the shared Rust
  session to navigate and reproject the linked address and title.
- Extend generated SwiftUI and WinUI launch acceptance through the native
  content surface's End-key mapping and require the shared Rust viewport to
  report a real scroll-state change.
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
- Direct-launch acceptance for the emitted SwiftUI and WinUI applications,
  requiring each generated shell to load its package-owned Rust bridge and
  render the native content surface against a deterministic local page.
- Direct interaction acceptance for both emitted applications: the package
  hosts edit the generated native address control and activate navigation,
  then require the shared Rust session to load and title a second local page.
- Direct native history acceptance for both emitted applications: the package
  hosts activate the generated Back and Forward controls, then require the
  shared Rust session to reload and retitle both deterministic local pages.
- Direct native Reload and Home acceptance for both emitted applications: the
  generated controls must fetch a changed response at the current URL and then
  return to the shared session's home URL before the gate succeeds.
- The XAML host writes rendered BGRA pixels through WinRT's supported
  `IBuffer.AsStream()` projection, avoiding a native COM projection crash in
  the emitted WinUI application.
- The Mosaic-authored address field and toolbar buttons retain their MLL part
  names as native SwiftUI and WinUI automation identifiers, establishing one
  cross-platform identity contract for direct interaction gates.
