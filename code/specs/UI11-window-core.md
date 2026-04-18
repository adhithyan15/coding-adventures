# UI11 — Window Core

## Overview

This spec introduces a cross-platform window abstraction that is shared across
languages and first anchored in Rust for the native backends. It sits between
low-level native event facilities and renderer-specific drawing code. Its
purpose is not to hide every platform quirk. Its purpose is to establish a
small, honest contract:

- create a presentation host
- receive normalized window and input events
- expose renderer-facing handles for the host's drawable surface

On desktop platforms, that presentation host is a real native window. In the
browser, it is a mounted HTML canvas host that behaves like a window from the
renderer's point of view: it has a title-like identity, logical size, device
pixel ratio, redraw cadence, focus, visibility, and input events.

This gives us one Rust-facing model for:

- AppKit + Metal on Apple platforms
- Win32 + Direct2D on Windows
- future Wayland/X11 + Cairo or GPU APIs on Linux
- HTML Canvas in a browser tab

The key boundary is:

- rendering APIs draw into an existing host surface
- window backends create and manage that host surface

So Metal, Direct2D, and Cairo stay renderers. They do not become the windowing
layer.

---

## Goals

1. Define one repository-owned window API whose semantics stay the same across
   languages.
2. Use Rust crates as the native backend implementations that other languages
   can call through bridges.
3. Expose renderer-friendly native handles without forcing the core contract to
   know how each renderer works.
4. Support a browser backend without pretending the browser provides OS-style
   windows.
5. Stay compatible with the existing `event-loop` and `native-event-core`
   layers.

## Non-Goals

- A widget toolkit
- layout, styling, accessibility trees, or menu systems
- cross-platform text input perfection in the first slice
- flattening Wayland, Win32, AppKit, and browser DOM semantics into one giant
  "universal window" interface

---

## Where It Fits

```text
renderer crates
paint-metal / paint-vm-direct2d / future cairo renderer / browser canvas renderer
    ↑
window-core
    ↑
Rust native backends: window-appkit / window-win32 / future window-wayland
TypeScript browser backend: window-canvas
    ↑
AppKit run loop / Win32 message pump / Wayland queue / browser DOM + RAF
    ↑
native-event-core / event-loop / platform dispatch
```

The window layer is above raw readiness/completion backends. A resize or key
press is not an `epoll` event. It is a window-system event that the backend
normalizes into a `WindowEvent`.

---

## Package Layout

### `window-core`

The backend-neutral API crate.

Responsibilities:

- window identifiers
- logical and physical size types
- builder attributes
- normalized window and input events
- renderer-target handle enums
- `Window` and `WindowBackend` traits
- error types and validation rules

This Rust crate is the first concrete implementation of the contract, but the
abstraction itself is language-neutral. Other languages must preserve the same
names, event meanings, and renderer-target concepts even when their concrete
runtime types differ.

### `window-appkit`

Apple desktop backend.

Responsibilities:

- create `NSApplication`, `NSWindow`, and content view state
- translate AppKit events into `WindowEvent`
- expose AppKit/Metal-friendly handles such as `NSWindow`, `NSView`, and
  optional `CAMetalLayer`

### `window-win32`

Windows desktop backend.

Responsibilities:

- register a window class and create `HWND`
- translate Win32 messages into `WindowEvent`
- expose Win32-friendly handles such as `HWND`
- leave Direct2D or Direct3D device creation to renderer crates

### `window-canvas` (TypeScript)

Browser backend.

Responsibilities:

- treat a mounted `<canvas>` element as the presentation host
- map DOM input, resize, visibility, and animation-frame callbacks into
  `WindowEvent`
- keep CSS logical size and backing-store pixel size in sync with
  `devicePixelRatio`
- expose canvas-focused render targets for browser rendering code

This backend is intentionally implemented in pure TypeScript rather than in a
Rust crate. Node-based TypeScript can call the Rust native backends; the pure
browser path should stay browser-native.

### `window-c` (Rust C ABI)

Native bridge foundation for languages that already consume repository-owned C
wrappers.

Responsibilities:

- expose a stable C ABI over the shared `window-core` model
- translate plain C structs into `WindowAttributes`
- create native windows through `window-appkit` on Apple platforms today
- reserve the same ABI for Win32-backed creation once `window-win32` grows real
  native creation
- give Go, Swift, C#, F#, and other C-interop languages one backend-neutral
  entry point

### Future Backends

- `window-wayland`
- `window-x11`

Linux is intentionally deferred because "Linux windowing" is not one thing.
Wayland and X11 deserve honest backends, not a rushed false abstraction.

---

## Polyglot Package Families

This repository now treats windowing as a package family rather than as a
Rust-only API.

### Shared Contract Packages

Every language may ship a `window-core` package that mirrors the repository
contract:

- identity types like `WindowId`
- logical and physical size structs
- `SurfacePreference`
- `MountTarget`
- `WindowAttributes` and builder validation
- normalized `WindowEvent` values
- render-target tags and backend-neutral errors

These packages do not need native interop in order to be useful. They are the
shared semantic layer.

### Runtime-Bridge Native Packages

Languages with existing runtime-specific Rust bridge crates should add native
window packages that preserve the same contract while delegating native work to
Rust:

- Python via `python-bridge`
- Ruby via `ruby-bridge`
- Lua via `lua-bridge`
- Perl via `perl-bridge`
- Node/TypeScript-on-Node via `node-bridge`
- Elixir via `erl-nif-bridge`

These packages should expose the same concepts as `window-core`, but the native
object lifetime is owned by the runtime-specific bridge layer rather than by a
C ABI.

### C-Interop Native Packages

Languages that already wrap repository-owned C shims should build on
`window-c`:

- Go
- Swift
- C#
- F#

These ports should not talk directly to Objective-C or Win32 from every
language package. The Rust `window-c` shim is the stable native seam.

### Pure Browser Package

The browser implementation is special:

- `window-canvas` is pure TypeScript
- it does not go through Rust
- it still mirrors the same `window-core` contract and `WindowEvent` meanings

This preserves one abstraction while respecting the browser's actual runtime
model.

---

## Core Concepts

### 1. Window vs Render Target

A window is the thing that owns lifecycle and input:

- open
- resize
- focus
- close
- visibility
- redraw requests

A render target is the thing a renderer needs in order to draw:

- an `NSView` / `CAMetalLayer`
- an `HWND`
- a Wayland surface
- an HTML canvas mount

The window abstraction must expose both, but it must not confuse them.

### 2. Browser "Windowing"

The browser backend does not create native top-level OS windows. Instead it
models each mounted canvas host as a window-like object.

That host gets:

- a `WindowId`
- logical size in CSS pixels
- physical size in backing-store pixels
- focus and blur events
- pointer and keyboard events
- `RedrawRequested` from `requestAnimationFrame`
- `VisibilityChanged` from page or element visibility changes
- `CloseRequested` from teardown or explicit unmount

This is the correct abstraction boundary. The renderer wants "a thing I can draw
into that resizes and emits input," and a mounted canvas satisfies that.

### 3. Honest Escape Hatches

Some features are backend-specific:

- AppKit activation policy
- Win32 class styles
- browser mount selectors
- future Wayland protocol extensions

`window-core` therefore defines a compact shared API and allows backend crates
to expose additional configuration and handle accessors without forcing those
details into every platform.

---

## Public API

The initial Rust-facing API is:

```rust
pub struct WindowId(pub u64);

pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

pub enum SurfacePreference {
    Default,
    Metal,
    Direct2D,
    Cairo,
    Canvas2D,
}

pub enum MountTarget {
    Native,
    BrowserBody,
    ElementId(String),
    QuerySelector(String),
}

pub struct WindowAttributes {
    pub title: String,
    pub initial_size: LogicalSize,
    pub min_size: Option<LogicalSize>,
    pub max_size: Option<LogicalSize>,
    pub visible: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub preferred_surface: SurfacePreference,
    pub mount_target: MountTarget,
}

pub struct WindowBuilder { ... }

pub trait Window {
    fn id(&self) -> WindowId;
    fn logical_size(&self) -> LogicalSize;
    fn physical_size(&self) -> PhysicalSize;
    fn scale_factor(&self) -> f64;
    fn request_redraw(&self) -> Result<(), WindowError>;
    fn set_title(&self, title: &str) -> Result<(), WindowError>;
    fn set_visible(&self, visible: bool) -> Result<(), WindowError>;
    fn render_target(&self) -> RenderTarget;
}

pub trait WindowBackend {
    type Window: Window;

    fn create_window(
        &mut self,
        attributes: WindowAttributes,
    ) -> Result<Self::Window, WindowError>;

    fn pump_events(&mut self) -> Result<Vec<WindowEvent>, WindowError>;
}
```

### Events

```rust
pub enum WindowEvent {
    Created { window_id: WindowId },
    Resized {
        window_id: WindowId,
        logical_size: LogicalSize,
        physical_size: PhysicalSize,
        scale_factor: f64,
    },
    RedrawRequested { window_id: WindowId },
    CloseRequested { window_id: WindowId },
    Destroyed { window_id: WindowId },
    FocusChanged { window_id: WindowId, focused: bool },
    VisibilityChanged { window_id: WindowId, visible: bool },
    PointerMoved { window_id: WindowId, x: f64, y: f64 },
    PointerButton {
        window_id: WindowId,
        button: PointerButton,
        state: ElementState,
    },
    Scroll {
        window_id: WindowId,
        delta_x: f64,
        delta_y: f64,
    },
    Key {
        window_id: WindowId,
        key: Key,
        state: ElementState,
        modifiers: ModifiersState,
        text: Option<String>,
    },
    TextInput { window_id: WindowId, text: String },
}
```

### Render Targets

The core crate exposes renderer-facing handles as tagged enums, not as a single
fake universal pointer:

```rust
pub enum RenderTarget {
    AppKit(AppKitRenderTarget),
    Win32(Win32RenderTarget),
    BrowserCanvas(BrowserCanvasRenderTarget),
    Wayland(WaylandRenderTarget),
    X11(X11RenderTarget),
}
```

Examples:

- `AppKitRenderTarget` contains `NSWindow`, `NSView`, and optional
  `CAMetalLayer` addresses represented opaquely
- `Win32RenderTarget` contains `HWND`
- `BrowserCanvasRenderTarget` contains mount metadata and size state

The enum is intentionally explicit so renderer crates can pattern-match on the
target they actually support.

---

## Validation Rules

`window-core` validates only portable invariants:

- width and height must be finite and non-negative
- `min_size <= initial_size <= max_size` where bounds exist
- browser mount targets are only valid for browser backends
- native-only expectations are rejected by browser backends and vice versa

Backend crates validate platform-specific invariants:

- Win32 class registration state
- AppKit main-thread requirements
- browser DOM mount lookup failures

---

## Browser Canvas Approach

The browser backend follows these rules:

1. Logical size is the canvas CSS box in CSS pixels.
2. Physical size is `logical_size * devicePixelRatio`, rounded to integers.
3. The backend updates the canvas backing-store size whenever the logical size
   or DPR changes.
4. `requestAnimationFrame` becomes `RedrawRequested`.
5. DOM `pointer*`, `wheel`, `keydown`, `keyup`, `focus`, and `blur` events map
   to normalized `WindowEvent` values.
6. `ResizeObserver` or equivalent mount observation produces `Resized`.
7. Mount lookup is described by `MountTarget`:
   - `BrowserBody` means create or attach under `<body>`
   - `ElementId(id)` means attach to that element
   - `QuerySelector(sel)` means resolve that selector

The browser backend therefore behaves like a tab-local multi-window system where
each mounted canvas is one window host.

---

## C ABI Shape

The first C wrapper is intentionally small and backend-neutral.

It owns:

- C enums mirroring `SurfacePreference`
- C enums describing mount-target kind and render-target kind
- POD structs for logical size, physical size, and attributes
- opaque window handles
- error retrieval through a thread-local last-error message function

The initial exported operations are:

- create a native window on the current platform
- query id, logical size, physical size, and scale factor
- request redraw
- set title
- set visibility
- inspect the render-target kind
- retrieve platform-specific target payloads when available

This ABI is not the widget toolkit. It is the portable seam that lets higher
level language packages start creating and managing native presentation hosts.

---

## Phased Delivery

### Phase 1

- add `window-core`
- add backend scaffolding crates:
  - `window-appkit`
  - `window-win32`
- implement the shared Rust API, validation, and mock-driven tests
- document the browser-canvas host model that TypeScript will implement

### Phase 2

- add `window-c`
- implement the pure TypeScript `window-core` and `window-canvas` packages
- wire browser mount handling, DPR synchronization, and normalized DOM events
- expose macOS native creation through the C ABI
- refactor existing macOS window display crates to consume `window-appkit`

### Phase 3

- add language-level `window-core` mirrors in the supported package families
- add bridge-backed native packages for Python, Ruby, Lua, Perl, Node, and
  Elixir
- add C-interop native packages for Go, Swift, C#, and F#
- wire native creation and event translation for Win32 through the same seams

### Phase 4

- add Wayland and/or X11 backends
- integrate more advanced lifecycle, IME, clipboard, and accessibility support

---

## Testing Strategy

### `window-core`

- builder validation tests
- size conversion tests
- render-target tagging tests
- mock backend integration tests

### Native Backends

- unsupported-platform tests on non-target CI
- platform-gated smoke tests on supported targets
- event normalization tests where platform APIs can be mocked

### Browser Backend

- TypeScript unit tests for mount-target logic and backing-store size
  synchronization math
- TypeScript unit tests for normalized DOM event translation and redraw
  scheduling

### C ABI

- Rust unit tests for C-struct conversion and error propagation
- platform-gated smoke tests for native window creation where available

---

## Initial Recommendation

Start with a strong `window-core` and honest backend shells. The first real
consumer should be a renderer demo that creates a window host and then hands its
render target to Metal, Direct2D, or Canvas2D code. That proves the boundary is
correct before we grow into a full UI toolkit.
