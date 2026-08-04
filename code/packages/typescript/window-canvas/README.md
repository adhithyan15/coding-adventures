# @coding-adventures/window-canvas

Pure TypeScript browser window backend for the shared window-core contract.

## What It Does

`window-canvas` treats a mounted `<canvas>` as a window-like presentation host.
It gives renderers a stable place to draw while normalizing browser events into
the repository's `WindowEvent` model.

## What It Handles

- mount lookup via `BrowserBody`, element id, or query selector
- CSS logical size versus backing-store physical size
- `requestAnimationFrame` as `redraw-requested`
- pointer, wheel, keyboard, focus, and visibility events
- a browser render target carrying mount metadata plus size state

## Build Output

`npm run build` type-checks the package and writes JavaScript plus declarations
under `dist`. The package-root layout is preserved (`dist/src`, `dist/tests`,
and `dist/vitest.config.*`) so generated files never appear beside tracked
TypeScript sources. Vitest excludes `dist`, preventing compiled test copies
from running or affecting source coverage.

## Usage

```typescript
import { LogicalSize, MountTargets, SurfacePreference, WindowBuilder } from "@coding-adventures/window-core";
import { CanvasBackend } from "@coding-adventures/window-canvas";

const backend = new CanvasBackend();
const windowHandle = new WindowBuilder()
  .title("Canvas Window")
  .initialSize(new LogicalSize(800, 600))
  .mountTarget(MountTargets.browserBody())
  .preferredSurface(SurfacePreference.Canvas2D)
  .buildWith(backend);

windowHandle.requestRedraw();
```
