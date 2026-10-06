# BarcodeLayout1D

`BarcodeLayout1D` converts one-dimensional barcode runs into backend-neutral
`PaintScene` rectangles.

The package preserves its original Swift API and adds a strict
`barcode-layout-1d-v1` facade:

- `expandBinaryV1` and `expandWidthV1` perform bounded run expansion;
- `computeLayoutV1` produces deterministic module and symbol geometry; and
- `projectSceneV1` creates portable rectangle-only scenes.

The v1 facade has stable error identifiers, checked limits, bounded metadata,
value-copy ownership, and rejects text before native resolution. Its dynamic
conformance test consumes all 56 cases in the shared language-neutral corpus.

## Development

```bash
bash BUILD
```
