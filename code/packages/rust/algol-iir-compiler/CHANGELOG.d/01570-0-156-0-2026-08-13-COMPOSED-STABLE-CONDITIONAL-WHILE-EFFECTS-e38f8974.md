## 0.156.0 — 2026-08-13 — composed stable conditional while effects

Capped `while` dependency analysis now selects a body branch controlled by a
statically known boolean composition when every referenced selector is a local
bare boolean that the whole body never writes. Mixed, written, nonlocal, array,
by-name, and otherwise unknown selector sets remain conservative.

