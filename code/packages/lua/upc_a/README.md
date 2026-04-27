# upc-a (Lua)

Dependency-free UPC-A encoder that emits backend-neutral paint scenes.

## Usage

```lua
local upc_a = require("coding_adventures.upc_a")

local scene = upc_a.draw_upc_a("03600029145")
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
