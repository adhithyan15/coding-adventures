# barcode_layout_1d

Pure 1D barcode layout that converts barcode runs into `PaintScene`.

The original Ruby helpers remain available for existing barcode packages. The
strict `barcode-layout-1d-v1` facade adds:

- `expand_binary_v1` and `expand_width_v1` for bounded run expansion;
- `compute_layout_v1` for deterministic module and symbol geometry; and
- `project_scene_v1` for portable rectangle-only paint scenes.

The v1 facade uses stable error identifiers, checked limits, bounded metadata,
fresh output values, and rejects human-readable text before any native
resolver can run. Its behavior is verified against the shared 56-case corpus
in `code/specs/fixtures/barcode-layout-1d-v1`.

## Development

```bash
bash BUILD
```
