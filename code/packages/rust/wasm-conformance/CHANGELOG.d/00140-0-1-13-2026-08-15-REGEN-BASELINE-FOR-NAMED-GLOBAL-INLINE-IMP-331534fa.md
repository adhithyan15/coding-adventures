## 0.1.13 — 2026-08-15 — regen baseline for named global inline-import fix (WASM19)

Regenerates the baseline against `wasm-wast-parser` 0.1.8's fix for the
named global inline-import shorthand. No vendored-file-list change (same
pinned commit). Aggregate tallies and every already-parsing file's
pass/fail counts are byte-for-byte UNCHANGED (verified via a full
structured diff) -- the only change is `global.wast`'s `parse_failures`
entry moving further, from `at byte 17077: ... found "$g0"` to `at byte
17335: ... found "funcref"`. That new failure point is the SAME extended
active-`elem`-segment syntax (`(elem (table $t) (global.get $g2) funcref
(ref.func $f))`) that already blocks `call_indirect.wast`, and is already
tracked as out-of-scope in `code/specs/W08-wasm-funcref-externref.md`.

