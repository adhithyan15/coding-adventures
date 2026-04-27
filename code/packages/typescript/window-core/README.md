# @coding-adventures/window-core

TypeScript mirror of the repository's shared windowing contract.

## What It Contains

- `WindowId`, `LogicalSize`, and `PhysicalSize`
- `SurfacePreference` and `MountTarget`
- `WindowAttributes` plus a fluent `WindowBuilder`
- normalized `WindowEvent` values
- renderer-facing `RenderTarget` tags
- `WindowHandle` and `WindowBackend` interfaces

This package is intentionally backend-neutral. It does not create DOM nodes or
native windows by itself. It gives browser and native adapters one consistent
type system.

## Usage

```typescript
import {
  LogicalSize,
  MountTargets,
  SurfacePreference,
  WindowBuilder,
} from "@coding-adventures/window-core";

const attributes = new WindowBuilder()
  .title("Browser Surface")
  .initialSize(new LogicalSize(800, 600))
  .mountTarget(MountTargets.browserBody())
  .preferredSurface(SurfacePreference.Canvas2D)
  .build();
```
