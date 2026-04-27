# code128 (Lua)

Dependency-free Code 128 encoder that emits backend-neutral paint scenes.

## Usage

```lua
local code128 = require("coding_adventures.code128")

local scene = code128.draw_code128("HELLO-123")
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
