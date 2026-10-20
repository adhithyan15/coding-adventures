# Generated figures

Generate configured SVG lesson figures, or verify both their canonical inputs and
committed bytes are current:

```bash
npm run build
npm run generate:figures
npm run check:figures
```

`core/figure-generation.json` binds each figure to one canonical lesson and a safe
`<track>/book/figures/*.svg` target. `core/generated-figure-hashes.d/` fingerprints
the figure-driving source subset and the exact SVG separately, so either a stale
claim or an edited artifact fails `--check`.
