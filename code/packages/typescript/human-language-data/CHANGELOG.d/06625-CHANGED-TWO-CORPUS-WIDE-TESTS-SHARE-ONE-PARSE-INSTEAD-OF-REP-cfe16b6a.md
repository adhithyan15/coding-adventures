### Changed — two corpus-wide tests share one parse instead of repeating it

`tests/modality-manifest.test.ts` now parses the real curriculum once, in a `beforeAll` hook, and builds its modality manifest once. Every test that only reads them shares the result. `tests/chapter-intro.test.ts` renders the books once, at import, instead of a second time inside its first test.

Two tests had crossed the 30s budget under full-suite load on CI (the Telugu A1 PR): "keeps the committed manifest in step with the lessons" and "is present on every generated chapter that has a capability". Both passed in 16-19s in isolation. The tranche that tipped them over was small; the cause was repeated whole-corpus work, which grows with every content PR. The budget in `vitest.config.ts` is unchanged.

Local timings, in isolation:
- committed-manifest check: 18.8s -> 10.8s
- chapter-opening check: 16.0s -> 0.05s
- core-vs-whole modality check: 14.7s -> 0.3s
- delivery-marker check: 18.4s -> 0.03s
