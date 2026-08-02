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
- The project-shell gate also verifies that every backend obtains the native
  node through its optional `MosaicHost` seam rather than substituting its
  sample placeholder. On macOS, the generated SwiftPM app is compiled with a
  real host-provided `NSView`.
- The SwiftUI project receives a package-owned `MosaicHost.swift` adapter. It
  dynamically loads `libventure_browser_macos.dylib`, projects the shared
  `BrowserChromeController` props, sends generated Mosaic events back through
  the shared reducer, and mounts the live Metal page renderer as the
  `content-surface` `NSView`. Native link activation asks the generated Mosaic
  host state to reproject address, title, and history slots from that same Rust
  browser session rather than reimplementing them in Swift.
- The XAML project receives the matching package-owned `MosaicHost.cs`
  adapter. It loads `venture_browser_windows.dll`, projects that same shared
  reducer into generated WinUI controls, and mounts Direct2D-rendered pixels
  in the generated `content-surface` `UIElement`. Pointer-wheel scrolling and
  link activation call back into the same Rust browser session. The direct
  generated-app gates inject native wheel deltas and require those production
  paths to scroll and repaint the shared viewport.
- The native content surfaces are keyboard focus targets. Arrow, Page,
  Space/Shift-Space, Home, and End keys use the exact semantic scroll-command
  contract owned by `venture-browser-core`; Command-Left/Right on macOS and
  Alt-Left/Right on Windows reuse the Mosaic `onBack`/`onForward` reducer
  events and reproject chrome props after navigation. The direct generated-app
  gates require both native history shortcuts to traverse a real linked page.
- Native SwiftUI and WinUI surface-size changes use matching Rust resize ABIs.
  The shared session recomposes its retained render tree for the new logical
  viewport, preserves and clamps scroll state, updates hit regions, and
  repaints without refetching the HTML document.
- Platform-native integration gates build and directly launch the emitted
  SwiftUI and WinUI applications against a local deterministic page. Each app
  must load its package-owned bridge and render the native `content-surface`
  before writing its acceptance marker.

## Build every backend

Venture owns a cross-platform acceptance entry point for the complete Mosaic
backend matrix. Both scripts compile this exact MIL/MLL/MSL package, emit all
nine project shells, and then invoke the native build available for each
backend on the current machine:

```sh
./scripts/build-all.sh
```

```powershell
.\scripts\build-all.ps1
```

React and Electron run their production builds; HTML and Web Components run
JavaScript syntax checks; SwiftUI, Qt, XAML, Flutter, and Compose invoke their
native toolchains. Platform-exclusive builds are explicitly deferred to their
native host; missing host-applicable toolchains are reported as skips. Pass
`--strict` on POSIX or `-Strict` on PowerShell to reject those missing
toolchains in a provisioned macOS, Windows, or Linux build job. `--emit-only` /
`-EmitOnly` remains useful for inspecting all generated projects without
claiming that their native builds passed.

On macOS the matrix also builds the Venture Rust dynamic library and places it
next to the generated SwiftUI project. Run the native shell from that directory
with `swift run`; set `VENTURE_START_URL` to override the initial page or
`VENTURE_BROWSER_LIBRARY` to load a bridge from another absolute path.

On Windows the PowerShell matrix builds `venture-browser-windows`, copies its
DLL beside the generated WinUI project, and runs the x64 `dotnet build`. The
generated project copies that native bridge next to the executable so its
package-owned `MosaicHost.cs` can load it without a handwritten Win32 chrome
layer.

The macOS and Windows Rust package tests are the authoritative direct-launch
gates. Each test builds its platform bridge in an isolated temporary target
before emitting and launching the generated project:

```sh
cargo test -p venture-browser-macos
```

```powershell
cargo test -p venture-browser-windows
```

These gates launch the generated window, render the host surface, edit and
navigate the native address control, and invoke the Mosaic-authored Back,
Forward, Reload, and Home controls. They then focus the emitted native content
surface and require a wheel delta followed by an End-key command to scroll the
shared Rust viewport. Before surface input begins, each host must prove that
focus moved from generated chrome into the real native content surface. From
that scrolled state, each host must return to
document start, activate a real HTML link through the native surface pointer
path, and observe the shared Rust session reproject the linked address and
title into the generated chrome. The gates then use Command-Left/Right or
Alt-Left/Right to traverse that shared history in both directions before they
resize the real native content surface and require the production
adapter to reflow that retained shared session and produce a successful native
frame whose pixel dimensions match the resized surface before reporting
success. Every transition must update the shared browser session before the
shell reports success.

This package is a browser-wiring milestone, not a claim of complete Venture or
HTML conformance.
