# coding-adventures-window-native

Rust-backed native window creation for Python via the repo's zero-dependency
`python-bridge`.

## What It Provides

- `create_window(...)` for the first native Python window smoke path
- `Window` wrapper methods for id, sizes, scale factor, title, visibility, and
  redraw requests
- shared window concepts like `SurfacePreference`, `RenderTargetKind`,
  `LogicalSize`, and `PhysicalSize`
- AppKit-backed native creation on macOS in the first slice

## Scope

This package is the first bridge-backed Python window package. It deliberately
starts small:

- macOS AppKit window creation works
- non-macOS imports succeed, but creation raises `WindowError`
- event pumping is not exposed yet

## Example

```python
from window_native import SurfacePreference, create_window

window = create_window(
    title="Python Native Window",
    width=480,
    height=320,
    preferred_surface=SurfacePreference.METAL,
    visible=False,
)

print(window.id(), window.render_target_kind())
window.close()
```
