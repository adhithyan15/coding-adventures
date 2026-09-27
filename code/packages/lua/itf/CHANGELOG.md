# Changelog — itf (Lua)

## Unreleased

- Add shared fixture conformance, scalar-aware preflight, and stable error IDs.

## 0.1.0 — 2026-04-13

Initial release.

- `normalize_itf` — even-length digit validation
- `encode_itf` — digit-pair interleaving into run sequences
- `expand_itf_runs` — bar/space run expansion with framing patterns
- `draw_itf` — PaintScene generation through `barcode-layout-1d`
