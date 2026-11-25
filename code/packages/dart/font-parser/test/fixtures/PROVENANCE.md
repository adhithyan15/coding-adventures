# `stix-two-math.MATH`

A byte-identical copy of
`code/packages/rust/font-parser/tests/fixtures/stix-two-math.MATH`: the raw
OpenType `MATH` table extracted from STIX Two Math, checked against value
sets produced by **fontTools** (an independent implementation). See the
`PROVENANCE.md` beside the original for the source, the licence, and the
command that regenerates the expected numbers.

## Why a copy rather than a cross-tree read

Dart lane tests read shared inputs from `code/fixtures/` (as with
`Inter-Regular.ttf`); no lane reads another language's test fixtures, so
this lane carries its own copy instead. The bytes are static — extracted
once from a real font — so drift is a non-issue; `cmp` against the Rust
original reproduces the check. The copy keeps the anti-circularity
property intact: real bytes plus someone else's expected values.

## Licence notice (travels with the extract per the OFL)

- **Source:** `STIXTwoMath.otf` (STIX Two Math)
- **Upstream:** https://github.com/stipub/stixfonts
- **Copyright:** Copyright 2001-2021 The STIX Fonts Project Authors
- **Licence:** SIL Open Font License, Version 1.1 —
  https://scripts.sil.org/OFL
