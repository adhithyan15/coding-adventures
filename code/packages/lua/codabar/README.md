# codabar (Lua)

Dependency-free Codabar encoder that emits backend-neutral paint scenes.

## Usage

```lua
local codabar = require("coding_adventures.codabar")

local scene = codabar.draw_codabar("A1234B")
```

This package stops at `PaintScene` so the same barcode logic can feed native
Paint VMs, Canvas, SVG, or future codecs.
