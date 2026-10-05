# barcode_layout_1d

Pure 1D barcode layout that converts barcode runs into `PaintScene`.

The original functions remain available for existing encoders. The strict,
portable `barcode-layout-1d-v1` entry points add bounded binary and width
expansion, deterministic symbol spans, and rectangle-only scene projection:

```lua
local layout = require("coding_adventures.barcode_layout_1d")
local runs = layout.runs_from_binary_pattern_v1("1100", {
    source_label = "A", source_index = 0, role = "data",
})
local geometry = layout.compute_barcode_1d_layout_v1(runs, 10)
local scene = layout.project_barcode_1d_scene_v1(runs, 10)
```

The v1 functions raise payload-blind error identifiers. They reject
human-readable text before invoking any native font resolver. The native Lua
test consumes all 56 cases in the shared
`code/specs/fixtures/barcode-layout-1d-v1` corpus and verifies this boundary.
`dkjson` is used only by the test boundary, not production geometry.

## Development

```bash
bash BUILD
```
