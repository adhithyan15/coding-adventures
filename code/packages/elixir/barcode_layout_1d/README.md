# barcode_layout_1d

Pure 1D barcode layout that converts barcode runs into `PaintScene`.

The original functions remain available for existing encoders. The strict,
portable `barcode-layout-1d-v1` entry points add bounded binary and width
expansion, deterministic symbol spans, and rectangle-only scene projection:

```elixir
alias CodingAdventures.BarcodeLayout1D, as: Layout

runs = Layout.runs_from_binary_pattern_v1("1100",
  source_label: "A", source_index: 0, role: "data")
geometry = Layout.compute_barcode_1d_layout_v1(runs, 10)
scene = Layout.project_barcode_1d_scene_v1(runs, 10)
```

The v1 functions raise `CodingAdventures.BarcodeLayout1D.V1Error` with stable
error identifiers. They reject human-readable text before invoking any native
font resolver. The native ExUnit test consumes all 56 cases in the shared
`code/specs/fixtures/barcode-layout-1d-v1` corpus and verifies this boundary.

## Development

```bash
bash BUILD
```
