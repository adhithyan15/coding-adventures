# code39 (Lua)

Dependency-free Code 39 encoder that emits backend-neutral paint scenes.

## Usage

```lua
local code39 = require("coding_adventures.code39")

local scene = code39.draw_code39("HELLO")
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
