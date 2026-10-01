# barcode-layout-1d

Pure 1D barcode layout that converts barcode runs into `PaintScene`.

The strict `*_v1` entry points implement the shared 56-case
`barcode-layout-1d-v1` contract with bounded integer arithmetic, Unicode
scalar validation, stable error IDs, inferred and explicit symbol spans,
canonical string metadata, fresh result ownership, and rectangle-only scene
projection. The original methods remain available for downstream source
compatibility.

Both human-readable text request forms are rejected before geometry or native
resolution. Production has an empty capability manifest and no font or
shaping dependency; JSON loading and SHA-256 are confined to tests.

## Development

```bash
bash BUILD
```
