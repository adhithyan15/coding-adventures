# itf (Lua)

Dependency-free ITF encoder that emits backend-neutral paint scenes.

## Usage

```lua
local itf = require("coding_adventures.itf")

local scene = itf.draw_itf("123456")
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.

Validation follows `barcode-symbologies-v1`; `error_id` maps checked failures
to stable language-neutral identifiers.
