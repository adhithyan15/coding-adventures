# P2D10 — Paint VM Cairo + Skia Barcode Integration

## Status: Implemented

## Problem

`barcode-1d::render_scene_to_pixels()` returned a hard error on Linux and other
non-Windows, non-Apple platforms:

```
"native barcode rendering is not wired on this platform yet"
```

Two root causes:

1. **Cairo was gated to Linux/BSD only** despite Cairo 1.18.x being widely
   available on macOS via Homebrew (`pkg-config --modversion cairo` returns
   `1.18.4` on the developer machine). The `paint-vm-cairo` crate compiled
   only its native Cairo path for `target_os = "linux" | "freebsd" | "openbsd"
   | "netbsd"`. On macOS it silently fell back to a deterministic software
   smoke renderer.

2. **Skia was not wired to the barcode pipeline at all.** `paint-vm-skia` (via
   the `skia-safe` crate which bundles Skia as a C++ static library) is
   genuinely cross-platform — it builds and renders correctly on macOS, Linux,
   and Windows without any system-level Cairo dependency. It was never added as
   a dependency of `barcode-1d`.

The combined effect: Linux users had no path to a PNG from barcode data, and
macOS users of `paint-vm-cairo` got the degraded smoke renderer instead of the
native Cairo backend.

## Solution

### 1. Enable Cairo on macOS in `paint-vm-cairo`

The `cfg` predicates throughout `paint-vm-cairo/src/lib.rs` were extended to
include `target_os = "macos"`:

```
linux | freebsd | openbsd | netbsd
  → linux | macos | freebsd | openbsd | netbsd
```

This change affects:
- The `[target.'cfg(...)'.dependencies]` block in `Cargo.toml` so that
  `cairo-rs` is pulled in on macOS.
- The positive `#[cfg(any(target_os = "linux", ...))]` guards that enable the
  `mod native_cairo` rendering path.
- The negative `#[cfg(not(any(target_os = "linux", ...)))]` guards that enable
  the software smoke path — macOS now falls into the native path, not the
  smoke path.
- The `descriptor()` function, which now returns the "native" capability
  tier (all `SupportLevel::Supported` primitives) on macOS instead of the
  degraded smoke-renderer tier.

On Windows, where Cairo is not available as a system library, the software
smoke renderer remains enabled and the `cairo-rs` crate is not pulled in.

### 2. Wire Cairo and Skia to `barcode-1d`

**Cargo.toml additions:**

```toml
[dependencies]
paint-vm-skia = { path = "../paint-vm-skia" }   # always available

[target.'cfg(any(target_os = "linux", target_os = "macos", ...))'.dependencies]
paint-vm-cairo = { path = "../paint-vm-cairo" }  # Cairo available on unix
```

**Backend priority per platform:**

| Platform       | Primary         | Secondary | Fallback |
|----------------|-----------------|-----------|----------|
| Windows        | Direct2D (GPU)  | Skia (CPU)| —        |
| macOS          | Metal (GPU)     | Cairo*    | Skia (CPU)|
| Linux / BSD    | Cairo (CPU)     | Skia (CPU)| —        |
| Other          | Skia (CPU)      | —         | —        |

\* On macOS, `render_scene_to_pixels()` uses Metal (not Cairo) because Metal
gives hardware-accelerated rendering and is always present on Apple silicon.
Cairo is reachable on macOS via `render_with_backend("...", "cairo")`.

**Updated `current_backend()` return values:**

```
windows          → "direct2d"
apple            → "metal"
linux/bsd        → "cairo"
everything else  → "skia"
```

### 3. New `render_with_backend` API

A new public API allows callers to explicitly choose a paint backend:

```rust
pub fn render_with_backend(
    data: &str,
    options: Option<&Options>,
    backend: &str,
) -> Result<PixelContainer, String>

pub fn render_with_backend_for_symbology(
    symbology: &str,
    data: &str,
    options: Option<&Options>,
    backend: &str,
) -> Result<PixelContainer, String>

pub fn render_png_with_backend(
    data: &str,
    options: Option<&Options>,
    backend: &str,
) -> Result<Vec<u8>, String>
```

Supported `backend` names:
- `"skia"` — Skia CPU raster, all platforms
- `"cairo"` — Cairo vector raster, macOS + Linux + BSD
- `"metal"` — Metal GPU, macOS/iOS only (panics on non-Apple if miscalled)
- `"direct2d"` — Direct2D GPU, Windows only

Unsupported backends return `Err(String)` rather than panicking.

### 4. Barcode-2D note

`barcode-2d::layout()` returns a `PaintScene` that can be rendered through
`paint_vm_skia::render()` or any other backend directly. The same backend
priority applies for QR codes.

## Test coverage

New tests added to `barcode-1d`:

| Test | Platforms |
|------|-----------|
| `skia_renders_all_symbologies` | All |
| `cairo_renders_all_symbologies` | macOS, Linux, BSD |
| `primary_backend_renders_png` | All (replaces platform-gated test) |

The old `render_png_returns_bytes` and `render_png_for_symbology_accepts_string_input`
tests remain gated to Windows and Apple because they exercise the platform-default
backend path which still requires the native windowing stack.

The old `render_pixels_is_honest_when_backend_is_missing` test is removed; it
was accurate before this change but is now incorrect since Linux has Cairo
wired.

## Files Changed

- `code/packages/rust/paint-vm-cairo/Cargo.toml` — add macOS to Cairo dep target
- `code/packages/rust/paint-vm-cairo/src/lib.rs` — extend `cfg` predicates to include `macos`
- `code/packages/rust/barcode-1d/Cargo.toml` — add `paint-vm-skia` + `paint-vm-cairo` deps
- `code/packages/rust/barcode-1d/src/lib.rs` — wire backends, add `render_with_backend` API
- `code/specs/P2D10-paint-vm-cairo-skia-barcode.md` — this document
