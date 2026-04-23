# UI12 — Python Window Native

## Overview

This spec adds the first runtime-bridge window package on top of the Rust
window foundations introduced in UI11.

The package is:

- `code/packages/python/window-native`
- published as `coding-adventures-window-native`
- imported as `window_native`

Its job is to expose a small Python-facing window API while delegating native
window creation and mutation directly to the Rust `window-core` and
`window-appkit` crates through `python-bridge`.

This package is intentionally not built on top of the `window-c` ABI. The C
ABI remains useful for languages that need a stable C seam, but Python should
use the repository's native bridge infrastructure instead.

---

## Goals

1. Let a Python script create a real native window on macOS.
2. Preserve the shared UI11 concepts:
   - logical size
   - physical size
   - surface preference
   - render target kind
   - mutable window state
3. Keep the unsafe/native boundary inside one Rust extension module.
4. Provide a Pythonic surface API without forcing Python callers to know about
   raw Objective-C or C ABI details.

## Non-Goals

- Full event pumping in the first Python slice
- Menus, drag/drop, accessibility, IME, or dialogs
- Linux and Windows native Python backends in this change
- Re-implementing `window-core` entirely in Python before proving the native
  bridge path

---

## Package Layout

```text
code/packages/python/window-native/
├── BUILD
├── CHANGELOG.md
├── Cargo.toml
├── README.md
├── build.rs
├── pyproject.toml
├── src/
│   ├── lib.rs
│   └── window_native/
│       └── __init__.py
└── tests/
    └── test_window_native.py
```

### Rust Extension Module

The Rust `cdylib` exports a CPython extension module named `window_native`
through `python-bridge`.

Responsibilities:

- keep a process-local registry of native window handles
- create windows through `window-appkit::AppKitBackend`
- expose handle-based module functions for:
  - create
  - close
  - query id/size/scale
  - request redraw
  - set title
  - set visibility
  - inspect render target kind
- translate Rust `WindowError` values into a Python `WindowError` exception

### Python Wrapper Layer

`src/window_native/__init__.py` wraps the low-level extension functions with
lightweight Python value types and a more readable API.

Responsibilities:

- define `SurfacePreference` as an `IntEnum`
- define `RenderTargetKind` as a `str` `Enum`
- define `LogicalSize` and `PhysicalSize` as dataclasses
- define a small `Window` wrapper class that owns one native handle
- expose `create_window(...)`

---

## Public Python API

### Enums

```python
class SurfacePreference(IntEnum):
    DEFAULT = 0
    METAL = 1
    DIRECT2D = 2
    CAIRO = 3
    CANVAS2D = 4


class RenderTargetKind(str, Enum):
    NONE = "none"
    APPKIT = "appkit"
    WIN32 = "win32"
    BROWSER_CANVAS = "browser-canvas"
    WAYLAND = "wayland"
    X11 = "x11"
```

### Value Types

```python
@dataclass(frozen=True)
class LogicalSize:
    width: float
    height: float


@dataclass(frozen=True)
class PhysicalSize:
    width: int
    height: int
```

### Window Wrapper

```python
class Window:
    def close(self) -> None: ...
    def id(self) -> int: ...
    def logical_size(self) -> LogicalSize: ...
    def physical_size(self) -> PhysicalSize: ...
    def scale_factor(self) -> float: ...
    def request_redraw(self) -> None: ...
    def set_title(self, title: str) -> None: ...
    def set_visible(self, visible: bool) -> None: ...
    def render_target_kind(self) -> RenderTargetKind: ...
```

### Factory

```python
def create_window(
    *,
    title: str = "Coding Adventures Window",
    width: float = 800.0,
    height: float = 600.0,
    preferred_surface: SurfacePreference = SurfacePreference.DEFAULT,
    visible: bool = True,
    resizable: bool = True,
    decorations: bool = True,
    transparent: bool = False,
) -> Window: ...
```

---

## Platform Behavior

### macOS

`create_window(...)` creates a real AppKit window through `window-appkit`.

The first slice must support:

- default surface selection
- explicit Metal preference
- hidden-window creation for tests and smoke checks
- title updates
- visibility toggling
- redraw requests
- render target kind reporting as `RenderTargetKind.APPKIT`

### Non-macOS

The module should import successfully, but native creation should fail closed
with `WindowError` explaining that the Python native window backend is only
wired for AppKit in this slice.

That keeps the package honest while still allowing tests to assert the expected
unsupported-platform behavior on Linux and Windows CI.

---

## Error Model

The extension module exports a Python `WindowError` exception.

Use it for:

- unsupported platform
- unsupported renderer/backend combinations
- invalid attributes such as non-finite or negative sizes
- invalid or already-closed window handles

Signature mistakes at the Python boundary should still use normal Python
exceptions such as `TypeError`.

---

## Testing

The package should exceed 80% coverage.

Required tests:

1. Pure wrapper tests:
   - enums round-trip expected values
   - dataclass wrappers are returned with correct sizes
   - `Window.close()` is idempotent
2. macOS tests:
   - create a hidden window successfully
   - query id, sizes, scale factor, and render target kind
   - mutate title / visibility / redraw without error
3. non-macOS tests:
   - `create_window(...)` raises `WindowError` with an unsupported-platform
     style message

The tests must not require a long-running event loop in the first slice.

---

## Future Work

- add event pumping and normalized `WindowEvent` values to Python
- add Python-side `WindowBuilder` and `WindowAttributes`
- add Python wrappers for Win32 and future Linux backends
- mirror the same architecture in Perl using `perl-bridge`
