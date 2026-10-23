---
category: Testing & coverage
---

# A branch that edits a script inventory must also run the script-ductus tests

The Malayalam B1 core added `strokeOrderSource` records (Jayasree recorded
strokes) to six rows of `data/scripts/malayalam.json`: ഈ ഐ ഔ ഋ, ൗ and ഃ.
Every human-language-data test passed: the full suite, validate, and the
strict book compile. But `code/packages/typescript/script-ductus` has a
second invariant. In tests/strokes.test.ts, every inventory row that
claims a stroke-order source or a lift count must have a matching ductus
(font-checked stroke geometry) citing the same source. That package's
suite is not part of human-language-data's, so the failure (718 expected,
724 verified claims) would only have shown in CI.

The fix kept the six rows but dropped the stroke claims. The glyph-gap
queue now pins the four vowels as one gap each, which is honest, and the
letter lessons describe the shapes without claiming a verified order.
The Telugu B1 core had already taken that route for its own new vowels.

What to do differently: any branch that touches `data/scripts/*.json` or
`data/ductus/**` runs `cd code/packages/typescript/script-ductus && npx
vitest run` before pushing. Add stroke-order sources only together with
the ductus geometry they promise.
