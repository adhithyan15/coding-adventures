# code39 (Lua)

Code 39 barcode encoder — normalize, encode, expand runs, render SVG.

## Usage

```lua
local code39 = require("coding_adventures.code39")

-- Encode a string
local encoded = code39.encode_code39("HELLO")
-- Returns: { {char="*",...}, {char="H",...}, {char="E",...}, ..., {char="*",...} }

-- Expand into bar/space runs
local runs = code39.expand_code39_runs("HELLO")
-- Each run: { color="bar"|"space", width="narrow"|"wide", ... }

-- Draw as SVG
local scene = code39.draw_code39("HELLO")
```

## Dependencies

None — self-contained. Optionally uses `coding_adventures.draw_instructions` if available.
