# ean-13 (Lua)

Dependency-free EAN-13 encoder that emits backend-neutral paint scenes.

## Usage

```lua
local ean_13 = require("coding_adventures.ean_13")

local scene = ean_13.draw_ean_13("400638133393")
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
