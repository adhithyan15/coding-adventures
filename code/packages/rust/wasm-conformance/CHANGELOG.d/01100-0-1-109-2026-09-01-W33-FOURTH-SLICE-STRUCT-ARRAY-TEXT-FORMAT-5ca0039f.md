## 0.1.109 — 2026-09-01 — W33 fourth slice: struct/array TEXT-format grammar + runtime semantics

Regenerated the baseline after `wasm-types` 0.1.17, `wasm-wast-parser`
0.1.91, `wasm-validator` 0.2.79, `wasm-execution` 0.9.82, and
`wasm-runtime` 0.6.19 (struct/array TEXT-format parsing, static
checking, and runtime semantics). Exactly 4 files changed in the full
257-file corpus; every other file byte-for-byte identical.

**`struct.wast`**: `module` 0/6 (not yet supported) → 6/6 (100%);
`assert_return` 0/17 (nys) → 17/17 (100%); `assert_trap` 0/2 (nys) →
2/2 (100%); `assert_malformed` unchanged 1/1 (100%, the "duplicate
field" case). `assert_invalid` 4/4 (100%) → 3/3 (100%) + 1 not yet
supported — an honest reclassification: this file's own field-name
`(struct.get 0 $x ...)` "type mismatch" case needs an instruction-level
check (the func's declared result type vs. the field's REAL type) this
crate has never had for ANY opcode, not something struct/array
introduced — previously this case was invisible ("Pass" only because
the whole module failed to PARSE, which trivially satisfies
`assert_invalid`); now it parses for real and honestly reports what it
can't check.

**`array.wast`**: `module` 0/7 (nys) → 4/4 (100%) + 3 nys;
`assert_return` 0/24 (nys) → 10/10 (100%) + 14 nys; `assert_trap` 0/17
(nys) → 6/6 (100%) + 11 nys; `assert_invalid` unchanged 6/6 (100%). The
remaining not-yet-supported directives are `array.new_data`/
`array.new_elem`'s own sections — deliberately not implemented this
slice (need real data-/elem-segment integration), confirmed via direct
inspection, not assumed.

**`type-rec.wast`** and **`type-subtyping.wast`**: struct/array grammar
unlocks parsing further into BOTH files (they interleave func/struct
types inside the same `rec` groups extensively), surfacing a small
number of new, individually-traced fails — every one attributable to
either of two PRE-EXISTING, already-documented gaps, now newly
REACHABLE rather than newly broken:
- **(3b), cross-module/same-module canonical type-group equivalence**
  (`code/specs/W33-wasm-gc-recursive-type-subtyping.md`'s own
  first-slice addendum already named this the real remaining boundary):
  `type-rec.wast`'s own "Static matching of recursive types"/"Dynamic
  matching of recursive types" sections and `type-subtyping.wast`'s own
  "Linking" section BOTH depend on recognizing two independently-
  declared, structurally-identical `rec` groups as the SAME type — this
  crate's nominal-only `sub`-chain check (correct within its own scope)
  cannot do that by construction, exactly as documented three addenda
  ago.
- **No instruction-level type checker for struct/array's own precise
  result types**: `struct.get`/etc. push `StackType::Unknown` for the
  read value (matching this crate's pre-existing, long-standing
  convention for every not-fully-modeled opcode, e.g. `ref.cast`), so a
  case that needs the checker to notice a DECLARED type disagreeing
  with the field's real type reports `NotYetSupported`, not a false
  pass — this is the SAME class of limitation already tracked
  throughout this crate (`code/specs/W02-wasm-validator.md`), not new.

Per-file tallies, `type-rec.wast`: `module` 2/2(100%)+9nys → 9/11(81.8%)
[+7 real passes, +2 new fails (3b)]; `register` 0/1(nys)→1/1(100%)
[+1]; `assert_return` 0/1(nys)→0/1(0%) [1 new fail (3b — canonical
equivalence needed for `call_indirect`/`ref.func` against
structurally-but-not-nominally-equal rec groups)]; `assert_trap`
0/2(nys)→2/2(100%) [+2]; `assert_invalid` 9/9(100%)+1nys→7/7(100%)+3nys
[2 more honest reclassifications, same instruction-level-checker gap as
`struct.wast`'s own case above].

`type-subtyping.wast`: `module` 12/12(100%)+34nys→21/22(95.5%)+24nys
[+9 real passes, +1 new fail (3b)]; `register` 4/4(100%)+7nys→7/7(100%)+4nys
[+3]; `assert_return` 10/10(100%)+7nys→12/13(92.3%)+4nys [+2 real
passes, +1 new fail (3b — the "Subsumption" 3-cycle section)];
`assert_unlinkable` 6/8(75%)→5/8(62.5%) [1 new fail — a THIRD "Linking"
section case needing (3b), alongside the 2 (M10/M11) already documented
in the first-slice addendum].

Zero other files in the 257-file corpus changed. Verified via a
programmatic full-baseline diff, not spot-checked.

