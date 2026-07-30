# Changelog

All notable changes to `cobol-iir-compiler` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/); this
crate predates any release, so everything lives under Unreleased until the first
tag.

## [Unreleased]

### Added — v0.68.0: combined INSPECT TALLYING/REPLACING — a LEADING half may carry a region

`INSPECT src TALLYING c FOR {ALL|LEADING} d [{BEFORE|AFTER} p] REPLACING {ALL|LEADING} s BY r
[{BEFORE|AFTER} q]` now lowers when a **LEADING** half (tally and/or replace) ALSO carries a
`{BEFORE|AFTER}` region. Compiled byte-identical to the `coding-adventures-cobol-runtime` 0.72.0 oracle.

- **A reject-lift, not new machinery.** Both `emit_inspect_tallying` and `emit_inspect_replacing`
  ALREADY implement the LEADING+region lowering (the leading run ANCHORED at the window start), used by
  the standalone paths. The combined `(true, true)` dispatch arm previously passed
  `allow_leading_region = false` to both, deferring the combination; it now passes `true`, enabling the
  exact same byte-identical lowering. The tally still emits FIRST over the original bytes, the replace
  SECOND over the same original bytes (its window is derived before the unroll overwrites the source),
  so ISO tally-then-replace ordering holds.
- **Guards retained honestly.** Every caller of `emit_inspect_tallying`/`emit_inspect_replacing` now
  passes `allow_leading_region = true`, so the `!allow_leading_region` later-rung guards no longer
  fire; they are kept so the gate reads uniformly with the sibling later-rung guards (an always-true
  bool param read in a guard is not a clippy error, and the param stays meaningful documentation).
- **Co-totality.** Combined `FOR CHARACTERS` and a multi-character region delimiter remain later rungs,
  rejected identically to the oracle. The non-ASCII disposition is unchanged: TALLYING counts are
  byte-clean, and the REPLACING half's per-position reconstruction still traps on a multi-byte source —
  the pre-existing byte-vs-char chip shared by every REPLACING lowering, not a new divergence.

### Added — v0.67.0: INSPECT CONVERTING with a data-name FROM/TO operand

`INSPECT src CONVERTING from TO to [{BEFORE|AFTER} x]` now lowers when the `from` and/or `to` table is
a **data-name** (`PIC X` item) rather than a string literal — either or both sides may be an item,
mixing freely with a literal on the other. Compiled byte-identical to the `coding-adventures-cobol-runtime`
0.71.0 oracle on ASCII operands.

- **Operand kind.** A new `ConvOperand { Literal(String), Item { reg, width } }` mirrors the oracle's
  `ConvertOperand`. `emit_inspect_converting` resolves each `from`/`to` node via a `converting_operand`
  method (a literal is carried by value; a data-name resolves to its register + declared width through
  the shared `item_index`; a numeric item, group/undeclared name, figurative, numeric literal, and
  reference modification are clean later rungs with the same messages the oracle uses).
- **Table entries: baked const vs loop-invariant runtime read.** The equal-length check spans each
  side's compile-time length (a literal's char count OR an item's declared width). For a LITERAL the
  table entries are baked exactly as before — a `const`(byte) per `from[k]`, a 1-char `str_const` per
  `to[k]`. For a DATA-NAME they become RUNTIME reads emitted ONCE before the per-position loop:
  `from[k] = str_index(item, k)` (a byte) and `to[k] = str_slice(item, k, k+1)` (a 1-char string).
  These reads are loop-invariant (the `from`/`to` item does not change during the translate), so
  hoisting them out is both the natural lowering and the correctness invariant — a `from`/`to` that
  ALIASES the source is read while the source still holds its ORIGINAL bytes (the source register is
  overwritten only at the very end). The per-position first-match-wins chain and the `{BEFORE|AFTER}`
  region guard are BYTE-IDENTICAL to the literal path — they consume the `from_consts`/`to_consts`
  registers the same way regardless of how the entries were produced.
- **Byte-vs-char note.** A non-ASCII LITERAL `from`/`to` stays rejected (the compiler compares raw
  bytes). A non-ASCII byte in a data-name item's runtime storage is the pre-existing byte-vs-char
  operand chip (shared with the literal-source scans) — not statically rejectable — so the ASCII case
  is byte-identical and non-ASCII item content stays that shared chip.

### Added — v0.66.0: INSPECT REPLACING multi-item list with a LEADING item

A multi-item `INSPECT src REPLACING {ALL|LEADING} a BY x {ALL|LEADING} b BY y …` now lowers when one
(or more) of the items is `LEADING` — the REPLACING twin of v0.65.0's TALLYING multi-item-with-LEADING
rung (#65). The `LEADING` reject inside a multi-item REPLACING list is LIFTED; compiled byte-identical
to the `coding-adventures-cobol-runtime` 0.70.0 oracle on ASCII sources.

- **Mirror of the tally active-flag machine.** `emit_inspect_replacing_multi` gains the SAME per-item
  `active` run-flag machine as `emit_inspect_tally_multi`: one i64 `active` register per item (init 1,
  consulted only for `LEADING` items). The ONLY difference from the tally side is that the decision
  loop EMITS the winning item's replacement string instead of bumping a counter (and keeps the ORIGINAL
  char on no match). At each position the FIRST eligible item in written order wins — an `ALL` item is
  eligible iff `(start ≤ j < end) AND c == search`; a `LEADING` item ALSO requires its `active` flag
  still 1.
- **Run-update independent of the winner.** AFTER the per-position `done` convergence label (both a
  match and the no-match fall-through reach it), EVERY `LEADING` item's `active` is decayed
  `active := active AND (eq OR NOT in_win)` — a run breaks at the FIRST in-window mismatch, a matching
  char keeps it alive even if a higher-priority item claimed the position, and positions OUTSIDE the
  window never touch `active` (anchoring the run at the window start). `eq`/`in_win` are RECOMPUTED per
  leading item in the update section (the chain's registers may not have been reached on an early
  `jmp done`), exactly as the tally side's `cont` section does.
- **No new scope.** Each item is a single-char `{ALL|LEADING} search BY replace` pair with an OPTIONAL
  `{BEFORE|AFTER}` region; `CHARACTERS`/`FIRST` items and the combined `TALLYING … REPLACING` with
  several items stay later rungs (rejected identically on both engines). A
  multi-character/figurative/wider/numeric/reference-modified search/replace/region delimiter still
  falls to the shared `single_delim_code`/`single_delim_str` check.
- **Byte-vs-char.** REPLACING RECONSTRUCTS the source with per-position byte `str_slice`, so a
  non-ASCII source is the PRE-EXISTING byte-vs-char reconstruction chip (task_396ba6f6) shared by every
  REPLACING lowering — the byte-based compiler traps on a multi-byte source, exactly as the merged
  single-item and multi-item ALL paths do. This rung introduces NO new non-ASCII divergence.
- **Types/reader.** `ReplaceItem` gains a `leading` bool; `inspect_replacing_multi` reads the `LEADING`
  keyword per item (mirroring `inspect_tally_multi`); a new `ResolvedReplaceLeadingItem` alias carries
  the search/replace registers plus `leading`/`active`/`window`.
- **Tests.** The old "several items and a LEADING item is a later rung" reject test is converted to a
  now-supported positive; added first-match-priority (LEADING vs ALL same delim), a LEADING+region
  anchored run, two independent LEADING runs (immediate break, and disjoint-window both-fire), the
  run-breaks-on-higher-priority-claim subtlety, single-item + ALL-only regressions, a FILLER-between
  cross-producer-binding parity test, and a non-ASCII characterization test pinning the shared chip.

### Added — v0.65.0: alphanumeric → SIGNED numeric MOVE (completes the Char↔Numeric MOVE matrix)

A cross-category `MOVE <alphanumeric> TO <signed-numeric>` (`MOVE A TO N` where `A` is `PIC X(m)` and
`N` is `PIC S9(i)V9(d)`) now lowers, compiled byte-identical to the
`coding-adventures-cobol-runtime` 0.69.0 oracle. This was the ONLY remaining unhandled cell of the
Char↔Numeric × signed/unsigned MOVE matrix — the matrix is now **complete** (both directions, both
signednesses).

- **Semantics.** An alphanumeric source carries NO operational sign — COBOL does not read an overpunch
  from a plain `PIC X` source — so the receiver stores the folded MAGNITUDE and its sign is ALWAYS
  POSITIVE. The fold (`V = V*10 + (byte - '0')` left→right) and scale placement (`V mod 10^(i+d)` at
  scale `d`) are IDENTICAL to the already-shipped unsigned-receiver path. DISPLAY of the signed field
  overpunches the units digit on its POSITIVE row (`{A…I` for units 0-9), so `MOVE "123" TO S9(3)`
  shows `12C`, `MOVE "120"` shows `12{`.
- **Guard relaxation.** The cross-category arm's pattern dropped its `signed: false` constraint —
  `(ItemKind::Char, ItemKind::Numeric { dec_digits: d, .. })` — so it accepts any numeric receiver.
- **Positive store via existing signed-aware path.** After the byte fold we `emit_abs` the value
  before `store_scaled`, mirroring the oracle's `unsigned_abs()`. This is the crux: a source byte
  below `'0'` (a SPACE in an uninitialised field) makes the raw fold NEGATIVE, and `store_scaled`
  re-applies the sign of the value it is handed (`reapply_sign`) for a signed receiver — which would
  wrongly store a negative value. Absing first makes `reapply_sign` a genuine no-op → a POSITIVE value
  is stored, byte-identical to the oracle. For an all-digit source the fold is already non-negative, so
  the unsigned path's output is unchanged.
- **Exhaustive match.** With both cross-category directions and both signednesses handled, the four
  `(kind, kind)` arms are now exhaustive over `ItemKind`'s two variants, so the former catch-all `_`
  reject arm was removed (keeping it would fire an `unreachable_patterns` warning, which CI denies). A
  future third item kind would now fail to compile and force this logic to be revisited deliberately.
- **Scope.** The ≤18-character source-width guard (i64 fold) is unchanged. The non-ASCII byte-vs-char
  behaviour of `emit_str_to_int` (folds the item's `width()` = char-count leading bytes, vs the
  oracle's full-byte fold) is PRE-EXISTING and shared with the unsigned path — this rung does not touch
  it (a dedicated jit_e2e test documents that the unsigned path diverges identically).

### Added — v0.64.0: alphanumeric level-88 with a THRU range

A level-88 condition-name on an **alphanumeric** (`PIC X`) conditional variable now also lowers for an
inclusive `THRU` **range whose bounds are string literals** (`88 PASSING VALUE "A" THRU "D"`), in BOTH
directions, compiled byte-identical to the `coding-adventures-cobol-runtime` 0.68.0 oracle. This is
the deferred half of v0.63.0 (#68), which lowered only discrete-string VALUEs; the range case
(`88 X VALUE "A" THRU "Z"`) was rejected up front and is lifted here. No grammar change — the same
rules the numeric level-88 range uses already parse this.

- Generalized predicate (`all_single_str` → `all_str_values`): the accept predicate now holds when
  every VALUE item is a string `Single(Src::Str)` OR a `Range(Src::Str, Src::Str)` (BOTH bounds string
  literals) — logically IDENTICAL to the oracle's, so both engines accept and reject the very same
  programs.
- Read (`emit_condition_name`): a string range `lo THRU hi` lowers to
  `and(cmp_ge(var, lo), cmp_le(var, hi))` over the SAME alphanumeric `str_cmp` path
  (`emit_str_condition`) an `IF var >= "…"` / `IF var <= "…"` relation runs — reusing the exact
  `str_const` + fixed `StrOperand` subject the discrete-string equality builds, and the same `and` the
  numeric range's `emit_value_test` emits. Range and discrete results OR-fold with `or`, mirroring the
  numeric fold exactly.
- Set (`emit_set`): when the first VALUE item is a range `lo THRU _`, stores its LOW bound `lo` — the
  string fit to the receiver width by `format_into_picture` and emitted as the slot's `str_const`,
  mirroring the numeric SET.
- Scope / boundary (co-total with the oracle): a range with a NON-string bound (`88 X VALUE "A" THRU
  5`), a numeric/figurative VALUE, or a mixed list on an alphanumeric 88 stays a later rung — rejected
  UP FRONT (before any dead instructions) IDENTICALLY to the oracle. The FILLER-88 reject from v0.63.0
  still holds for a range 88, and the numeric level-88 paths (single + range) are unchanged. Byte-vs-
  char is ASCII-clean; a non-ASCII bound is the pre-existing alphanumeric byte-vs-char behavior.

### Added — v0.63.0: level-88 condition-name on an alphanumeric item (read + SET TO TRUE)

A level-88 condition-name declared on an **alphanumeric** (`PIC X`) conditional variable now lowers in
BOTH directions — reading it (`IF IS-YES`) and setting it (`SET IS-YES TO TRUE`) — for the
**discrete-string VALUE** case, compiled byte-identical to the `coding-adventures-cobol-runtime`
0.67.0 oracle. Previously both paths rejected at emit time ("a level-88 condition-name on an
alphanumeric item is a later rung" / "SET … TO TRUE on an alphanumeric conditional variable is a later
rung"). This mirrors the numeric level-88 already shipped.

- Read (`emit_condition_name`): when every VALUE item is a discrete string literal, each value becomes
  a `cmp_eq` over the SAME alphanumeric `str_cmp` path (`emit_str_condition`) an `IF var = "…"`
  relation runs — the variable's slot against the value's `str_const`, space-padded to a common
  width — and the value-list OR-folds with `or`, mirroring the numeric `emit_value_test` fold exactly.
- Set (`emit_set`): stores the FIRST value into the slot exactly as `MOVE "…" TO item` — the string is
  fit to the receiver width by `format_into_picture` and emitted as the slot's `str_const`, the same
  const-into-slot store the MOVE-literal path emits.
- Scope / boundary (co-total with the oracle): accepted iff the variable is a `Char` item AND every
  VALUE item is a discrete string (`Single(Src::Str)`) — the same `all_single_str` predicate the
  oracle applies. An alphanumeric **THRU range** (`88 X VALUE "A" THRU "Z"`) and a **numeric or
  figurative** VALUE on an alphanumeric 88 (`88 X VALUE 5`) stay later rungs — rejected UP FRONT
  (before any dead instructions) IDENTICALLY to the oracle. The numeric level-88 paths are unchanged.
- No grammar change: the same `value_clause` / `condition_name` / `set_stmt` rules the numeric
  level-88 uses already parse this. Byte-vs-char is ASCII-clean; a non-ASCII value is the pre-existing
  alphanumeric byte-vs-char behavior inherited from the IF-alphanumeric path.
- FILLER guard (co-total): a level-88 whose conditional variable is an **unnamed** (`FILLER`) item is
  now rejected at collect time ("a level-88 condition-name on an unnamed (FILLER) conditional variable
  is a later rung"), matching the oracle's message. The compiler does not push FILLERs to its item
  table, so a FILLER-88's `var = items.len()-1` would otherwise bind to the wrong (last named) item; a
  new `prev_entry_unnamed_filler` flag — set on a FILLER entry, cleared on a NAMED non-88 entry, left
  unchanged on a level-88 — lets `collect_condition_name` reject before `checked_sub`. This closes the
  divergence for BOTH the new alphanumeric AND the pre-existing numeric FILLER-88 case. A level-88
  following a FILLER *and then a named item* still binds to the named item and is accepted.

### Added — v0.62.0: STRING with a reference-modification sending field

`STRING base(start:len) DELIMITED BY … INTO dst` — a reference modification is now accepted as a STRING
**sending field**, compiled byte-identical to the `coding-adventures-cobol-runtime` 0.66.0 oracle.
Previously any refmod STRING sending field was rejected at emit time ("a reference modification as a
STRING sending field is a later rung"); that reject is now LIFTED for **constant (literal) indices**.
No grammar change was needed — the grammar already parses a refmod suffix on the STRING sending
operand.

- Lowering: `string_source` now handles `Operandy::RefMod` by calling the shared `ref_mod_slice` (the
  SAME `str_slice` DISPLAY / comparison / MOVE-source emit, so the slice bytes already agree with the
  oracle), returning `(slice_reg, len)`. Because the STRING image contract is `(reg, usize)` — a
  COMPILE-TIME length — only a `SliceLen::Const` is usable, so literal indices `WS(2:3)` and an omitted
  length `WS(3:)` are supported.
- Boundary: a **computed (data-name) index** would yield a `SliceLen::Runtime` length known only at
  run time, which the `(reg, usize)` contract cannot express. It is rejected UP FRONT — before any
  slice code is emitted (no dead instructions) — with "a computed reference modification as a STRING
  sending field is a later rung", IDENTICALLY to the oracle, keeping the two engines co-total.
- Downstream unchanged: the refmod image is just another char image, so `DELIMITED BY SIZE`,
  `DELIMITED BY <delim>`, the concat/overlay, and `WITH POINTER` consume it exactly as they do a plain
  alphanumeric-item sending field — no special-casing.
- Byte-vs-char: `ref_mod_slice` is byte-based and the oracle's `refmod_string` is char-based; they
  coincide on the ASCII-clean windows this rung targets, so accepted programs emit byte-identical
  output. A multi-byte char inside/after the window is the PRE-EXISTING refmod byte-vs-char chip
  (shared with DISPLAY / MOVE-source), not introduced here — the non-ASCII parity test keeps the
  multi-byte char strictly OUTSIDE the window.

### Added — v0.61.0: MOVE with a reference-modification source

`MOVE base(start:len) TO dst` — a reference modification is now accepted as a MOVE **source** when the
receiver is **alphanumeric**, compiled byte-identical to the `coding-adventures-cobol-runtime` 0.65.0
oracle. Previously any refmod MOVE source was rejected at emit time (`REFMOD_CONTEXT_MSG` — "reference
modification is only supported in DISPLAY and comparison contexts on this rung"); that reject is now
LIFTED for an alphanumeric receiver. No grammar change was needed — the grammar already parses a
refmod suffix on the MOVE source operand.

- Lowering: `ref_mod_slice` emits the SAME `str_slice` DISPLAY/comparison use (so the slice bytes
  already agree with the oracle) and reports its length as a `SliceLen`. The new `move_slice_into_char`
  fits the slice to the receiver's width by the ordinary alphanumeric char rule (left-justify;
  space-pad the tail if wider; truncate on the right if narrower):
  - `SliceLen::Const` (a literal:literal or literal: refmod) defers to `move_str_into_char`, the SAME
    const-width char fit a plain alphanumeric-item MOVE uses.
  - `SliceLen::Runtime` (a computed data-name index) — the slice width is unknown at compile time, so
    a single width-agnostic form is lowered: concat `recv_w` trailing spaces onto the slice (making it
    at least `recv_w` characters for any length `L ≥ 0`), then keep the leftmost `recv_w`. For
    `L ≥ recv_w` that is the slice's first `recv_w` characters (truncate); for `L < recv_w` it is the
    `L` slice characters followed by `recv_w − L` spaces (pad) — exactly `move_into_char`'s two cases,
    so the bytes match the oracle regardless of the run-time length.
- Constant `SRC(2:3)`, omitted-length `SRC(3:)`, and computed `SRC(J:K)` indices are all supported;
  an out-of-range slice traps identically to the oracle. Multiple receivers `MOVE SRC(1:3) TO A B`
  reshape the same slice into each (the existing per-receiver loop in `emit_move`).
- Byte-vs-char: `ref_mod_slice` is byte-based and the oracle's `refmod_string` is char-based; they
  coincide on the ASCII-prefix windows this rung targets, so accepted programs emit byte-identical
  output. A multi-byte char inside/after the window is the PRE-EXISTING refmod byte-vs-char chip
  (shared with DISPLAY/comparison), not introduced here — the non-ASCII parity test keeps the
  multi-byte char strictly OUTSIDE the window (`"abcdé"`, `SRC(1:3)` → `"abc"`).
- Remaining boundary: a **numeric** receiver stays a later rung, rejected on both engines.

### Added — v0.60.0: INSPECT TALLYING multi-item list with a LEADING item

`INSPECT source TALLYING counter FOR {ALL|LEADING} a [{BEFORE|AFTER} p] {ALL|LEADING} b … ` — the
single-counter MULTI-item TALLYING list (two or more `tally_item`s under one `tally_for`) may now MIX
`ALL` and `LEADING` items, each still carrying its own optional `{BEFORE|AFTER}` region, compiled
byte-identical to the `coding-adventures-cobol-runtime` 0.64.0 oracle. Previously any `LEADING` item
in a multi-item list was rejected at emit time ("INSPECT TALLYING with several items and a LEADING
item is a later rung"); that reject is now LIFTED. Only a `CHARACTERS` item in a multi-item list,
SEVERAL counters, and the combined `TALLYING … REPLACING` form with several items remain later rungs.
No grammar change was needed.

- Semantics: ONE runtime left-to-right pass with a per-`LEADING`-item `active` run flag (i64, init
  `1`, allocated before the loop — the runtime-loop analogue of the compile-time-unrolled `active`
  flag in the single-item `emit_inspect_replacing` LEADING lowering). In the tally-decision chain a
  `LEADING` item's eligibility AND-gates on its `active` register (so it counts only while its run is
  alive); at the per-position convergence label — reached by BOTH a tally match (`jmp`) and a no-match
  fall-through — EVERY leading item's run is updated: `active := active AND eq` (region-less) or
  `active := active AND (eq OR NOT in_win)` (windowed, so positions outside the window never touch the
  run, anchoring it at the window start). `eq`/`in_win` are recomputed at the convergence label
  because an early match `jmp` skips the later chain registers. This mirrors the oracle's separate
  active-update pass exactly, so a compiled program matches the tree-walk reference byte-for-byte.
- The `LEADING` run stays alive when a higher-priority item claims a matching char (the active-update
  breaks a run only on an in-window `c != d`, NOT on "this item didn't tally") — verified.
- Non-ASCII-clean POSITIVE parity (NOT a trap): the byte-index scan counts identically to the
  char-index oracle on a non-ASCII source; a `LEADING` run breaks at the multi-byte char's FIRST byte
  (its continuation bytes match nothing). Verified: `"aaébb"` `FOR LEADING "a" ALL "b"` → `4` on both
  engines (`assert_matches_oracle` asserts the DISPLAYed counter is byte-identical).
- `inspect_tally_multi` now returns `Vec<TallyLeadingItem<'_>>` (a new
  `(&GrammarASTNode, bool, Option<(RegionKind, &GrammarASTNode)>)` alias adding the `leading` flag);
  the several-counters reader keeps the `ALL`-only `TallyItem`. The counter must remain an
  unsigned-integer `PIC 9(n)`.
- Tests (jit_e2e via `assert_matches_oracle`): `LEADING … ALL`, `ALL … LEADING` (different delims,
  run breaks at start), `ALL … LEADING`/`LEADING … ALL` same-delim run-survival, a `LEADING` item with
  a region anchored at the window start, two `LEADING` items (same-source and disjoint windows), the
  non-ASCII positive parity, and the still-rejected multi-item `CHARACTERS` item. The obsolete
  multi-item-LEADING reject test was removed.

### Added — v0.59.0: INSPECT TALLYING several counters each item with a BEFORE/AFTER region

`INSPECT source TALLYING c1 FOR ALL a [{BEFORE|AFTER} p] [ALL b …] c2 FOR ALL d [{BEFORE|AFTER} q] …`
— the SEVERAL-COUNTERS TALLYING form (two or more `tally_for` groups) where each `ALL` delimiter item
of ANY group may now carry its OWN optional `{BEFORE|AFTER}` window, compiled byte-identical to the
`coding-adventures-cobol-runtime` 0.63.0 oracle. Previously any region in the multi-counter path was
rejected at emit time ("INSPECT TALLYING with several counters and a BEFORE/AFTER region is a later
rung"); that reject is now LIFTED. No grammar change was needed. This is the multi-COUNTER analogue
of the v0.58.0 single-counter multi-item TALLYING-region rung.

- `inspect_tally_counters` now returns `Vec<TallyCounterGroup>` (`= (String, Vec<TallyItem>)`),
  parsing each item's region with the same keyword/operand extraction the single-item
  `inspect_tally_all` uses. The LEADING/CHARACTERS rejects are UNCHANGED (the path stays `ALL`-only).
- `emit_inspect_tally_counters` materialises `str_len(S)` ONCE up front, then flattens every delimiter
  to a new `FlatCounterDelim` = `(group_index, delim_byte_code, Option<[start,end) window>)` in
  WRITTEN ORDER — deriving each item's window with the SAME `emit_inspect_region_window` the
  single-item region emitter uses, materialised BEFORE the loop. In its runtime `str_len`-bounded
  scan each region-carrying entry's `cmp_eq` link now gates on `start <= j < end AND c == D` against
  the RUNTIME position register `j` (a region-less entry folds to `eq` alone — byte-identical to the
  old lowering). The first in-window match bumps that group's accumulator and jumps to the j-advance
  (first-match-wins across counters); each per-group accumulator is added to its counter afterward,
  re-reading the counter's register fresh so a shared counter accumulates both shares.
- Non-ASCII-clean POSITIVE parity (NOT a trap): TALLYING only COUNTS, so the byte-based scan and the
  char-based oracle count identically even on a non-ASCII source; a new e2e test pins `"aé0b0"` with
  `C1 FOR ALL "0" BEFORE "b"  C2 FOR ALL "0" AFTER "b"` → C1=1, C2=1 on both engines.
- New e2e tests (all via `assert_matches_oracle`): two groups with mixed BEFORE/AFTER regions, a
  region item + a region-less item across groups, an empty AFTER-absent window, an earlier window
  starving a later group, the same counter in two groups each with a region, the non-ASCII positive
  parity, and the still-rejected LEADING / CHARACTERS items.

### Added — v0.58.0: INSPECT TALLYING several items each with a BEFORE/AFTER region

`INSPECT source TALLYING counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|AFTER} q] …` — each
`ALL` delimiter item of a single-counter MULTI-item TALLYING may now carry its OWN optional
`{BEFORE|AFTER}` window, compiled byte-identical to the `coding-adventures-cobol-runtime` 0.62.0
oracle. Previously any region on a multi-item tally list was rejected at emit time ("INSPECT
TALLYING with several items and a BEFORE/AFTER region is a later rung"); that reject is now LIFTED.
No grammar change was needed. This is the count-side analogue of the v0.57.0 multi-item
REPLACING-region rung.

- A new `TallyItem` type carries `(&GrammarASTNode, Option<(RegionKind, &GrammarASTNode)>)` (the
  delimiter node plus its own region node); `inspect_tally_multi` parses each item's region with the
  same keyword/operand extraction the single-item `inspect_tally_all` uses. The LEADING/CHARACTERS
  rejects for a multi-item list are UNCHANGED (the multi path stays `ALL`-only), as is the
  several-counters reject.
- `emit_inspect_tally_multi` materialises `str_len(S)` ONCE up front (the per-item window helper
  needs it, and the loop reuses it for the `j >= len` bound), then reuses `emit_inspect_region_window`
  — the SAME window the single-item region emitter and the REPLACING side emit — to derive each
  item's `[start, end)` before the loop. In its runtime `str_len`-bounded scan loop each
  region-carrying item's `cmp_eq` link now gates on `start <= j < end AND c == D` against the RUNTIME
  position register `j`; a region-less item's link stays `c == D` (the window guard folds away). This
  composes the pre-existing multi-item first-match-per-position count chain with the single-item
  region gate.
- Non-ASCII-clean (a POSITIVE parity, NOT a trap): the tally only COUNTS — it never `str_slice`s the
  source into a new string — and each window is content-defined (bounded by the first ASCII region
  delimiter), so this byte-index scan and the oracle's char-index scan count the SAME ASCII matches
  even on a non-ASCII source. Added a POSITIVE non-ASCII e2e parity test (`assert_matches_oracle`):
  `"aé0b0"` with `ALL "0" BEFORE "b" ALL "0" AFTER "b"` DISPLAYs `002` on both engines. A non-ASCII
  item/region delimiter *operand* stays the pre-existing `single_delim_code` chip.
- Scope kept for a later rung (unchanged, identical messages to the oracle): a `LEADING` or
  `CHARACTERS` item in a multi-item list; SEVERAL counters (more than one `tally_for`); and the
  combined `TALLYING … REPLACING` form with several tally items. The single-item
  `TALLYING FOR ALL … {BEFORE|AFTER}` path is untouched.

### Added — v0.57.0: INSPECT REPLACING several items each with a BEFORE/AFTER region

`INSPECT source REPLACING ALL a BY x [{BEFORE|AFTER} p] ALL b BY y [{BEFORE|AFTER} q] …` — each
item of a MULTI-item REPLACING may now carry its OWN optional `{BEFORE|AFTER}` region, compiled
byte-identical to the `coding-adventures-cobol-runtime` 0.61.0 oracle. Previously any region on a
multi-item list was rejected at emit time ("INSPECT REPLACING with several items and a BEFORE/AFTER
region is a later rung"); that reject is now LIFTED. No grammar change was needed.

- `ReplaceItem` now carries `Option<(RegionKind, &GrammarASTNode)>` (its own region node);
  `inspect_replacing_multi` parses each item's region with the same keyword/operand extraction the
  single-item `inspect_replacing_all` uses. The LEADING/CHARACTERS/FIRST rejects for a multi-item
  list are UNCHANGED (the multi path stays `ALL`-only).
- `emit_inspect_replacing_multi` reuses `emit_inspect_region_window` — the SAME window the
  single-item region emitter and the TALLYING side emit — to derive each item's `[start, end)`
  ONCE, over the ORIGINAL source, before the unrolled `0..W` pass (the runtime length is
  materialised once, shared by every item that carries a region). In the per-position ordered
  if-else chain each region-carrying item's link now gates on `start <= j < end AND c == x`; a
  region-less item's link stays `c == x` (the window guard folds away). This is the exact
  composition of the pre-existing multi-item first-match chain with the single-item region gate.
- Byte-safety: the match only fires on a single-char ASCII search, so a multi-byte source char is
  never falsely matched, and each content-defined window selects the same positions on both
  engines. Reconstruction of a source that itself contains a multi-byte char remains the
  PRE-EXISTING byte-vs-char chip shared by every REPLACING lowering (per-position `str_slice`
  cannot slice a multi-byte char and traps, identically to the single-item `REPLACING ALL`); this
  rung adds no new non-ASCII behavior.
- Scope kept for a later rung (unchanged, identical messages to the oracle): a `LEADING` or
  `CHARACTERS`/`FIRST` item in a multi-item list, and the combined `TALLYING … REPLACING` form with
  several items. The single-item `REPLACING ALL … {BEFORE|AFTER}` path is untouched.

### Added — v0.56.0: INSPECT REPLACING CHARACTERS BY x (no region)

`INSPECT source REPLACING CHARACTERS BY x` — the "replace every position" form is now compiled,
byte-identical to the `coding-adventures-cobol-runtime` 0.60.0 oracle. Previously it was rejected
at emit time ("INSPECT REPLACING CHARACTERS is a later rung"). No grammar change was needed.

- Unlike `REPLACING ALL …` there is no per-position compare — EVERY position becomes `x`
  unconditionally. `emit_inspect_replacing_characters` reuses the REPLACING-ALL rebuild scaffold
  minus the `cmp_eq`: it appends the 1-character replacement string `width` times (the picture's
  compile-time CHAR width) into a fresh accumulator, then copies it back to the source register.
- Byte-basis co-totality: the oracle fills `n = storage.len()` (BYTE-length) copies then
  `move_into` re-pads/truncates to the picture's CHAR size. Emitting exactly `width` copies here
  reproduces that capped image on both engines. Worked non-ASCII regression: `PIC X(5) VALUE
  "café"` (5 chars / 6 bytes) REPLACING CHARACTERS BY `"Z"` → `"ZZZZZ"` (FIVE `Z`s) on both.
- Dispatch: the lone-REPLACING branch of `emit_inspect` detects the CHARACTERS keyword on the
  SINGLE replace item FIRST and routes to `emit_inspect_replacing_characters`, mirroring the
  oracle's `read_statement`.
- Guards, applied identically to the oracle: (3) a `{BEFORE|AFTER}` region on the CHARACTERS item
  is deferred; (2) a single-char but NON-ASCII *literal* `x` is a later rung (an explicit
  `is_ascii()` pre-check so the diagnostic/gating match the oracle rather than
  `single_delim_str`'s byte-based "multi-character" reject) — a `PIC X(1)` *item* replacement is
  not ASCII-gated; (1) `x` must be a single character (`single_delim_str`).
- Scope unchanged elsewhere: a CHARACTERS item inside a MULTI-item `REPLACING` list, and inside a
  combined `TALLYING … REPLACING`, remain later rungs, rejected identically to before.

### Added — v0.55.0: INSPECT TALLYING … FOR CHARACTERS (+ optional region)

`INSPECT source TALLYING counter FOR CHARACTERS [ {BEFORE|AFTER} x ]` — the "count every
position" tally form is now compiled, byte-identical to the `coding-adventures-cobol-runtime`
0.59.0 oracle. Previously it was rejected at emit time ("INSPECT TALLYING … FOR CHARACTERS is a
later rung"). No grammar change was needed.

- `FOR CHARACTERS` does NOT scan for a delimiter. The count is the LENGTH of the region window,
  ADDed to the counter: with no region `cnt = str_len(S)`; with a `{BEFORE|AFTER} x` region
  `cnt = end - start` (`sub`) over the SAME window `emit_inspect_region_window` produces for
  `FOR ALL`, so it inherits the identical BEFORE→whole / AFTER→empty not-found asymmetry. The
  count folds into the counter via the SAME `store_scaled` ADD the ALL/LEADING path uses.
- `emit_inspect_tallying` gains an `allow_characters` gate: the standalone lone-TALLYING caller
  passes `true`; the combined `TALLYING … REPLACING` caller passes `false`, so a combined
  CHARACTERS half is a clean later-rung error matching the oracle. `inspect_tally_all` now
  detects `CHARACTERS`, returns `characters: bool`, and reads NO delimiter operand on that path
  (`delim_node` becomes `Option`).
- The per-character match loop, `single_delim_code`, and delimiter registers are skipped
  entirely on the CHARACTERS path — only `str_len`, the optional window, and one `sub`/`add`
  are emitted.
- Multi-item / multi-counter `CHARACTERS` remain later rungs, rejected identically to before.

### Added — v0.54.0: UNSTRING … ON OVERFLOW / NOT ON OVERFLOW

`UNSTRING source DELIMITED BY delim INTO r1 [r2 …] [WITH POINTER p] [ON OVERFLOW imp…] [NOT ON
OVERFLOW imp…]` — the two optional overflow imperatives are now compiled, byte-identical to the
`coding-adventures-cobol-runtime` 0.58.0 oracle. Previously they were rejected at emit time
("UNSTRING … ON OVERFLOW / NOT ON OVERFLOW is a later rung"). The DIRECT sibling of the STRING
overflow dispatch (v0.53.0). No grammar change was needed.

- `emit_unstring` now yields an `overflow` i64 register (`1`/`0`) computed with the IDENTICAL
  comparison the oracle uses:
  - **Scan path:** after the receiver loop, `overflow = cmp_le(p, len)` (the final cursor `p` did
    not run past the source ⇒ fields remain), overwriting the pre-seed.
  - **`WITH POINTER` out of range:** the flag is PRE-SEEDED to `1` before the out-of-range guards,
    which jump straight to `us_end` with it still set (out-of-range IS overflow), skipping the
    scan, the `cmp_le` overwrite, and the write-back.
- After settling the flag, `emit_unstring` splits the `statement` children at the `NOT` keyword
  (exactly as the oracle reader and `emit_if`'s `ELSE` split do) into the ON / NOT-ON lists, then
  emits the usual `jmp_if_false`/branch/`label` skeleton guarding on the `overflow` register.
- The whole flag + skeleton is emitted ONLY when a clause is present — a plain UNSTRING with
  neither clause lowers EXACTLY as before this rung (no `overflow` register, no branch).
- **Behaviour change:** the out-of-range `WITH POINTER` case now runs the `ON OVERFLOW` list
  (still no data movement / pointer write-back), matching the oracle.

### Added — v0.53.0: STRING … ON OVERFLOW / NOT ON OVERFLOW

`STRING s1 s2 … DELIMITED BY {SIZE | delim} INTO t [WITH POINTER p] [ON OVERFLOW imp…] [NOT ON
OVERFLOW imp…]` — the two optional overflow imperatives are now compiled, byte-identical to the
`coding-adventures-cobol-runtime` 0.57.0 oracle. Previously they were rejected at emit time
("STRING … ON OVERFLOW / NOT ON OVERFLOW is a later rung"). No grammar change was needed.

- `emit_string` (and the shared `emit_string_pointer_overlay`) now yield an `overflow` i64
  register (`1`/`0`) computed with the IDENTICAL comparison the oracle uses:
  - **No `WITH POINTER`:** overflow ⇔ `total > width`, a COMPILE-TIME-known boolean materialised
    as a `const`.
  - **`WITH POINTER`:** `emit_string_pointer_overlay` pre-seeds the flag to `1`, lets the two
    out-of-range guards fall through to `st_end` with it still set (out-of-range IS overflow),
    and OVERWRITES it in the in-range path with the drop test `clen > avail`.
  - **`DELIMITED BY delim`, no pointer:** overflow ⇔ the run-time `clen > width` test (`gt`),
    reused directly as the flag.
- After the overlay, `emit_string` splits the `statement` children at the `NOT` keyword (exactly
  as the oracle reader and `emit_if`'s `ELSE` split do) into the ON / NOT-ON lists, then emits the
  usual `jmp_if_false`/branch/`label` skeleton guarding on the `overflow` register. Both clauses
  absent ⇒ no branch skeleton is emitted (a plain STRING lowers exactly as before).

### Added — v0.52.0: STRING … WITH POINTER

`STRING s1 s2 … DELIMITED BY {SIZE | delim} INTO t WITH POINTER p` — the optional `WITH POINTER`
phrase is now MODELLED, byte-identical to the `coding-adventures-cobol-runtime` 0.56.0 oracle.
Previously it was rejected at emit time ("STRING … WITH POINTER is a later rung"). No grammar
change was needed — the grammar already parses `WITH POINTER NAME`. This is the direct mirror of
the UNSTRING … WITH POINTER rung (v0.51.0).

`p` is an unsigned-integer item (`PIC 9(n)`) holding the **1-based** character position in the
RECEIVER at which the first transferred character is placed:

- **Overlay offset.** `emit_string` builds the concatenation register as before, then (with a
  pointer) hands off to a shared run-time overlay `emit_string_pointer_overlay`: it reads the
  pointer register `pv`, computes the 0-based start `pv − 1`, `avail = size − start`, and
  `chars_placed = min(concat_len, avail)`, and rebuilds the receiver as `recv[0,start] ++
  concat[0,chars_placed] ++ recv[start+chars_placed, size]` — overwriting only the filled run,
  keeping the untouched head and tail. Because the offset is a run-time value, the compile-time
  slicing of the no-pointer `DELIMITED BY SIZE` path no longer applies; the concat length is
  materialised as a `const` and the overlay runs at run time. With `p = 1` the result is the
  no-pointer overlay exactly.
- **Out-of-range guard.** The pointer's VALUE is a run-time datum, so it cannot be checked at
  build time. The overlay lowers a guard: if `p < 1` or `p > size` it jumps past the whole
  operation (overlay AND write-back) to a trailing `st_end` label, leaving the receiver and the
  pointer untouched — the ISO "overflow ⇒ no data movement" rule, matching the oracle's early
  return byte-for-byte.
- **Write-back.** After the overlay the pointer is stored as `pv + chars_placed` (the 1-based
  position one past the last character stored; `size + 1` when the content filled to or past the
  receiver end), reshaped into the pointer's `PIC 9(n)` picture through the same `store_scaled`
  path INSPECT's counter and UNSTRING's pointer use.
- **Build-time picture validation** (co-total with the oracle): the pointer must be an unsigned
  integer `PIC 9(n)`, `n ≤ 18`. A signed, fractional, non-numeric, group, or over-wide pointer is
  a clean later rung, rejected with the SAME message the oracle raises. The receiver NAME is split
  off from the pointer NAME at the `POINTER` keyword (the grammar is flat), matching the oracle
  reader.

`ON OVERFLOW` / `NOT ON OVERFLOW` remain deferred.

### Added — v0.51.0: UNSTRING … WITH POINTER

`UNSTRING S DELIMITED BY "," INTO r1 r2 … WITH POINTER p` — the optional `WITH POINTER`
phrase is now MODELLED, byte-identical to the `coding-adventures-cobol-runtime` 0.55.0 oracle.
Previously it was rejected at emit time ("UNSTRING … WITH POINTER is a later rung"). No grammar
change was needed — the grammar already parses `WITH POINTER NAME`.

`p` is an unsigned-integer item (`PIC 9(n)`) holding a **1-based** start position:

- **Start offset.** `emit_unstring` initialises the scan cursor from `p_value − 1` (a run-time
  read of the pointer item's register) instead of the constant 0. Everything downstream — the
  delimiter scan, per-receiver `str_slice`/reshape, exhaustion, empty fields — is UNCHANGED. With
  `p = 1` the lowering is behaviourally the no-pointer scan.
- **Out-of-range guard.** Because the pointer's VALUE is a run-time datum, it cannot be checked at
  build time. The emitter lowers a guard: if `p < 1` or `p > len` it jumps past the whole
  operation (receiver moves AND write-back) to a trailing `us_end` label, leaving the receivers
  and the pointer untouched — the ISO "overflow ⇒ no data movement" rule, matching the oracle's
  early return byte-for-byte.
- **Write-back.** After the scan the pointer is stored as `min(p, len) + 1` (clamp removes the
  scan's phantom step past end-of-source, `+ 1` restores 1-basing), reshaped into the pointer's
  `PIC 9(n)` picture through the same `store_scaled` path INSPECT's counter uses.
- **Build-time picture validation** (co-total with the oracle): the pointer must be an unsigned
  integer `PIC 9(n)`, `n ≤ 18` (so the value fits the `i64` slot). A signed, fractional,
  non-numeric, group, or over-wide pointer is a clean later rung, rejected with the SAME message
  the oracle raises. The receiver NAME list is split off from the pointer NAME at the `POINTER`
  keyword (the grammar is flat), matching the oracle reader.

`ON OVERFLOW` / `NOT ON OVERFLOW` remain deferred.

### Added — v0.50.0: UNSTRING with a reference-modified source

`UNSTRING S(2:3) DELIMITED BY "," INTO w1 w2 w3` — a reference-modified item slice
`base(start:len)` is now accepted as the UNSTRING source, byte-identical to the
`coding-adventures-cobol-runtime` 0.54.0 oracle. Previously it was rejected at emit time
("UNSTRING with a reference-modified source is a later rung"). No grammar change was needed —
the grammar already parses a ref-mod operand in that position.

This is a direct mirror of the literal-source rung: the ONLY thing that changed is the source
character provider. `emit_unstring` now has an `Operandy::RefMod` arm that obtains `s_reg` by
calling the SHARED `ref_mod_slice` helper (the same helper DISPLAY / comparisons use) — which
emits the identical `str_slice` (constant-folded for literal indices, register-computed for a
data-name index) and enforces the identical numeric-base and out-of-range rejects. Everything
downstream — the delimiter scan (`str_len` / `str_index` / `str_slice` loop) and the
per-receiver reshape — is UNCHANGED, because it reads `s_reg` purely as a string register.

Because the slice register is byte-for-byte what the oracle's `refmod_string` produces (DISPLAY
of the same slice already agreed between the engines), the split behaviour matches the oracle
for every case: field boundaries, empty fields, source exhaustion (trailing receivers keep
their prior VALUE), and per-receiver width reshaping. Both the literal-index path (`S(2:3)`)
and the computed-index path (`S(J:3)`, J a `PIC 9` data-name) are supported.

**Still deferred / unchanged.** A NUMERIC base under ref-mod is a later rung — the shared
`ref_mod_slice` rejects a numeric base, so UNSTRING inherits that reject identically to the
oracle. A GROUP base, out-of-range indices, and a signed/fractional index item behave exactly
as the existing reference-modification machinery already does (this rung only routes the
UNSTRING source through it; it does not change that machinery). No new ASCII guard is added:
the source base is an IDENTIFIER, so there is no new literal-scanning surface — a non-ASCII
base under ref-mod is the SAME pre-existing byte-vs-char behaviour the reference-modification
rungs already have (reachable via `DISPLAY S(2:3)`), tracked as the shared chip.

### Added — v0.49.0: STRING with DELIMITED BY a single-char delimiter

`STRING a b c DELIMITED BY "," INTO r` — a real single-character delimiter now compiles,
byte-identical to the `coding-adventures-cobol-runtime` 0.53.0 oracle. Previously only
`DELIMITED BY SIZE` was supported; a real delimiter was rejected at emit time. No grammar
change was needed — `string_delim` already parses `SIZE | operand`.

With `DELIMITED BY delim` each sending field contributes only its PREFIX up to the first
delimiter char. Where the `DELIMITED BY SIZE` path has all-compile-time-known lengths (a fixed
`str_slice`/`str_concat` overlay), the delimited path's per-field boundaries are DATA-dependent,
so `emit_string` now emits a genuine per-field scan LOOP (the same shape UNSTRING uses):
`flen = str_len(F); j = 0; while j < flen && F[j] != d { j++ }; prefix = F[0,j]`. The prefixes
are concatenated and the running length becomes a run-time value, so the receiver overlay also
runs at run time: `clen = str_len(concat); take = min(clen, W); r = concat[0,take] ++ r[take,W]`
— the preserved tail `r[take,W]` reproduces STRING's no-space-fill rule exactly as the
compile-time branch does. The `DELIMITED BY SIZE` path is UNCHANGED and still emits the exact
same fixed IIR as before.

The delimiter is reduced by the SAME `single_delim_code` UNSTRING/INSPECT use, so a
multi-character / numeric / figurative / reference-modified / wider-item delimiter rejects
identically to the oracle.

**ASCII guard.** A non-ASCII single-character LITERAL delimiter (`DELIMITED BY "é"`, one char /
two bytes) is rejected before lowering (the scan compares bytes while the oracle scans chars),
matching the oracle's reject with the same message. A non-ASCII string-LITERAL sending field
under an active delimiter (`STRING "café" DELIMITED BY "," …`) is likewise rejected. A non-ASCII
PIC X(1) delimiter ITEM is not build-time detectable, so — as with UNSTRING — it is left as the
shared byte-vs-char chip (both engines accept), keeping the accept/reject sets co-total.

Still deferred (rejected here and on the oracle alike): a multi-character delimiter, a non-ASCII
literal delimiter, a non-ASCII literal sending field under a delimiter, per-field different
delimiters, `WITH POINTER`, `ON OVERFLOW`, and a numeric receiver.

### Added — v0.48.0: UNSTRING with a literal source

`UNSTRING "a,b,c" DELIMITED BY "," INTO w1 w2 w3` — an alphanumeric STRING LITERAL in the
UNSTRING SOURCE position now compiles, byte-identical to the
`coding-adventures-cobol-runtime` 0.52.0 oracle. Previously a literal source was rejected at
emit time ("UNSTRING with a literal source is a later rung"). No grammar change was needed —
the grammar already parses a literal operand there.

Only the source PROVIDER changed. `emit_unstring` obtains its string register `s_reg` either
from an alphanumeric item's own char register (identifier source, as before) or, for a string
literal, from a fresh `str_const` register holding the literal's bytes. A `str_const` register
behaves identically to an item's char register under `str_len` / `str_index` / `str_slice` —
exactly as the `spaces_const` register already does inside the same routine — so the entire
downstream scan-and-fill loop (delimiter scan, per-receiver slice, truncate/pad reshape,
cursor advance, exhausted-source guard) is UNCHANGED and shared between the two providers.

Only an **ASCII** string literal is accepted. The oracle scans a literal source by CHARACTER
while `emit_unstring` lowers it to BYTE-based IIR string ops (`str_len`/`str_index`/
`str_slice`), so the two agree only when each character is one byte; a non-ASCII literal is
rejected before the `str_const` is emitted (`if !s.is_ascii()`), matching the oracle's
read-time reject with the same message so the accept/reject sets stay co-total.

Still deferred (rejected on the compiler and the oracle alike): a NUMERIC-literal source
(`UNSTRING 123 …`), a FIGURATIVE source (`UNSTRING SPACE …`), a NON-ASCII string-literal
source — only an ASCII alphanumeric string literal is supported — and a reference-modified
source (unchanged). `WITH POINTER`, `ON OVERFLOW`, a multi-character/`ALL`/`OR` delimiter, and
a numeric/group receiver remain later rungs.

### Added — v0.47.0: INSPECT TALLYING with multiple counters

`INSPECT source TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d [ALL e …] …` — TWO OR MORE
`tally_for` groups, each with its OWN counter and one-or-more single-char `FOR ALL`
delimiters — now compiles, byte-identical to the `coding-adventures-cobol-runtime` 0.51.0
oracle. Previously any several-counter TALLYING was rejected at read time ("several
counters is a later rung"). This GENERALISES the v0.46.0 multi-item single-counter lowering
to a list of `(counter, delimiter)` pairs where the matched pair's OWN counter is bumped.

Semantics (ISO COMBINED priority list ACROSS counters — the crux): ALL delimiters of ALL
groups form ONE ordered priority list, scanned in a SINGLE left-to-right pass. At each
position the delimiters are tried IN WRITTEN ORDER (group 1's items first, then group 2's,
…) and the FIRST that matches increments ITS OWN group's counter by 1, then the scan
advances. The per-position first-match `break` means an earlier group CONSUMES the
position — a character it claims NEVER reaches a later group's delimiter, so the groups are
NOT independent counts:

- `"aa"  TALLYING C1 FOR ALL "a"  C2 FOR ALL "a"`  gives `C1 += 2, C2 += 0`
- `"aba" TALLYING C1 FOR ALL "a" ALL "b"  C2 FOR ALL "a"`  gives `C1 += 3, C2 += 0`

Lowering: `emit_inspect_tally_counters` keeps ONE accumulator register per GROUP (init 0)
through a single RUNTIME `top:/end:` loop over `len = str_len(S)`. It reads `S[j]` once and
walks an ordered `cmp_eq` chain over the flattened `(group, delimiter)` pairs in written
order: on the FIRST match it bumps THAT group's accumulator and jumps past the rest to the
continue label; no match falls through with no bump. After the loop, `counter := counter +
accumulator` folds per group via the same `store_scaled` (silent high-order truncation on
overflow) the single-item tally uses — and each final add re-reads the counter's storage
register, so two groups naming the SAME counter both add to that one item correctly. A new
`inspect_tally_counters` CST reader walks the same `tally_for`/`tally_item` children the
oracle walks, and the dispatch keys PURELY on `fors.len() >= 2`, so the two engines'
accept/reject sets stay co-total.

Scope bound (this rung, identical messages on both engines): every item of every group must
be an `ALL` single-char delimiter with NO `{BEFORE|AFTER}` region and NO
`LEADING`/`CHARACTERS`; every counter must be an unsigned integer `PIC 9(n)`. A group
carrying any of those, and the COMBINED `TALLYING … REPLACING` form with several counters
(still rejected by `inspect_tally_all`'s several-counters guard), remain later rungs.
Exactly ONE `tally_for` keeps the single-counter lowerings (`emit_inspect_tallying` /
`emit_inspect_tally_multi`) UNCHANGED.

### Added — v0.46.0: INSPECT TALLYING with multiple FOR items (one counter)

`INSPECT source TALLYING counter FOR ALL a ALL b [ALL d …]` — TWO OR MORE `FOR ALL` tally
items sharing ONE counter — now compiles, byte-identical to the
`coding-adventures-cobol-runtime` 0.50.0 oracle. Previously any multi-item TALLYING was
rejected at read time ("several FOR phrases is a later rung").

Semantics (ISO priority-list, the count-side analogue of multi-REPLACING): ONE
left-to-right pass over the source. At each position the delimiters are tried IN WRITTEN
ORDER and the FIRST that matches increments the shared count by 1, then the scan advances.
The per-position first-match rule is what makes DUPLICATE delimiters NOT double-count:
`FOR ALL "a" ALL "a"` over `"aa"` adds 2 (each `a` counted once by the first item), not 4.
Net, the count is the number of source positions whose char equals SOME delimiter, each
counted once, ADDED to the counter (INSPECT adds; it does not clear it first).

Lowering: unlike the REPLACING emitter (which rebuilds a fixed-width string and so unrolls
`0..width` at compile time), the tally builds no string, so `emit_inspect_tally_multi`
emits a genuine RUNTIME `top:/end:` loop over `len = str_len(S)` — mirroring the
single-item `emit_inspect_tallying` loop shape. At each position it reads `S[j]` once and
walks an ordered `cmp_eq` chain (one link per delimiter): on the FIRST match it bumps
`cnt` and jumps past the rest to the continue label (so a position is counted at most
once); no match falls through with no bump. Then `counter := counter + cnt` folds via the
same `store_scaled` (silent high-order truncation on overflow) the single-item tally uses.
A new `inspect_tally_multi` CST reader counts the same `tally_item` children the oracle
counts, so the two engines' accept/reject sets are co-total.

Scope bound (this rung): the multi-item path supports ONLY `ALL` items, each a single-char
delimiter, with NO `{BEFORE|AFTER}` region and NO `LEADING`/`CHARACTERS`, under EXACTLY ONE
counter. A multi-item list carrying any of those, SEVERAL counters (more than one
`tally_for`), and the combined `TALLYING … REPLACING` form with several tally items remain
later rungs — rejected with identical messages on both engines. A single tally item keeps
the full single-item path (LEADING, region).

### Added — v0.45.0: INSPECT REPLACING with multiple replace items

`INSPECT source REPLACING ALL a BY x ALL b BY y [ALL c BY z …]` — TWO OR MORE replace
items in one REPLACING clause — now compiles, byte-identical to the
`coding-adventures-cobol-runtime` 0.49.0 oracle. Previously any multi-item REPLACING was
rejected at read time ("several replace items is a later rung").

Semantics (ISO): ONE left-to-right pass over the source. At each position the items are
considered IN WRITTEN ORDER and the FIRST whose single-char search matches the ORIGINAL
character wins; the position then advances. Two properties follow, both pinned by tests:

- FIRST-MATCH-WINS: only the earliest-written matching item fires at a position.
- NO RE-CHAINING: the byte a replacement produces is never re-examined by a later item.
  `REPLACING ALL "a" BY "b" ALL "b" BY "z"` over `"ab"` gives `"bz"`, not `"zz"` — a
  naive sequential two-pass replace would give `"zz"`.

Lowering: `emit_inspect_replacing_multi` unrolls over the compile-time width; at each
position it reads `S[j]` ONCE from the original source register and emits an ordered
if-else chain (one link per item) that appends the first matching item's replacement and
jumps to the position's done label — the early jump is exactly first-match-wins, and
always comparing against the original `S[j]` is exactly no-re-chaining. When no item
matches, the original character slice is appended. The width-`W` result is copied back
into the source register through an empty concat AFTER the last read, so the source is
not overwritten mid-scan. A new `inspect_replacing_multi` CST reader counts the same
`replace_item` children the oracle counts, so the two engines' accept/reject sets are
co-total.

Scope bound (this rung): the multi-item path supports ONLY `ALL` items, each a
single-char search BY single-char replacement, with NO `{BEFORE|AFTER}` region and NO
`LEADING`/`CHARACTERS`/`FIRST`. A multi-item list carrying any of those, and the combined
`TALLYING … REPLACING` form with several items, remain later rungs — rejected on both
engines with identical messages. A single replace item keeps the full single-item path
(LEADING, region, …) unchanged.

### Added — v0.44.0: standalone INSPECT FOR LEADING / REPLACING LEADING with a BEFORE/AFTER region

The STANDALONE `INSPECT source TALLYING counter FOR LEADING delim {BEFORE|AFTER} x` and
`INSPECT source REPLACING LEADING search BY replace {BEFORE|AFTER} x` forms now compile,
byte-identical to the `coding-adventures-cobol-runtime` 0.48.0 oracle. The crux is that
the LEADING run is ANCHORED at the WINDOW START, not source position 0: it counts /
replaces only the maximal run of matching characters that begins AT the window's start
index and stops at the first non-matching character INSIDE the window (or the window
end).

- `emit_inspect_tallying`: when a region is present on a `FOR LEADING` count, the scan
  is anchored at the window start — the loop counter is seated at `start` (via a `mov`)
  and the loop bound is the window `end` (not `0..len`), so the existing
  stop-at-first-mismatch break yields the window-anchored run. The pre-window region
  guard is skipped on that path (unnecessary once the loop is bounded to the window).
  `FOR ALL`'s lowering is UNTOUCHED — it still scans `0..len` with the in-window guard,
  byte-identical to before.
- `emit_inspect_replacing`: the per-position unroll now computes an `in_region`
  register once and derives the branch condition as `use_repl = active AND eq AND
  in_region` for LEADING + region (`eq AND in_region` for ALL + region, `active AND eq`
  for LEADING with no region, plain `eq` otherwise). The run decays ONLY on an
  IN-WINDOW mismatch — `active := active AND (eq OR NOT in_region)` — so positions
  before the window leave `active` untouched and the run truly starts at the window
  start. The `ALL` and no-region `LEADING` lowerings are byte-identical to before.
- A new `allow_leading_region` flag on both emitters re-imposes the combined-form
  deferral: the standalone callers pass `true`, the combined caller passes `false`, so
  a combined `TALLYING … REPLACING` whose LEADING half carries a region is a clean
  later-rung error with the same message the shared reader used to raise. The shared
  parsing helpers `inspect_tally_all` / `inspect_replacing_all` no longer reject a
  LEADING phrase carrying a region (that gate moved to the emitters' flag).
- Scoped SMALL — only the two STANDALONE forms. Still deferred, identically on both
  engines: a combined `TALLYING … REPLACING` with a LEADING half AND a region, and a
  multi-character region delimiter (rejected at emit via `single_delim_code`). No
  grammar change — the rejects were read/emit-time only.

### Added — v0.43.0: combined INSPECT TALLYING + REPLACING with a per-half BEFORE/AFTER region

The combined `INSPECT source TALLYING counter FOR ALL delim REPLACING ALL x BY y` form
now accepts an INDEPENDENT single-character `{BEFORE|AFTER}` region on EACH half — the
region that previously shipped only for the LONE `TALLYING FOR ALL` (v0.40.0) and
`REPLACING ALL` (v0.41.0) phrases. Each half narrows its own operation to a sub-slice
of the source bounded by the FIRST (leftmost) occurrence of that half's region
delimiter, with the ISO not-found asymmetry: `BEFORE` → the WHOLE source if the
delimiter is absent; `AFTER` → an EMPTY window if it is absent. Positions outside a
half's window are untouched. The two halves are fully independent — either, both, or
neither may carry a region, with their own kind and delimiter.

- The combined arm of `emit_inspect` now passes `allow_region = true` to BOTH
  `emit_inspect_tallying` and `emit_inspect_replacing` (previously `false`). Each
  standalone emitter already parses its own half's region from its own phrase child
  (`inspect_tallying` / `inspect_replacing`) and reuses the shared
  `emit_inspect_region_window` helper, so the only compiler change is routing each
  half's region through — no new region logic.
- ISO order is preserved: the tally count is emitted FIRST (it only READS `s_reg`),
  then the replace unroll rebuilds and stores back. Both halves' windows are therefore
  scanned over the SAME original source bytes, matching
  `coding-adventures-cobol-runtime` 0.47.0's `exec_inspect_tally_replace` byte-for-byte.
- Scoped SMALL — `FOR ALL` / `REPLACING ALL` only, single-character region delimiter
  only. Still rejected identically on both engines (each via the shared reader /
  `single_delim_code`): `FOR LEADING` or `REPLACING LEADING` carrying a region, and a
  multi-character region delimiter. No grammar change was needed.
- New e2e parity tests in `jit_e2e.rs`: `inspect_combined_region_tally_region_only`,
  `inspect_combined_region_replace_region_only`,
  `inspect_combined_region_both_before_distinct_delimiters`,
  `inspect_combined_region_before_and_after_different_kinds`,
  `inspect_combined_region_both_after`,
  `inspect_combined_region_tally_after_not_found_replace_before_not_found`,
  `inspect_combined_region_tally_before_not_found_whole_source`,
  `inspect_combined_region_replace_after_not_found_empty`,
  `inspect_combined_region_delimiter_at_last_position`,
  `inspect_combined_region_delimiter_equals_search_char_each_half`,
  `inspect_combined_region_pic_x1_delimiter`,
  `inspect_combined_region_adds_to_a_nonzero_counter`,
  `inspect_combined_region_delimiter_at_position_zero_empty_before`, and the three
  reject tests `inspect_combined_for_leading_with_a_region_is_a_later_rung`,
  `inspect_combined_replacing_leading_with_a_region_is_a_later_rung`,
  `inspect_combined_multi_char_region_delimiter_is_a_later_rung`. The obsolete
  `inspect_replacing_combined_with_a_region_is_a_later_rung` reject test is removed
  (the form it guarded is now supported).

### Added — v0.42.0: INSPECT CONVERTING with a BEFORE/AFTER region

`INSPECT source CONVERTING from TO to {BEFORE|AFTER} z` now compiles instead of being
rejected as a later rung — the exact analogue of the TALLYING- and REPLACING-region
rungs applied to the character translation. A `{BEFORE|AFTER} z` region narrows the
translation to a sub-slice of the source, bounded by the FIRST (leftmost) occurrence
of the SINGLE-character region delimiter `z`, with the ISO not-found asymmetry:
`BEFORE z` translates left of the first `z` (WHOLE source if `z` is absent); `AFTER z`
translates right of it (EMPTY — nothing converted — if `z` is absent). Positions
OUTSIDE the region keep their original character, even if that character appears in
the `from` set.

- `inspect_converting_pair` now PARSES the `inspect_region` CST child into
  `Option<(RegionKind, delim_node)>` (reusing the same keyword/operand extraction the
  count and replace sides use) instead of rejecting it, and returns it as the third
  element of the new `ConvertPhrase` alias. `emit_inspect_converting` REUSES
  `emit_inspect_region_window` — the SAME helper the TALLYING and REPLACING sides
  emit — to derive `[start, end)` over the ORIGINAL source, then guards its
  per-position translate unroll: when a region is active and position `j` lies outside
  the window it jumps straight past the table chain to the "keep the original
  character" fall-through (materialising the compile-time `j` into a register to
  compare against the runtime window bounds).
- With NO region the extra guard folds away — the lowering is byte-identical to
  v0.41.0, and matches `coding-adventures-cobol-runtime` 0.46.0's
  `exec_inspect_converting` window byte-for-byte on every accepted input (both now
  share the oracle's `region_window` semantics). No grammar change was needed.
- Scoped SMALL — CONVERTING only, single-character region delimiter only. A
  MULTI-character region delimiter is still rejected identically on both engines via
  `single_delim_code`, exactly like the search/tally delimiter.
- New e2e parity tests in `jit_e2e.rs`:
  `inspect_converting_before_translates_only_left_of_the_delimiter`,
  `inspect_converting_after_translates_only_right_of_the_delimiter`, both not-found
  branches, `inspect_converting_after_region_delimiter_at_position_zero`,
  `inspect_converting_before_region_delimiter_as_last_char`,
  `inspect_converting_region_delimiter_is_also_in_the_from_set`,
  `inspect_converting_before_delimiter_at_position_zero_is_an_empty_region`,
  `inspect_converting_multichar_table_with_a_region`,
  `inspect_converting_region_delimiter_is_a_pic_x1_item`,
  `inspect_converting_region_shorter_than_the_from_set`, and the reject test
  `inspect_converting_multi_char_region_delimiter_is_a_later_rung`. The obsolete
  `inspect_converting_before_region_is_a_later_rung` reject test is removed (the form
  it guarded is now supported).

### Added — v0.41.0: INSPECT REPLACING ALL with a BEFORE/AFTER region

`INSPECT source REPLACING ALL x BY y {BEFORE|AFTER} z` now compiles instead of being
rejected as a later rung — the exact analogue of the TALLYING-region rung applied to
the substitution instead of the count. A `{BEFORE|AFTER} z` region narrows the ALL
replacement to a sub-slice of the source, bounded by the FIRST (leftmost) occurrence
of the SINGLE-character region delimiter `z`, with the ISO not-found asymmetry:
`BEFORE z` replaces left of the first `z` (WHOLE source if `z` is absent); `AFTER z`
replaces right of it (EMPTY — no replacement — if `z` is absent). Positions OUTSIDE
the region keep their original character.

- `inspect_replacing_all` now PARSES the `inspect_region` CST child into
  `Option<(RegionKind, delim_node)>` (reusing the count side's keyword/operand
  extraction) instead of rejecting it, and rejects `REPLACING LEADING` carrying a
  region. `emit_inspect_replacing` REUSES `emit_inspect_region_window` — the SAME
  helper the TALLYING side emits — to derive `[start, end)` over the ORIGINAL source,
  then guards its per-position `ALL` unroll so a match at position `j` is rewritten
  only when `start <= j < end` (materialising the compile-time `j` into a register to
  compare against the runtime window bounds). It gains an `allow_region` gate: the
  lone `REPLACING ALL` path passes `true`, the combined path `false`.
- With NO region the extra guard folds away — the lowering is byte-identical to
  v0.40.0, and matches `coding-adventures-cobol-runtime` 0.45.0's `inspect_replace`
  window byte-for-byte on every accepted input (both now share the oracle's
  `region_window` semantics).
- Scoped SMALL — `REPLACING ALL` only, single-character region delimiter only. Still
  rejected identically on both engines: `REPLACING LEADING` + region, a region on the
  combined `TALLYING … REPLACING` form (`allow_region == false`), and — via
  `single_delim_code`, exactly like the search delimiter — a MULTI-character region
  delimiter.
- New e2e parity tests in `jit_e2e.rs`:
  `inspect_replacing_before_replaces_only_left_of_the_delimiter`,
  `inspect_replacing_after_replaces_only_right_of_the_delimiter`, both not-found
  branches, `inspect_replacing_after_region_delimiter_at_position_zero`,
  `inspect_replacing_region_delimiter_equals_search`,
  `inspect_replacing_region_delimiter_equals_replacement`,
  `inspect_replacing_before_delimiter_at_position_zero_is_an_empty_region`,
  `inspect_replacing_region_delimiter_is_a_pic_x1_item`, and the reject tests
  `inspect_replacing_leading_with_a_region_is_a_later_rung`,
  `inspect_replacing_multi_char_region_delimiter_is_a_later_rung`, and
  `inspect_replacing_combined_with_a_region_is_a_later_rung`.

### Added — v0.40.0: INSPECT TALLYING FOR ALL with a BEFORE/AFTER region

`INSPECT source TALLYING counter FOR ALL delim {BEFORE|AFTER} x` now compiles
instead of being rejected as a later rung. A `{BEFORE|AFTER} x` region narrows the
count to a sub-slice of the source, bounded by the FIRST (leftmost) occurrence of
the SINGLE-character region delimiter `x`, with the ISO not-found asymmetry:
`BEFORE x` counts left of the first `x` (whole source if `x` is absent); `AFTER x`
counts right of it (EMPTY if `x` is absent).

- `inspect_tally_all` now PARSES the `inspect_region` CST child into `Option<
  (RegionKind, delim_node)>` (a new compiler-side `RegionKind { Before, After }`)
  instead of rejecting it, and rejects `FOR LEADING` carrying a region. A new
  `emit_inspect_region_window` helper emits a single scan for the first occurrence of
  the region delimiter (a `found` flag + first index `fidx`), then derives the window
  `[start, end)` per the BEFORE→whole / AFTER→empty rule. `emit_inspect_tallying`
  gains an `allow_region` gate (lone TALLYING passes `true`, the combined path
  `false`) and, when a region is present, bounds its existing per-position `FOR ALL`
  count loop with `j < start || j >= end → skip` (jump to `nobump`, keep scanning).
- With NO region the lowering emits nothing extra — it is byte-identical to v0.39.0,
  and matches `coding-adventures-cobol-runtime` 0.44.0's `inspect_tally` window
  byte-for-byte on every accepted input.
- Scoped SMALL — `TALLYING FOR ALL` only, single-character region delimiter only.
  Still rejected identically on both engines: `FOR LEADING` + region, a region on the
  combined `TALLYING … REPLACING` form (`allow_region == false`), and — via
  `single_delim_code`, exactly like the tally delimiter — a MULTI-character region
  delimiter. `REPLACING`/`CONVERTING` regions, `CHARACTERS`, several counters/FOR
  phrases, a numeric source, and a non-integer/signed counter remain clean
  `Unsupported`.
- New e2e parity tests in `jit_e2e.rs`: `inspect_before_counts_only_left_of_the_
  delimiter`, `inspect_after_counts_only_right_of_the_delimiter`, both not-found
  branches, `inspect_after_delimiter_at_position_zero`, `inspect_before_delimiter_at_
  last_position`, `inspect_region_delimiter_equals_tally_delimiter`, `inspect_before_
  delimiter_at_position_zero_is_an_empty_region`, `inspect_region_adds_to_a_nonzero_
  counter`, `inspect_region_delimiter_is_a_pic_x1_item`, and the two reject tests
  `inspect_for_leading_with_a_region_is_a_later_rung` and `inspect_multi_char_region_
  delimiter_is_a_later_rung`.

### Added — v0.39.0: combined INSPECT TALLYING with REPLACING LEADING

The COMBINED `INSPECT source TALLYING counter FOR ALL|LEADING delim REPLACING LEADING
x BY y` now compiles instead of being rejected as a later rung. This is the exact
MIRROR of v0.38.0: where that rung let the TALLYING half be `FOR LEADING`, this rung
lets the REPLACING half be `LEADING` (rewrite only the **consecutive run** of `x` at
the START of the source, stopping at the first non-match). The two halves' leading
flags are now fully independent — either, both, or neither may be LEADING.

- The change is a single flag: `emit_inspect`'s `(has_tally, has_repl) == (true,
  true)` arm now calls `emit_inspect_replacing(verb, &s_reg, source_width, /*
  allow_leading */ true)` for the replace half (matching the tally half, which
  already passed `true`). `emit_inspect_replacing` already threaded the `active`-
  guarded run unroll from the lone `REPLACING LEADING` rung — the per-position
  `use_repl = and(active, eq)` with `active` sticking at 0 after the first mismatch —
  so no codegen change was needed. Tally still emits FIRST (over the original `s_reg`
  bytes), then the leading replace rebuild, matching the oracle's tally-then-replace
  order; a shared `delim == x` is counted before it is substituted.
- This composes the two existing lone-form lowerings, so the JIT output is
  byte-identical to `coding-adventures-cobol-runtime` 0.43.0. Every other combined
  gate is unchanged: a second `FOR`/replace item, `CHARACTERS`/`BEFORE`/`AFTER`, a
  multi-character/figurative/wider operand, a numeric source, and a non-integer/
  signed counter remain clean `Unsupported`.
- The combined-deferred-half unit test flips from a reject to a "now compiles"
  assertion (`inspect_combined_tally_replacing_leading_now_compiles`). New e2e parity
  tests in `jit_e2e.rs`: `inspect_combined_replacing_leading_all_tally` (`"00X00"`,
  `FOR ALL` tally + leading replace, shared `delim == search` → `004` / `"**X00"`),
  `inspect_combined_both_halves_leading` (`"00X00"` both halves leading → `002` /
  `"**X00"`), `inspect_combined_replacing_leading_no_run` (`"X00X"` → `002` /
  unchanged), and `inspect_combined_characters_is_still_a_later_rung` (a still-
  deferred combined `CHARACTERS` sub-form remains rejected on both engines).

### Added — v0.38.0: combined INSPECT TALLYING FOR LEADING with REPLACING ALL

The COMBINED `INSPECT source TALLYING counter FOR LEADING delim REPLACING ALL x BY y`
now compiles instead of being rejected as a later rung. The combined form's TALLYING
half may now be `FOR LEADING` (count only the **consecutive run** of `delim` at the
START of the source) as well as `FOR ALL`; the REPLACING half stays `ALL`-only.

- The change is a single flag: `emit_inspect`'s `(has_tally, has_repl) == (true,
  true)` arm now calls `emit_inspect_tallying(verb, &s_reg, /* allow_leading */
  true)` for the tally half (the REPLACING-half call keeps `allow_leading = false`,
  so a combined `TALLYING … REPLACING LEADING` is still a clean `Unsupported`).
  `emit_inspect_tallying` already emitted the correct leading-run lowering — the
  `leading ? end : nobump` mismatch jump that breaks out of the count loop at the
  first non-match instead of merely skipping the `cnt += 1` — so no codegen change
  was needed. Tally still emits FIRST (over the original `s_reg` bytes), then the
  `REPLACING ALL` rebuild, matching the oracle's tally-then-replace order.
- This composes the two existing lone-form lowerings, so the JIT output is
  byte-identical to `coding-adventures-cobol-runtime` 0.42.0. Every other combined
  gate is unchanged: a combined `REPLACING LEADING`, a second `FOR`/replace item,
  `CHARACTERS`/`BEFORE`/`AFTER`, a multi-character/figurative/wider operand, a
  numeric source, and a non-integer/signed counter remain clean `Unsupported`.
- The combined-deferred-half unit test flips from a reject to a "now compiles"
  assertion (`inspect_combined_tally_for_leading_now_compiles`). New e2e parity tests
  in `jit_e2e.rs`: `inspect_combined_for_leading_counts_only_the_leading_run`
  (`"000X0"`, shared `delim == search` → `003` / `"***X*"`),
  `inspect_combined_for_leading_no_leading_run` (`"X00X"` → `000` / `"X**X"`),
  `inspect_combined_for_leading_all_characters_match` (`"0000"` → `004` / `"****"`),
  and `inspect_combined_replacing_leading_is_a_later_rung` (still rejected).

### Added — v0.37.0: INSPECT REPLACING LEADING (leading-run replace)

A lone `INSPECT source REPLACING LEADING search BY replace` now compiles instead of
being rejected as a later rung. `REPLACING LEADING` replaces only the run of
**consecutive** `search` characters at the START of the source, stopping at the
first character that is not `search`; positions after that first gap are left
unchanged **even if they equal `search`** (the contrast with `REPLACING ALL`, which
replaces every occurrence). Width is unchanged (single char → single char).

- The per-position **rebuild** unroll over the compile-time width `W` is reused
  verbatim; the ONLY addition for `LEADING` is a runtime `active` flag (`i64`, init
  `const 1`) threaded through the loop: at each `j`, `eq = cmp_eq(str_index(s,j),
  search)`, the replace branch is taken iff `use_repl = and(active, eq)`, and then
  `active = and(active, eq)` — so once a mismatch clears `active` it stays 0 for
  every later position and no further character is replaced. This is byte-identical
  to the oracle's stateful `in_run` map. When not `LEADING` the extra `and` is not
  emitted and the unroll is byte-identical to the original `REPLACING ALL` lowering.
- `inspect_replacing_all` now returns a `leading: bool` (true for `REPLACING
  LEADING`, false for `REPLACING ALL`); `emit_inspect_replacing` takes an
  `allow_leading` flag so the **combined** `TALLYING … REPLACING` path passes
  `false` — a combined `TALLYING … REPLACING LEADING` stays a clean `Unsupported`.
- `REPLACING ALL` is byte-identical to before. `REPLACING CHARACTERS`/`FIRST`,
  `BEFORE`/`AFTER` regions, several replace items, a `REPLACING LEADING` inside the
  combined form, and a multi-character/figurative/wider/numeric search or
  replacement remain deferred and reject identically to the oracle.
- Tests: `inspect_replacing_leading_now_compiles` (was
  `…_is_a_later_rung`) and `inspect_combined_tally_replacing_leading_is_a_later_rung`
  unit tests; JIT e2e `inspect_replacing_leading_*` (000123→***123, 00X00→**X00 vs
  ALL **X**, 120003 unchanged, 0000→****, blank unchanged, PIC X(1) operands),
  all pinned byte-for-byte against the oracle via `assert_matches_oracle`.

### Added — v0.36.0: INSPECT TALLYING FOR LEADING (leading-run count)

A lone `INSPECT source TALLYING counter FOR LEADING delim` now compiles instead of
being rejected as a later rung. `FOR LEADING` counts only the run of **consecutive**
`delim` characters at the START of the source, stopping at the first character that
is not `delim`, then ADDs that count to the counter (INSPECT adds; it does not clear
the counter first — identical to `FOR ALL` in that respect).

- The scan reuses the exact `FOR ALL` count loop (`str_len` + `str_index`/`cmp_eq`,
  then `store_scaled`). The **only** difference is the not-equal branch's jump
  target: `FOR ALL` jumps to `nobump` (skip the `cnt += 1`, keep scanning), while
  `FOR LEADING` jumps to `end` (break out of the loop). This mirrors the oracle's
  `filter(…).count()` vs `take_while(…).count()`.
- `inspect_tally_all` now returns a `leading: bool` (true for `FOR LEADING`, false
  for `FOR ALL`); `emit_inspect_tallying` takes an `allow_leading` flag so the
  **combined** `TALLYING … REPLACING` path passes `false` — a combined
  `TALLYING … FOR LEADING … REPLACING` stays a clean `Unsupported`.
- Still deferred and rejected identically to before: `LEADING` inside a `REPLACING`
  clause, `LEADING` in the combined form, `BEFORE`/`AFTER` regions, `CHARACTERS`,
  `FIRST`, a multi-character/figurative delimiter, and a numeric/group source. The
  `FOR ALL` lowering is byte-identical to the previous release.
- New JIT e2e tests pin the compiled leading-run scan to the oracle byte-for-byte:
  `"000123"` FOR LEADING → 3 (and FOR ALL → 3, agreeing here); `"120003"` FOR
  LEADING → 0 (FOR ALL would be 3); `"0000"` → 4; a blank `PIC X(3)` → 0; a
  `PIC X(1)` delimiter item; and adding onto a nonzero counter. The lone-tally unit
  test now asserts it compiles; the combined-LEADING reject test is retained.

### Changed — v0.35.0: EVALUATE reuses the IF relation dispatch (mixed numeric↔alphanumeric subject/WHEN)

An `EVALUATE` whose subject and a `WHEN` value are in **different** categories —
a numeric subject vs an alphanumeric `WHEN` (`EVALUATE NUM WHEN "042"`), or an
alphanumeric subject vs a numeric `WHEN` — now compiles instead of being rejected.
Previously `EVALUATE` split into two same-category paths (`emit_when_match` reading
each `WHEN` via `read_arith_term`, or `emit_when_match_str` via `str_value`), each of
which rejected the other category — a reject-vs-answer gate divergence, since the
oracle already routes every subject-vs-`WHEN` comparison through `compare_operands`
(which handles mixed pairs exactly as an `IF` relation does).

The fix factors the category-dispatching core of `emit_relation` into a reusable
`emit_operand_relation(left, right, op)` helper, then rewrites `EVALUATE`'s `WHEN`
matching to call it: a single value `[v]` emits `emit_operand_relation(subject, v,
"cmp_eq")`, a `THRU` range `[lo, hi]` emits `and(cmp_ge, cmp_le)`, and the value-list
`OR`-folds as before. Each subject-vs-`WHEN` comparison is now identical to
`IF subject <relop> value`, so `EVALUATE` inherits `IF`'s full category dispatch —
numeric (scaled `cmp_*`), alphanumeric (`str_cmp`), and mixed numeric↔alphanumeric
with **unsigned / signed (overpunch) / scaled** digit images, figuratives, and the
`ZERO`-numeric routing (`WHEN ZERO` stays a numeric comparison) — and its **deferral
set**: a numeric-literal `WHEN` against an alphanumeric subject (and a group operand)
is a clean reject on both engines, exactly as the oracle defers it. Byte-identical to
`coding-adventures-cobol-runtime` 0.39.0 by construction.

- Factored `emit_operand_relation` out of `emit_relation` (behaviour of `IF`
  relations unchanged — all existing relation tests still pass).
- Deleted the now-dead `emit_when_match_str`, `str_value`, and `emit_scaled_cmp`;
  `emit_when_match` is rewritten to take the subject grammar node and dispatch through
  the shared helper. `emit_evaluate` no longer pre-classifies the subject.
- New e2e tests: `evaluate_numeric_subject_alphanumeric_when`,
  `evaluate_signed_numeric_subject_alphanumeric_when`,
  `evaluate_scaled_numeric_subject_alphanumeric_when`,
  `evaluate_scaled_numeric_subject_numeric_when_stays_numeric`,
  `evaluate_mixed_thru_range`, `evaluate_numeric_subject_when_zero_stays_numeric`,
  and `evaluate_alpha_subject_numeric_literal_when_is_a_later_rung` (both engines
  reject the deferred numeric-literal pairing identically).

### Added — v0.34.0: figurative-vs-figurative comparison

A relational condition comparing **two figurative constants** (`IF ZERO = ZERO`,
`IF ZERO = SPACE`, `IF SPACE < ZERO`, …) now compiles instead of being rejected as
a later rung. Each figurative has no operand length to borrow, so both resolve to a
single fill character (width 1) — `ZERO` → `"0"`, `SPACE` → `" "` — and are compared
by the ordinary space-padded `str_cmp` path. This is byte-identical to the oracle,
whose `src_chars` of a figurative is empty so both `fill_fig` to `len().max(1)` = 1.
So `ZERO = ZERO` and `SPACE = SPACE` are true, `ZERO ≠ SPACE`, and by byte value
(`'0'` = 0x30 > `' '` = 0x20) `ZERO > SPACE` / `SPACE < ZERO`. Fixes a
reject-vs-answer gate divergence surfaced by adversarial review of v0.33.0 (the
oracle already answered these; the compiler rejected them). One-line change in
`emit_str_condition` (the `(None, None)` width arm); no grammar / lexer / parser
change.

### Added — v0.33.0: SIGNED numeric ↔ alphanumeric COMPARISON (overpunched image)

The mixed numeric ↔ alphanumeric relation (`IF NUM = "str"`, `<`, `>`, …) now
accepts a **SIGNED** numeric operand (`PIC S9(i)V9(d)`, integer or scaled), not only
an unsigned one. Oracle-first and byte-identical to
`coding-adventures-cobol-runtime` 0.37.0. No grammar / lexer / parser change.

- **The comparison image carries a trailing sign overpunch.** When a signed DISPLAY
  numeric is compared against an alphanumeric operand, its comparison image is its
  `(i + d)`-digit zero-padded MAGNITUDE with the operational sign folded into a
  TRAILING OVERPUNCH on the units (last) digit — the SAME image the signed
  numeric → alphanumeric MOVE (v0.32.0) produces. The units digit `u` maps: positive
  `{ A B C D E F G H I`, negative `} J K L M N O P Q R`. So `PIC S9(3) = -123`
  compares **equal** to `"12L"`, `= +123` equal to `"12C"`, and a scaled
  `PIC S9V9 = -4.2` equal to `"4K"`. Ordering follows the byte comparison of these
  images.
- **Fixed: `numeric = ZERO` is now a NUMERIC comparison.** `emit_relation`
  previously routed a numeric item compared against the `ZERO` figurative through the
  alphanumeric mixed path (`str_operand` carries `ZERO` as `Fig('0')`). For an unsigned
  item that happened to agree with the oracle (a zero-padded magnitude string orders
  like its value); for a SIGNED item the overpunched image (`"00{"`, `"12L"`, …)
  compared against `"000"` answered wrongly — silently miscompiling the ubiquitous
  `IF BALANCE = ZERO`. `numeric ↔ ZERO` now takes the numeric comparison path (`ZERO`
  → `0`), matching the oracle (whose mixed gate excludes `Fig::Zero` and numeric-compares
  `Num` vs `Fig::Zero`). `ZERO` stays alphanumeric only against a character operand;
  `ZERO`-vs-`ZERO` stays a string compare.
- **The lowering reuses `emit_signed_num_alpha_image`.** `num_digit_str_operand`'s
  signed arm (previously a clean `Unsupported`) now binds `int_digits`/`dec_digits`,
  computes `n = i + d`, and returns `StrOperand::Fixed { reg:
  emit_signed_num_alpha_image(&num_reg, n), len: n }` — the exact same overpunched
  image builder the signed MOVE uses (magnitude via `emit_num_digit_string`, units
  overpunch by slicing the combined `"{ABCDEFGHI}JKLMNOPQR"` table at
  `units + neg*10`). Both operands then flow through the identical space-padded
  `str_cmp` an all-alphanumeric relation takes, so the byte comparison matches the
  oracle exactly. The unsigned path and the pure numeric-vs-numeric path are
  byte-identical to before.
- **Sign-of-zero (no regression).** A value that truncates to a zero magnitude stores
  `neg = false` (COBOL has no negative zero), so `emit_signed_num_alpha_image` on a
  zero slot yields `"00{"` — equal to the oracle's `overpunch_trailing("000", false)`.
- **Still deferred (rejected identically on both engines).** A numeric LITERAL vs an
  alphanumeric operand (a different pairing, out of scope) and a group item in a
  mixed comparison remain clean `Unsupported`s. The old
  `mixed_signed_numeric_vs_alphanumeric_is_a_later_rung` reject e2e test is replaced
  by positive oracle-parity tests (negative/positive equality, units-digit-0,
  scaled, an ordering relation, and sign-of-zero); a
  `signed_numeric_vs_alphanumeric_comparison_now_compiles` and a
  `numeric_literal_vs_alphanumeric_comparison_is_still_a_later_rung` unit test are
  added, and the group-item reject test is kept.

### Added — v0.32.0: SIGNED numeric → alphanumeric MOVE (trailing sign overpunch)

The cross-category numeric → alphanumeric MOVE now accepts a **SIGNED** source
(`PIC S9(i)V9(d)`, integer or scaled), not only an unsigned one. Oracle-first and
byte-identical to `coding-adventures-cobol-runtime` 0.36.0. No grammar / lexer /
parser change.

- **The image carries a trailing sign overpunch.** A signed DISPLAY numeric's
  alphanumeric image is its `(i + d)`-digit zero-padded MAGNITUDE with the
  operational sign folded into a TRAILING OVERPUNCH on the units (last) digit — the
  same zoned-decimal encoding the runtime's `overpunch_trailing` /
  `__cob_print_signed` produce on `DISPLAY`. The units digit `u` maps: positive
  `{ A B C D E F G H I`, negative `} J K L M N O P Q R`. So `S9(3) = +123 → "12C"`,
  `= -123 → "12L"`, `S9V9 = -4.2 → "4K"`.
- **The lowering builds the overpunched last byte arithmetically.** The match arm
  is generalized from `signed: false` to `signed: _`. For a signed source,
  `emit_signed_num_alpha_image` computes `neg = (slot < 0) ? 1 : 0` (`cmp_lt`),
  `mag = |slot|` (`emit_abs`), the `(i+d)`-digit magnitude image (the existing
  `emit_num_digit_string`), and `units = mag % 10`. It then slices ONE combined
  20-character constant `"{ABCDEFGHI}JKLMNOPQR"` at `idx = units + neg*10` — the
  positive row `{…I` at indices `0..=9`, the negative row `}…R` at `10..=19` — so
  `table[idx..idx+1]` is exactly `overpunch_trailing`'s character (`POS[u]` at `u`,
  `NEG[u]` at `10+u`). The final image is `image[0..n-1] ++ overpunch_char`
  (`str_slice` + `str_concat`); for `n == 1` the head slice is empty, so the result
  is just the overpunch char. It then feeds the same `move_str_into_char` reshape
  (left-justify, space-pad, or truncate) the unsigned image uses.
- **Why it matches `overpunch_trailing` byte-for-byte.** The overpunch table's
  positive/negative rows are laid end to end in the combined constant, so indexing
  at `units + neg*10` selects the identical byte the oracle picks by
  `POS[u]`/`NEG[u]`; the units digit is `|slot| % 10` and the sign is `slot < 0`,
  matching the oracle's magnitude-last-digit + `item.neg`. A signed *positive*
  source (`neg = 0`) takes the positive row, so `"12C"` — differing from an unsigned
  `"123"`. The unsigned path is unchanged (no overpunch, plain magnitude).
- **Still deferred (clean `Unsupported`).** An alphanumeric → SIGNED numeric MOVE, a
  `SIGN` clause with `SEPARATE`/`LEADING`, and a group on either side (the compiler
  models no group items, so a group receiver is rejected). The old
  `signed_numeric_to_alphanumeric_move_is_deferred` reject test is replaced by
  `signed_numeric_to_alphanumeric_move_lowers`; a
  `signed_numeric_to_group_receiver_is_deferred` unit test and seven `jit_e2e`
  oracle-parity tests (positive exact-fit, wider space-pad, narrower truncate,
  negative, units-digit-0, scaled `±4.2`, and a computed value) are added.

### Added — v0.31.0: alphanumeric → SCALED-receiver MOVE (`MOVE PIC X(m) TO 9(i)V9(d)`)

The REVERSE cross-category MOVE (alphanumeric → numeric) now accepts an
**unsigned SCALED** receiver `PIC 9(i)V9(d)` (`d > 0`), not only an unsigned
integer. Oracle-first and byte-identical to `coding-adventures-cobol-runtime`
0.35.0. No grammar / lexer / parser change.

- **The fold-is-the-slot rule.** The source's `m` characters fold left-to-right
  into an unsigned integer `V` (`V = V*10 + (byte - '0')`), and that fold **is the
  receiver's scaled-slot magnitude directly** — it fills the `(i + d)` digit
  positions RIGHT-justified with the implied point `d` places from the right. So
  the slot is `V mod 10^(i+d)`: left-zero-padded when the source is shorter than
  `i + d`, high-order-truncated when longer. This is **NOT** the arithmetic
  decimal-align rule — `V` is *not* multiplied by `10^d`. Examples:
  `MOVE "042" TO 9(2)V9` → slot `042` (reads `4.2`);
  `MOVE "42" TO 9(2)V9` → `042`; `MOVE "12345" TO 9(2)V9` → `345` (reads `34.5`);
  `MOVE "5" TO 9(1)V99` → `005` (reads `0.05`).
- **The lowering.** The compiler folds `V` exactly as for the integer receiver
  (`emit_str_to_int`), then hands `store_scaled` the **receiver's own scale `d`**
  as the value scale. `store_scaled` rescales `d → d` (a no-op — no shift) and
  keeps the low-order `(i + d)` digits (`mag mod 10^(i+d)`) = `V mod 10^(i+d)`.
  Passing scale `0` instead would up-shift by `10^d` (the wrong, arithmetic rule).
  For `d = 0` this reproduces the old integer-receiver path byte-for-byte. The
  match arm's `dec_digits: 0` gate is relaxed to `dec_digits: d` (any unsigned
  receiver); the `value_max_int = m` argument only feeds the up-scale overflow
  guard, which never fires here (from-scale == to-scale), so its exact value is
  immaterial.
- **Still deferred (clean `Unsupported`).** A **signed** (`PIC S9`) receiver, a
  source wider than 18 characters (its `i64` fold could overflow), and group items.
- **Tests.** The former `alphanumeric_to_scaled_numeric_move_is_a_later_rung`
  reject test becomes the positive `alphanumeric_to_scaled_numeric_move_lowers`;
  new `jit_e2e` cases cover exact-fit, shorter-source zero-pad, longer-source
  high-order truncation, more-fraction-than-source digits, MOVE-then-arithmetic,
  and the SPACE-source no-stray-sign regression, all through
  `assert_matches_oracle`. A new `alphanumeric_to_signed_scaled_numeric_move_is_a_later_rung`
  keeps the signed-scaled deferral.

### Added — v0.30.0: unsigned SCALED operand in num→alpha MOVE and mixed comparison

The numeric→alphanumeric MOVE and the mixed numeric↔alphanumeric comparison now
accept an **unsigned SCALED** numeric operand (`PIC 9(i)V9(d)`, `d > 0`, no `S`),
not only an unsigned integer. Oracle-first and byte-identical to
`coding-adventures-cobol-runtime` 0.34.0. No grammar / lexer / parser change.

- **The digit-image rule.** A scaled numeric moved to / compared as alphanumeric
  uses its **digit image = all its digits, integer part followed by fractional
  part, concatenated with NO decimal point** — the `(i + d)`-digit zero-padded
  magnitude. The scaled `i64` slot already holds `value * 10^d`, so its full
  `(i + d)` digits *are* the image (no point inserted). This is exactly what the
  oracle's `Decimal::digits()` (`int + frac`) yields. Examples:
  `PIC 9(2)V9 = 4.2` → `"042"`; `PIC 9(1)V99 = 3.14` → `"314"`.
- **MOVE.** `MOVE 9(2)V9=4.2 TO X(3)` → `"042"`, `→ X(5)` → `"042  "` (pad),
  `→ X(2)` → `"04"` (truncate). The MOVE arm's `dec_digits: 0` gate is relaxed to
  `signed: false` (any `dec_digits`) and `emit_num_digit_string` is called over
  `n = int_digits + dec_digits` digits — the identical `div`/`mod`/table-slice
  loop, just over more digits.
- **Comparison.** `IF 9(2)V9=4.2 = "042"` → **true**; `IF … > "040"` → **true**.
  `num_digit_str_operand` builds the same `(i + d)`-digit image, and both operands
  run the same space-padded `str_cmp` path a same-category alphanumeric relation
  uses, so the byte comparison is byte-identical to the oracle.
- **Still deferred (clean `Unsupported`).** A **signed** (`PIC S9`) numeric
  operand (integer or scaled), the **reverse** alphanumeric→scaled-receiver MOVE, a
  numeric-edited receiver, and group items.
- **Tests.** The former `scaled_numeric_to_alphanumeric_move_is_deferred` unit test
  is now the positive `scaled_numeric_to_alphanumeric_move_lowers`; new `jit_e2e`
  cases cover the exact-fit, more-fraction-digit, pad, truncate, and computed-value
  MOVEs and the equal / ordering / more-fraction-digit comparisons, all through
  `assert_matches_oracle`. The signed operand stays a deferral test.

### Added — v0.29.0: numeric ↔ alphanumeric comparison

A relational condition (in `IF` / `EVALUATE` / any condition context) comparing
an **unsigned-integer** numeric operand (`PIC 9(n)` — no `S`, no `V`) with an
**alphanumeric** operand (a `PIC X` item **or** a string literal). Oracle-first
and byte-identical to `coding-adventures-cobol-runtime` 0.33.0. No grammar /
lexer / parser change was needed — a `relation` already parses; this fills in a
formerly-`Unsupported` mixed pairing.

- **The rule.** When a numeric and a non-numeric operand are compared, COBOL
  treats the **numeric operand as though moved to an alphanumeric field** — its
  `n`-digit zero-padded **digit image** (the exact bytes a numeric→alphanumeric
  `MOVE` or a `DISPLAY` of the same item yields) — and the comparison proceeds by
  the **alphanumeric byte rule**: the shorter operand is space-padded on the right
  to the longer's length, then the two are compared byte-by-byte. So `IF NUM =
  "042"` with `NUM PIC 9(3) = 42` compares `"042"` = `"042"` → **true**; `IF NUM =
  "42"` compares `"042"` vs `"42 "` (space-padded) → **false**; `IF NUM > "040"`
  → **true**.
- **Lowering (`emit_relation` / `num_digit_str_operand`).** In the mixed arm the
  numeric side's digit image is built by **reusing `emit_num_digit_string`** (the
  same run-time image the numeric→alphanumeric `MOVE` builds) as a fixed-length
  `StrOperand`, and **both** operands are then fed through the **same**
  `emit_str_condition` path (space-pad each side to their common length, `str_cmp`,
  compare the ordering against `0`) a same-category alphanumeric relation uses —
  so the emitted byte comparison is identical to the oracle's.
- **Either side, item or literal.** The numeric operand may be the left or the
  right operand; the alphanumeric operand may be a `PIC X` item or a string
  literal. `EVALUATE`'s subject-vs-`WHEN` comparison in the oracle reuses the same
  `compare_operands`, so it benefits identically; the compiler's `EVALUATE` mixed
  lowering stays a later rung (its subject/`WHEN` paths are same-category only).
- **Deferral gate.** Only an unsigned-integer numeric item has an unambiguous
  image on this rung. A **signed** (`PIC S9`) or **scaled** (`PIC 9V9`) numeric
  operand (`num_digit_str_operand` → `Unsupported`), a **group** item on either
  side (its name is unregistered → `item_index` → `Unsupported`), and a
  **numeric-literal-vs-alphanumeric** pairing (a different pairing, kept out of
  scope → `Unsupported`) are all clean later rungs — matching the oracle, which
  rejects the same shapes.
- **Tests.** Seven `jit_e2e.rs` cases through `assert_matches_oracle`
  (`=` match / space-pad mismatch / `>` ordering / numeric on the right / against
  a `PIC X` item / symbolic `>=` and `<` / wider field) plus three compiler-unit
  rejects (signed, scaled, group).

### Added — v0.28.0: reverse cross-category MOVE (alphanumeric → unsigned-integer numeric)

The **reverse** cross-category `MOVE`: `MOVE alphanumeric-item TO numeric-item`,
restricted to an alphanumeric source (`PIC X(m)`) into an **unsigned integer**
receiver (`PIC 9(n)` — no `S`, no `V`). Oracle-first and byte-identical to
`cobol-runtime` 0.32.0. No grammar change was needed — `MOVE` already parses and
this reuses existing IIR ops.

- **The rule.** COBOL reads the alphanumeric source's `m` characters as an
  unsigned integer and de-scales it into the numeric receiver **right-justified**:
  the receiver keeps the **low-order `n` digits** — left-zero-padded when the
  source has fewer than `n` digits, high-order-truncated when more —
  i.e. `receiver = (integer formed from the m source chars) mod 10^n`. So
  `X(3)="042"` → `9(3)` is `42` (displays `"042"`), `X(2)="05"` → `9(4)` is `0005`,
  `X(5)="12345"` → `9(3)` is `345`.
- **Lowering (`emit_str_to_int`).** The `m` source bytes are folded left-to-right
  into an `i64`: for the character at position `k`, `d = str_index(src,k) - '0'`
  (each byte read with the IIR `str_index` op, the constant `'0'`=48 subtracted),
  then `value = value*10 + d` (`mul`/`add`). The folded `value` is stored through
  the **same** numeric-store helper a numeric `MOVE`/`COMPUTE` uses
  (`store_scaled` at scale 0), whose `mod 10^(int+dec)` applies the receiver-width
  truncation — so the compiled result matches the oracle, which folds the identical
  per-character arithmetic and stores via `move_into_numeric` at scale 0.
- **All-digit scope / non-digit choice.** This rung scopes to an **all-digit**
  source. A non-digit byte is *not* rejected: the same `(byte - '0')` arithmetic
  runs on both engines (defined-but-unspecified, identical by construction), so a
  clean identical runtime reject is unnecessary and no test exercises it. This was
  chosen over a runtime reject because the compiled path has no clean way to raise
  a runtime error for a non-digit that the oracle could mirror byte-for-byte.
- **Overflow guard.** An `i64` fold of an all-digit source of `≤ 18` characters
  stays below `10^18 < i64::MAX`, so it never overflows on either engine; a source
  **wider than 18 characters** is a clean `Unsupported` later rung, rejected
  identically on both engines.
- **Deferral gate.** A **signed** (`PIC S9`) or **scaled** (`PIC 9V9`) numeric
  receiver, and a **group** item on either side, remain clean `Unsupported` later
  rungs (only `(ItemKind::Char, ItemKind::Numeric { signed:false, dec_digits:0 })`
  is admitted; every other cross-category shape falls to the catch-all reject).
- **Tests.** Seven `jit_e2e.rs` cases through `assert_matches_oracle` (exact-fit,
  shorter source zero-pads, longer source high-order-truncates, single digit,
  MOVE-then-`ADD`, MOVE-then-`COMPUTE`, and a numeric→alpha→numeric round-trip),
  plus compiler-unit tests: the reverse move now lowers (asserting `str_index` +
  `mod`), and clean rejects for a signed receiver, a scaled receiver, a >18-char
  source, and a group source.

### Added — v0.27.0: cross-category MOVE (unsigned-integer numeric → alphanumeric)

The first **cross-category** `MOVE`: `MOVE numeric-item TO alphanumeric-item`,
restricted to an **unsigned integer** source (`PIC 9(n)` — no `S`, no `V`) into a
`PIC X(m)` receiver. Oracle-first and byte-identical to `cobol-runtime` 0.31.0. No
grammar change was needed — `MOVE` already parses and this reuses existing IIR ops.

- **The rule.** COBOL treats a numeric sending item moved to an alphanumeric
  receiver as though it were an alphanumeric item holding its **digit characters**
  — the item's `n`-digit zero-padded magnitude, exactly what `DISPLAY` prints —
  then moves it by the alphanumeric rules: **LEFT-justified**, space-padded on the
  right when the receiver is wider, truncated on the right when narrower. So
  `PIC 9(3)` holding `42` (image `"042"`) → `X(3)` is `"042"`, → `X(5)` is
  `"042  "`, → `X(2)` is `"04"`.
- **Lowering (`emit_num_digit_string`).** The `n`-character digit image is built at
  run time from the numeric slot: for each position the digit is
  `(slot / 10^k) % 10`, sliced out of a constant `"0123456789"` table
  (`str_slice [d, d+1)`) and concatenated onto an accumulator — no per-digit branch
  table. The `% 10` per position gives COBOL's silent high-order truncation, the
  same as the recursive `__cob_print_padded` DISPLAY helper.
- **Char reshape (`move_str_into_char`).** The `n`-wide digit string is then stored
  into the receiver through the **same `str_slice`/`str_concat` reshape** a
  same-category alphanumeric `MOVE` (`move_char_item`) uses — the string-source
  twin of that helper. Both funnel through the one alphanumeric-receiver rule the
  oracle's `move_into_char` performs, so the stored bytes agree.
- **Deferred (clean `Unsupported`, never wrong output):** the **reverse** direction
  (alphanumeric → numeric), a **signed** (`PIC S9`) or **scaled** (`PIC 9V9`) or
  **edited** numeric source, a **numeric-edited** receiver, and a **group** item on
  either side.
- **Tests.** Six `jit_e2e.rs` cases through `assert_matches_oracle` (exact-fit, pad,
  truncate, single digit `PIC 9`, a computed source via `ADD`, and a MOVE result
  compared alphanumerically), plus compiler-unit tests for the supported lowering
  and each deferred reject (signed / scaled source, alpha → numeric).

### Added — v0.26.0: computed (data-name) reference modification

Generalised reference modification `IDENT(start:len)` to accept **data-name**
(run-time integer) indices — `WS(J:K)`, `WS(J:)`, `WS(2:K)` — oracle-first and
byte-identical to `cobol-runtime` 0.30.0. No grammar change was needed: the
indices already parse as `operand`s; only the readers rejected non-literals.

- **Data model.** `Operandy::RefMod` / `Operand::RefMod` now carry `start`/`len`
  as a new `RefIndex` (`Lit(usize)` **or** `Name(String)`) instead of raw
  `usize`, so the literal and computed cases flow through one path.
  `read_refmod_index` returns a `RefIndex`; a bare `NAME` index is
  `RefIndex::Name`, an integer literal is `RefIndex::Lit`, and a
  signed/fractional literal or nested reference modification as the index is a
  clean later-rung reject.
- **`ref_mod_slice`** now returns `(reg, SliceLen)`:
  - **literal:literal** (and `literal:`) is **constant-folded exactly as before**
    (`const_refmod_len` validates the range at compile time and rejects an
    out-of-range constant slice — #8673's behaviour is preserved verbatim);
  - **computed** — the moment either index is a data-name — reads each index into
    an `i64` register (`refmod_index_reg`: a `const` for a literal, a `mov` of the
    live slot for an unsigned-integer item) and builds `start0 = start - 1` and
    `end = start0 + len` (or `end = width` for an omitted length) with `sub`/`add`,
    feeding a run-time `str_slice(src, start0, end)`. The slice's run-time length
    (`end - start0`) rides along as a register.
- **Out-of-range rule.** The emitted `str_slice` traps in the VM/wasm backends
  exactly when `start0 < 0 || end < start0 || end > width`; the oracle's
  `refmod_string` applies the identical predicate (returning `RefModOutOfRange`),
  so an in-range program slices byte-identically and an out-of-range one errors on
  both engines.
- **Comparison contexts.** `StrOperand` gained a `Runtime { reg, len_reg, max_len }`
  variant for a computed slice whose length is only known at run time.
  `emit_str_condition` now sizes the common comparison width from each operand's
  compile-time upper bound and space-pads each side to it — a `Fixed`/`Fig` side
  at compile time (`pad_spaces`/`fig_const`), a `Runtime` side at run time
  (`pad_runtime`, slicing a max-width space constant to the run-time pad count, the
  same trick UNSTRING uses). Padding both sides to any common width ≥ their actual
  lengths gives the same `str_cmp` result COBOL's max-of-actual-lengths padding
  does, so a run-time-length slice compares byte-identically to the oracle.
- **Deferred (unchanged):** a signed/fractional/non-numeric reference-modification
  index item, reference modification of a numeric item, and use in a
  numeric/arithmetic/`MOVE`-source context remain later rungs.
- Tests: 9 new `jit_e2e` cases (computed mid-substring, omitted length, mixed
  literal-start/data-name-length, `IF` comparison, `EVALUATE` subject,
  `COMPUTE`-driven index, an equal/unequal computed-vs-computed comparison, and
  two out-of-range cases asserting **both** engines trap), plus compiler-unit
  tests for the computed lowering and the still-deferred signed/fractional index,
  numeric base, and MOVE-source rejects. All #8673 literal-refmod tests still pass.

### Added — v0.25.0: `INSPECT … CONVERTING from TO to`

Lowered the `INSPECT … CONVERTING` verb — a per-character translation table —
oracle-first and byte-identical to `cobol-runtime` 0.29.0.

- **`emit_inspect_converting`** — dispatched from `emit_inspect` on the standalone
  `inspect_converting` node (checked before the tally/replace composition, since
  the grammar never lets `CONVERTING` sit beside `TALLYING`/`REPLACING`). The
  `from`/`to` string literals give a compile-time table: each `from[k]` is baked as
  a `const` compare byte and each `to[k]` as a 1-character `str_const`. The lowering
  UNROLLS over the compile-time source width `W`, and at each position reads
  `S[j]` once and runs a **first-match-wins** chain over the table — on the earliest
  `from[k]` equal to `S[j]` it appends `to[k]` and jumps past the rest; if nothing
  matches it appends the original `S[j, j+1)`. The `W`-wide accumulator is copied
  back into the source register only after the last read (no read-after-write
  hazard), exactly as `emit_inspect_replacing` does.
- **First-match-wins** mirrors the oracle's char→char map (which lets the earliest
  `from` occurrence win via `or_insert`), so a duplicated `from` character (e.g.
  `CONVERTING "AAB" TO "XYZ"` → A→X, not A→Y) is byte-identical between the two.
- Later rungs (clean `CompileError::Unsupported`): an unequal-length or non-ASCII
  `from`/`to` pair, a data-name (`PIC X` item) / figurative / numeric-literal /
  reference-modified `from`/`to`, and a `BEFORE`/`AFTER` region. A `CONVERTING`
  combined with `TALLYING`/`REPLACING` in one statement does not parse (mutually
  exclusive grammar alternatives), so it is a `CompileError::Parse` rejection.

### Added — v0.24.0: combined `INSPECT … TALLYING … REPLACING` (one statement)

Lowered the combined `INSPECT` — one statement carrying BOTH the `TALLYING` and
the `REPLACING` phrases — oracle-first and byte-identical to `cobol-runtime`
0.28.0.

- **`emit_inspect` dispatch** — when both phrases are present, `emit_inspect` now
  composes the two existing lowerings on the SAME source register in ISO order:
  the tally loop FIRST (reading the ORIGINAL bytes into the counter), then the
  replace rebuild (overwriting the source). Per the standard the combined form
  executes "as though an `INSPECT TALLYING` were specified, followed by an
  `INSPECT REPLACING`", so counting before replacing is what makes a shared
  delimiter/search character correct — the count sees every occurrence before any
  is substituted.
- **`emit_inspect_tallying`** — the count loop and counter store were factored out
  of `emit_inspect` into this helper so the combined case reuses it verbatim (no
  duplicated logic); the lone-`TALLYING` path calls the same helper. The tally
  loop only reads the source register, so a following `REPLACING` still sees the
  original image.
- No grammar/lexer/parser change: the grammar already accepted
  `inspect_tallying [inspect_replacing]`; only the two `has_tally && has_repl`
  rejects (in the compiler and the oracle) were removed.
- Later rungs unchanged (clean `CompileError::Unsupported`): a combined statement
  whose `TALLYING` half is `LEADING`/`CHARACTERS`, has several counters or FOR
  phrases, or a `BEFORE`/`AFTER` region — or whose `REPLACING` half is
  `CHARACTERS`/`LEADING`/`FIRST`, has several replace items, or a region — still
  rejects; the combined gate does not admit the deferred sub-forms. Multi-char /
  figurative / wider / numeric operands and a numeric/group source stay deferred.
- Tests: four new `jit_e2e` cases through `assert_matches_oracle` — distinct
  tally/search/replace chars, the `delim == search` ordering case (proving
  tally-before-replace), a non-zero counter (ADD preserved), and tallied/replaced
  chars at the source's ends — plus a still-deferred combined reject
  (`FOR ALL … ALL …`). The former "combined is a later rung" unit test became a
  positive "combined now compiles" test.

### Added — v0.23.0: `INSPECT … REPLACING ALL … BY …` (first rung)

Lowered COBOL's `INSPECT … REPLACING` verb (the substitution form), oracle-first
and byte-identical to `cobol-runtime` 0.27.0.

- **`emit_inspect_replacing`** — `INSPECT source REPLACING ALL x BY y` rebuilds the
  alphanumeric `source` **in place**, replacing every occurrence of the SINGLE
  character `x` with the SINGLE character `y`. Because both are single characters
  the width `W` is unchanged, so this is a **per-position map** that the compiler
  **unrolls** over the compile-time-known `W`: at each position `j`, `str_index`
  reads the source byte, `cmp_eq` tests it against the search byte, and a branch
  splices either the replacement (`y`, a 1-char string) or the original character
  (`str_slice(S, j, j+1)`) onto a `str_concat` accumulator. The `W`-wide result is
  copied into the source register — the same fixed-width image the oracle's
  `move_into` produces, byte-for-byte.
- **`single_delim_str`** — parallel of `single_delim_code`: reduces a
  single-character operand to a 1-char **string** register (a `str_const` for a
  1-char literal, or the item register for a `PIC X(1)` item) for the
  concatenation. The search `x` still reduces to a byte code via the shared
  `single_delim_code`; both share the single-character validation.
- **`emit_inspect` dispatch** — the shared source parsing now branches to the
  `TALLYING` or `REPLACING` lowering; the combined `TALLYING … REPLACING` in one
  `INSPECT` is rejected up front.
- **Later rungs** (clean `CompileError::Unsupported`): `REPLACING CHARACTERS BY`,
  `REPLACING LEADING`/`FIRST`, `BEFORE`/`AFTER` regions, several replace items, the
  combined `TALLYING … REPLACING`, a multi-character / figurative / numeric /
  wider-than-one search or replacement, and a numeric/group source.
- **Tests**: six `jit_e2e.rs` oracle-match cases (a repeated char; an absent char
  leaving the source unchanged; every character replaced; a `PIC X(1)` search and
  replacement; the char at both ends; the `END-INSPECT` form) plus compiler-unit
  tests for the happy-path rebuild and the `CHARACTERS`, `LEADING`,
  multi-character-search, several-items, and combined-`TALLYING`-`REPLACING`
  rejects.

### Added — v0.22.0: `INSPECT … TALLYING … FOR ALL` (first rung)

Lowered COBOL's `INSPECT … TALLYING` verb, oracle-first and byte-identical to
`cobol-runtime` 0.26.0.

- **`emit_inspect`** — `INSPECT source TALLYING counter FOR ALL delim` counts the
  (non-overlapping, left-to-right) occurrences of the SINGLE-character `delim` in
  the alphanumeric `source` and **ADDs** that count to the integer `counter`.
  Like `UNSTRING` the delimiter position is data-dependent, so it emits a genuine
  **scan loop**: `len = str_len(S)`, a cursor `j` (i64, init 0), and a count
  accumulator `cnt` (i64, init 0); at each position `S[j]` (read with `str_index`)
  is compared to the delimiter byte `D` (`cmp_eq`) and `cnt` is bumped on a match,
  looping while `j < len` (`cmp_ge`). The count is folded into the counter with
  the SAME numeric-store path `ADD` uses (`store_scaled`), so INSPECT **adds** to
  the counter (it does not clear it first) and a compiled program matches the
  oracle's `store_result(counter, counter + cnt)` byte-for-byte. The delimiter
  reduces to a single byte code via the shared `single_delim_code` (renamed from
  `unstring_delim_code`).
- **Later rungs** (clean `CompileError::Unsupported`, accepted by the grammar and
  rejected here): `FOR LEADING` / `FOR CHARACTERS` tallies, `BEFORE`/`AFTER`
  regions, several `TALLYING` counters or `FOR` phrases, any `REPLACING`
  (`INSPECT … REPLACING` and `INSPECT … TALLYING … REPLACING`), a multi-character /
  figurative / numeric / wider-than-one delimiter, and a numeric source or a
  non-integer/signed counter.
- **Tests**: six `jit_e2e.rs` oracle-match cases (count a char; zero occurrences;
  a non-zero starting counter proving ADD-not-replace; a `PIC X(1)` delimiter
  item; every character matches; the delimiter at both ends with `END-INSPECT`)
  plus compiler-unit tests for the happy-path lowering and the `REPLACING`,
  `TALLYING … REPLACING`, multi-character-delimiter, and `LEADING` rejects.

### Added — v0.21.0: `UNSTRING … DELIMITED BY … INTO` (first rung)

Lowered COBOL's `UNSTRING` verb, oracle-first and byte-identical to
`cobol-runtime` 0.25.0.

- **`emit_unstring`** — the inverse of `STRING`. Where `STRING`'s field
  boundaries are all compile-time-known, `UNSTRING`'s delimiter falls wherever the
  run-time bytes put it, so it emits a genuine **scan loop**. The source register
  `S`, its length `len = str_len(S)`, and a cursor `p` (i64, init 0) drive the
  statement; the delimiter reduces to a single byte code `D` (a `const` for a
  1-char literal, or `str_index(item, 0)` for a `PIC X(1)` item). Each receiver
  (a compile-time-known `n`) unrolls to a block guarded by `if p <= len`: scan
  `S[j]` from `p` for the next byte equal to `D` (or end-of-source) with
  `str_index`/`cmp_eq`/`cmp_ge`, cut the field `piece = str_slice(S, p, q)`, reshape
  it into the receiver as `str_slice(piece, 0, min(str_len(piece), W)) ++
  spaces(W - take)` (exactly the oracle's alphanumeric `move_into` — left-justify,
  space-pad, truncate), and advance `p = q + 1`. Because `p` never moves when a
  receiver is skipped, an exhausted source (`p > len`) leaves this and every later
  receiver unchanged; `p == len` (a trailing delimiter) still yields one final
  empty field.
- Later rungs (clean `CompileError::Unsupported`): `WITH POINTER`, `ON`/`NOT ON
  OVERFLOW`, a multi-character delimiter, a numeric/figurative/reference-modified
  delimiter, a delimiter item wider than one character, and a numeric/group source
  or receiver.

### Added — v0.20.0: `STRING … DELIMITED BY SIZE INTO` (first rung)

Lowered COBOL's `STRING` verb, oracle-first and byte-identical to
`cobol-runtime` 0.24.0.

- **`emit_string`** — concatenates the sending fields with a `str_concat` chain
  (each source is a `(register, compile-time length)` pair: an alphanumeric item's
  slot, or a `str_const` for a string / numeric literal), then overlays the result
  onto the receiver. The overlay honours COBOL's no-space-fill rule: when the
  concatenation is at least as wide as the receiver it is truncated
  (`t = str_slice(concat, 0, width)`); when shorter, the receiver's old tail is
  preserved (`t = str_concat(concat, str_slice(t, len, width))`). All indices are
  compile-time constants, mirroring the exact bytes the oracle's `exec_string`
  writes.
- Only `DELIMITED BY SIZE` this rung. A real (identifier/literal) delimiter,
  `WITH POINTER`, `ON`/`NOT ON OVERFLOW`, a numeric item as a sending field, a
  figurative sending field, and a non-alphanumeric receiver are clean
  `CompileError::Unsupported` "later rung" errors (the grammar accepts the
  delimiter/POINTER/OVERFLOW syntax so the rejection is friendly, not a parse
  error).
- Grammar/lexer: new `string_stmt` rule and the `STRING`/`DELIMITED`/`WITH`/
  `POINTER`/`OVERFLOW`/`END-STRING` keywords (`cobol-parser` 0.15.0 /
  `cobol-lexer` 0.7.0).
- Tests: six `jit_e2e` cases (concatenation, truncation, a literal source, a
  full-width item with its spaces, the no-space-fill tail, a numeric literal
  source) plus unit tests for the str-op lowering and the `DELIMITED BY <delim>`
  and `WITH POINTER` later-rung errors.

### Added — v0.19.0: reference modification `IDENT(start:len)` (PL09 step 5)

COBOL **reference modification** — `base(start:len)` selects `len` characters of
alphanumeric item `base` from 1-based position `start`; `base(start:)` (omitted
length) runs to the end of the item. Supported this rung with **constant integer
NUMBER-literal** start/length, on an alphanumeric (PIC X) base, in **DISPLAY**
operands and **IF / EVALUATE alphanumeric-comparison** operands (either side, or
against a literal). Implemented oracle-first, byte-identical.

- **Grammar/lexer.** New `COLON = ":"` token (`cobol-lexer` 0.6.0) and an optional
  reference-modification suffix on the `operand` rule (`cobol-parser` 0.14.0):
  `operand = NAME [ LPAREN operand COLON [ operand ] RPAREN ] | literal`. A bare
  NAME still parses exactly as before.
- **Reader.** `Operandy` gains `RefMod { base, start, len }`. `read_operand`
  detects the suffix (nested `operand` child nodes) and reads each index via
  `read_refmod_index`, which requires a plain integer NUMBER literal — a
  data-name/expression start or length is a *computed* reference modification, a
  clean `Unsupported` later rung.
- **Lowering.** A shared `ref_mod_slice(base, start, len) -> (reg, actual_len)`
  helper resolves the base to a `Char` item, computes `start0 = start-1` and
  `actual_len = len.unwrap_or(width - start0)`, validates bounds at compile time
  (`start >= 1`, `start-1+len <= width`), and emits a constant-index `str_slice`
  (mirroring `move_char_item`). `str_operand` and `emit_display` reuse it.
- **Deferral.** Reference modification of a numeric item, a computed start/length,
  and any numeric/arithmetic/MOVE-source use are `Unsupported` later rungs; an
  out-of-range **constant** reference modification is rejected at compile time,
  never lowered to a runtime trap.
- **Tests.** New `jit_e2e.rs` cases (DISPLAY of `WS(2:3)`, `WS(3:)`, `WS(1:1)`,
  full-width; `IF WS(1:3) = "ABC"`; `EVALUATE WS(2:2)`; ref-mod vs ref-mod) all
  byte-identical to the oracle; unit tests for the `str_slice` lowering and the
  computed-start / numeric-item / out-of-range `Unsupported` paths.

### Added — v0.18.0: alphanumeric `EVALUATE` subject (PL09 step 4)

`EVALUATE` now works over a **character** subject, not just numeric:
`EVALUATE GRADE WHEN "A" DISPLAY … WHEN "A" THRU "M" DISPLAY …`. Implemented
oracle-first, byte-identical, reusing the alphanumeric-IF `str_cmp` machinery.

- **No grammar/lexer change** — `EVALUATE`'s operands already cover string
  literals and character items.
- **Lowering.** `emit_evaluate` classifies the subject via `str_operand`: a
  character subject takes `emit_when_match_str`, which compares each `WHEN` value
  with `emit_str_condition` (space-pad + `str_cmp` + `cmp_* vs 0`) — a single value
  is `cmp_eq`, a `THRU` range is `and(cmp_ge, cmp_le)` — OR-folded, exactly like the
  numeric path but over strings. `str_cmp` is the same op alphanumeric `IF` uses, so
  every backend that runs strings already accepts it.
- **Deferral.** A numeric `WHEN` value against a character subject (or vice versa)
  is a later rung, matching a relation's numeric-vs-alphanumeric deferral.
- **Tests.** A new `jit_e2e.rs` case byte-identical to the oracle (character subject
  by value, `WHEN OTHER`, and a `THRU` range at both boundaries); unit tests for the
  str_cmp lowering and the numeric-value deferral (the prior "alphanumeric EVALUATE
  deferred" test now asserts it lowers); a `backend_compat` program; a `lang_matrix`
  alphanumeric `EVALUATE` row across all seven columns. Full suite **159 green** (38
  unit + 22 `backend_compat` + 99 `jit_e2e`). The oracle gains 2 alphanumeric
  `EVALUATE` unit tests.

### Added — v0.17.0: `EVALUATE` multiple values and `THRU` ranges per `WHEN` (PL09 step 4)

A `WHEN` can now list several values and inclusive `THRU` ranges:
`EVALUATE N WHEN 1 5 THRU 7 9 …`. Implemented oracle-first, byte-identical, reusing
the level-88-ranges boolean machinery.

- **Grammar (no lexer change).** `when_branch = "WHEN" ( "OTHER" | when_value
  { when_value } )`, `when_value = operand [ (THRU|THROUGH) operand ]`
  (`cobol-parser` 0.13.0). `THRU`/`THROUGH` were already reserved.
- **Lowering.** `emit_when_match` builds each `WHEN`'s boolean by **OR-folding** its
  value-list — a single value is `cmp_eq(subject, value)`, a `THRU` range is
  `and(cmp_ge, cmp_le)` — exactly the bitwise `and`/`or`-on-`0`/`1` machinery
  level-88 ranges use; the folded boolean feeds the existing `jmp_if_false` cascade.
  No new opcode; every backend already accepts it. Values within a `WHEN` are
  emitted by iteration, so a `WHEN` with thousands of values stays flat.
- **Deferral.** An alphanumeric subject/value is still a later rung. (`EVALUATE
  TRUE`, multiple subjects remain deferred.)
- **Tests.** A new `jit_e2e.rs` case byte-identical to the oracle (a multi-value
  `WHEN`; a `THRU` range at both boundaries; a mixed singles+range `WHEN`); a unit
  test asserting the `or`/`and`/`cmp_ge`/`cmp_le` fold; a `backend_compat` program; a
  `lang_matrix` multi-value/range `EVALUATE` row across all seven columns. Full
  suite **156 green** (37 unit + 21 `backend_compat` + 98 `jit_e2e`). The oracle
  gains 3 multi-value/range unit tests.

### Added — v0.16.0: `EVALUATE` (case statement, simple form) (PL09 step 4)

COBOL's case statement: `EVALUATE N WHEN 1 … WHEN 5 … WHEN OTHER … END-EVALUATE`.
Implemented oracle-first (lexer → grammar → `cobol-runtime` → this compiler),
byte-identical.

- **Lexer/grammar.** New `EVALUATE`/`OTHER`/`END-EVALUATE` keywords (`cobol-lexer`
  0.5.0; `END-EVALUATE` is hyphenated like `WORKING-STORAGE`); grammar
  `evaluate_stmt = "EVALUATE" operand { when_branch } "END-EVALUATE"`,
  `when_branch = "WHEN" ( "OTHER" | operand ) { statement }` (`cobol-parser` 0.12.0).
  Both generated files regenerated via `grammar-tools`.
- **Lowering.** `emit_evaluate` lowers to a **`cmp_eq` + `jmp_if_false` branch
  cascade** — the same ops `IF` uses, no new opcode. Each value `WHEN` compares the
  subject to its value at a common scale; a mismatch jumps to the next branch, a
  match runs the branch and jumps to the end (no fall-through). `WHEN OTHER` runs
  unconditionally once reached. Branches are emitted by **iteration**, so thousands
  of `WHEN`s stay flat. Every backend accepts the cascade.
- **Deferrals (clean errors).** An alphanumeric subject/value ([`read_arith_term`]
  rejects it), and multiple-value / `THRU` / `EVALUATE TRUE` / multi-subject forms.
- **Tests.** 2 new `jit_e2e.rs` cases byte-identical to the oracle (match a value /
  `WHEN OTHER` / no-match-no-OTHER / `STOP RUN` in a branch; a scaled subject); unit
  tests for the cmp_eq-cascade shape and the alphanumeric deferral; a
  `backend_compat` program; a `lang_matrix` `EVALUATE` row across all seven columns.
  Full suite **153 green** (36 unit + 20 `backend_compat` + 97 `jit_e2e`). The oracle
  gains 5 unit tests including a 2000-`WHEN` DoS regression (iterates, never
  overflows).

### Added — v0.15.0: `NOT` over a condition (PL09 step 4)

`NOT` now negates a whole condition — a relation, a level-88 condition-name, or a
parenthesised group: `IF NOT (A AND B)`, `IF NOT IS-OK`, `IF N > 0 AND NOT N > 9`.
This completes the condition story (word/symbolic relations, `AND`/`OR`,
parentheses, and now `NOT`). Implemented oracle-first, byte-identical.

- **Grammar (no lexer change).** A `negation = [ "NOT" ] simple_condition` layer
  sits between `conjunction` and `simple_condition`, so `NOT` binds tighter than
  `AND`/`OR` (`cobol-parser` 0.11.0). A relation's own `IS NOT …` still works and
  never collides (negation `NOT` precedes the first operand; relop `NOT` is between
  operands).
- **Lowering.** `emit_negation` inverts the leaf's `0`/`1` boolean with **`xor`
  against `1`** (`0^1=1`, `1^1=0`) — the logical NOT. IIR's `not` is *bitwise*
  (`~x`), which would not map `0`/`1` to `1`/`0`, so `xor` is the right op; the
  result is still `0`/`1` and feeds `jmp_if_false` like any condition boolean. This
  is **the first COBOL program to emit `xor`** — every print backend (wasm/jvm/clr)
  accepts it, as do native-AOT/LLVM/VM/JIT.
- **Byte-identical** to the oracle's `!eval_cond` (a comparison never faults and
  has no side effects).
- **Tests.** A new `jit_e2e.rs` case byte-identical to the oracle (NOT over a
  relation; de Morgan over a parenthesised group; NOT vs `AND`/`OR` precedence;
  double negation with a relop-level `NOT`); a unit test asserting `xor` is emitted;
  a `backend_compat` program; a `lang_matrix` COBOL `NOT` row across all seven
  columns. Full suite **148 green** (34 unit + 19 `backend_compat` + 95 `jit_e2e`).
  The oracle gains a `NOT` unit test.

### Added — v0.14.0: compound conditions (`AND` / `OR` / parentheses) (PL09 step 4)

`IF` and `PERFORM … UNTIL` conditions can now combine simple conditions with
`AND`/`OR` and parentheses: `IF N > 3 AND N < 9`, `IF (A OR B) AND C`. Implemented
oracle-first (grammar → `cobol-runtime` → this compiler), byte-identical.

- **Grammar (no lexer change).** `condition` became a precedence cascade —
  `disjunction` (`OR`) of `conjunction`s (`AND`) of `simple_condition`s (relation /
  condition-name / parenthesised `condition`) — so `AND` binds tighter than `OR`
  (`cobol-parser` 0.10.0). `AND`/`OR`/`(`/`)` were already tokens.
- **Lowering.** `emit_condition` recurses the cascade: each leaf already yields a
  `0`/`1` boolean, and `AND`/`OR` fold with the bitwise `and`/`or` ops (the same
  machinery level-88 ranges use) — exactly logical AND/OR on `0`/`1`, feeding
  `jmp_if_false` unchanged, **no new opcode**. This is **byte-identical to the
  oracle's short-circuit `&&`/`||`**: COBOL relations here have no side effects and
  a comparison never faults, so the compiler's full evaluation gives the same
  boolean.
- **Deferral.** `NOT` over a whole compound/parenthesised condition stays a later
  rung (`NOT` remains relation-level via the `relop`).
- **Tests.** 2 new `jit_e2e.rs` cases byte-identical to the oracle (the AND/OR +
  precedence + parentheses truth table; a condition-name combined with a relation);
  a unit test asserting the `and`/`or` fold; a `backend_compat` program; a
  `lang_matrix` COBOL compound-condition row across all seven columns. Full suite
  **145 green** (33 unit + 18 `backend_compat` + 94 `jit_e2e`). The oracle gains a
  compound-condition unit test and a DoS regression test (a 5000-term flat `AND`
  chain evaluates by iteration, not recursion — see the `cobol-runtime` 0.18.0
  note; `Cond`'s `AND`/`OR` are a flat `Vec`, so the chain cannot overflow the
  stack).

### Added — v0.13.0: symbolic relational operators (`> < = >= <= <>`) (PL09 step 4)

Conditions can now be written with symbols as well as COBOL's word operators:
`IF N >= 5`, `IF X <> Y`, `PERFORM … UNTIL I > 9`. Implemented oracle-first (lexer
→ grammar → `cobol-runtime` → this compiler), byte-identical.

- **Lexer/grammar.** New tokens `GT`/`LT`/`GE`/`LE`/`NE` (`cobol-lexer` 0.4.0;
  `EQ` already existed), 2-char before 1-char for longest-match; the `relop` rule
  gains the symbolic alternatives (`cobol-parser` 0.9.0). Both generated files
  regenerated via `grammar-tools`.
- **Lowering.** `relation_op` now resolves each operator to a base relation plus a
  *baseline* negation — the symbols `>=`/`<=`/`<>` already mean "not <", "not >",
  "not =" — and a written `NOT` composes with that baseline by XOR. So `>=` lowers
  to `cmp_ge`, `<>` to `cmp_ne`, `NOT >=` to `cmp_lt`, etc. The symbols reduce onto
  the existing `cmp_*` set, so there is **no new opcode** and no change to the
  branch structure — every backend already accepts it.
- **Tests.** A new `jit_e2e.rs` case covering the whole symbol truth table
  (including the `>=`/`<=` range boundaries and `NOT >=` ≡ `<`), byte-identical to
  the oracle; a unit test asserting each symbol maps to the right `cmp_*`; a
  `backend_compat` program; a `lang_matrix` COBOL symbolic-relop row across all
  seven columns. Full suite **141 green** (32 unit + 17 `backend_compat` + 92
  `jit_e2e`). The oracle gains a symbolic-operator unit test.

### Added — v0.12.0: `SET cond-name TO TRUE` (PL09 step 4)

The setter counterpart to level-88's condition-name test: `SET IS-DONE TO TRUE`
assigns the conditional variable the value that makes the name hold. Implemented
oracle-first (lexer → grammar → `cobol-runtime` → this compiler), byte-identical.

- **Lexer/grammar.** `SET`/`TRUE` become keywords (`cobol-lexer` 0.3.0); the
  grammar gains `set_stmt = "SET" NAME "TO" "TRUE"` (`cobol-parser` 0.8.0). Both
  generated files regenerated via `grammar-tools`.
- **Lowering.** `emit_set` resolves the condition-name to its conditional variable
  and its **first** `VALUE` item (a range's low bound), formats that value into the
  variable's picture at compile time, and emits a single `const` store into the
  variable's slot — the same store `MOVE <literal>` uses, so no new opcode. Every
  print backend (wasm/jvm/clr) accepts it, as do native-AOT/LLVM/VM/JIT.
- **Deferrals / errors.** `SET … TO TRUE` on an alphanumeric conditional variable
  is a later rung; an undeclared condition-name is a clean error.
- **Tests.** 2 new `jit_e2e.rs` cases byte-identical to the oracle (assign the
  first value; a range's low bound); unit tests for the lowering and the undeclared
  error; a `backend_compat` program; a `lang_matrix` COBOL `SET` row across all
  seven columns. Full suite **138 green** (31 unit + 16 `backend_compat` + 91
  `jit_e2e`). The oracle gains 3 unit tests.

### Added — v0.11.0: level-88 multiple values and `THRU` ranges (PL09 step 4)

Level-88 condition-names now accept a **list** of values and inclusive **`THRU`
ranges** — `88 COND VALUE 1 5 THRU 7 9` holds when the variable is `1`, `5..=7`,
or `9`. Implemented oracle-first (grammar → `cobol-runtime` → this compiler),
byte-identical throughout.

- **Grammar.** `value_clause` became `"VALUE" [IS] value_item { value_item }` with
  `value_item = literal [ (THRU|THROUGH) literal ]` (regenerated via `grammar-tools
  compile-grammar`; `THRU`/`THROUGH` were already reserved). The clause is shared
  with plain items, so a multi-value/range `VALUE` on a non-88 item is rejected in
  both the oracle and this compiler.
- **Model.** `CondName` now holds `Vec<ValueSpec>` (`Single(Src)` | `Range(Src,
  Src)`); `read_value_specs` reads every `value_item`.
- **Lowering.** `emit_condition_name` resolves every value into the variable's
  scaled slot representation, then emits one boolean per item — `cmp_eq` for a
  single value, `and(cmp_ge, cmp_le)` for a range — and OR-folds them with `or`.
  Because each `cmp_*` yields `0`/`1`, the bitwise `and`/`or` are exactly logical
  AND/OR, and the combined `i64` feeds `jmp_if_false` like any relational
  condition. **This is the first COBOL program to emit the `and`/`or` ops** — all
  print backends (wasm/jvm/clr) accept them, as do native-AOT/LLVM/VM/JIT.
- **Deferrals (clean errors).** An alphanumeric conditional variable is still a
  later rung. (Multiple values and ranges are now supported.)
- **Tests.** 3 new `jit_e2e.rs` cases byte-identical to the oracle (multi-value OR
  hit/miss; range inclusive at both boundaries; mixed singles + range across the
  domain); unit tests for the multi/range lowering and the multi-value-on-plain-item
  rejection; a `backend_compat` program emitting `and`/`or`; a `lang_matrix` COBOL
  multi-value/range row across all seven columns. Full suite **133 green** (29 unit
  + 15 `backend_compat` + 89 `jit_e2e`). The oracle gains 4 unit tests.

### Added — v0.10.0: level-88 condition-names (PL09 step 4)

The first COBOL feature that reaches past `cobol-runtime`'s prior surface: a
**level-88 condition-name** — the boolean shorthand `IF IS-OK` for "does my
conditional variable hold the value that makes me true?". Implemented oracle-first
(grammar → `cobol-runtime` → this compiler) so it stays byte-identical.

- **Grammar.** `condition` became `relation | condition_name` in
  `code/grammars/cobol/cobol.grammar` (regenerated via `grammar-tools
  compile-grammar`); the relation is tried first and a bare condition-name falls
  through. A level-88 *entry* already parsed as a `data_entry`, so only the
  reference site changed.
- **Registration.** `collect_condition_name` records each `88 NAME VALUE lit.`
  against the item defined just before it (its conditional variable) — it takes no
  storage and no register.
- **Lowering.** `emit_condition` now dispatches: a `condition_name` lowers via
  `emit_condition_name`, which formats the value into the variable's picture at
  compile time (the same reuse `MOVE <literal>` relies on) and emits a single
  `const` + `cmp_eq` on the variable's scaled slot — the same boolean a relational
  `IF` produces, so it composes with `IF`/`ELSE` and `PERFORM … UNTIL` unchanged
  and needs no new opcode.
- **Deferrals (clean errors).** A condition-name whose conditional variable is
  alphanumeric (needs a string compare), multiple `VALUE`s, and `VALUE … THRU`
  ranges are later rungs — matching the oracle's own deferrals.
- **Tests.** 4 new `jit_e2e.rs` cases byte-identical to the oracle (true/false
  branches; tracking a live value after `MOVE`; a scaled conditional variable; a
  condition-name driving `PERFORM UNTIL`); unit tests for the lowering and the
  alphanumeric deferral; a `backend_compat` program (wasm/jvm/clr accept the
  const/cmp_eq); a `lang_matrix` COBOL condition-name row across all seven columns.
  Full suite **127 green** (27 unit + 14 `backend_compat` + 86 `jit_e2e`). The
  oracle gains 3 unit tests (numeric condition-name in `IF` and `PERFORM UNTIL`,
  plus the alphanumeric deferral).

### Added — v0.9.0: nested COMPUTE division (scale-12 intermediate) (PL09 step 4)

Division nested inside a larger `COMPUTE` expression (e.g. `A / B + C`) now lowers
— previously only a **top-level** `COMPUTE r = a / b` did, and a nested division
was a clean "later rung" error.

- **Semantics.** The oracle carries *every* COMPUTE division at a fixed
  intermediate precision of `COMPUTE_DIV_SCALE` (12 fractional digits), truncating
  toward zero, then lets the surrounding operators combine that scale-12 value
  exactly. `eval_div_nested` reproduces exactly that quotient —
  `numerator = a · 10^(b.scale + 12)`, `denominator = b · 10^(a.scale)`,
  `quotient = numerator / denominator` (i64 `div`, truncating toward zero) — and
  returns it at scale 12. Because dividing by a fraction can *grow* the integer
  part, the quotient's integer bound is `a.int_bound + a.scale + b.scale`. The
  scale-12 result then flows through the existing add/subtract/multiply and final
  round/truncate store, all already proven byte-identical to the oracle.
- **`COMPUTE_DIV_SCALE` is now re-exported from `cobol-runtime`**, so the frontend
  reproduces the oracle's *exact* intermediate scale rather than hard-coding a copy
  that could drift.
- **i64 guard.** The oracle keeps the scale-12 math exact in `i128`; here the
  intermediates are `i64`, so a case whose numerator (`a.int + a.scale + b.scale +
  12` digits) or denominator could exceed the 18-digit model is a clean later rung,
  never a silent wrap. This still covers the common small-operand computations.
- **Zero divisor.** A nested division's zero divisor faults the emitted `div`,
  matching the oracle's hard `DivideByZero`. Because routing a mid-expression zero
  divisor to an `ON SIZE ERROR` handler would need to wrap the whole evaluation in
  a skip (which this rung does not), a COMPUTE that pairs `ON SIZE ERROR` with a
  nested division stays a later rung. A **top-level** division with a handler still
  lowers as before.
- **Portability.** The scale-12 quotient is plain `const`/`mul`/`div` — no new
  opcode and no dynamic strings — so it rides the same scaled-i64 substrate every
  backend already accepts (wasm / jvm / clr / native-AOT / LLVM / VM / JIT).
- **Tests.** 4 new `jit_e2e.rs` cases byte-identical to the oracle (`A / B + C`;
  division as the right operand of `+`; `A / B * C` ROUNDED; a fractional
  dividend); unit tests for the nested-division lowering, the `ON SIZE ERROR`
  deferral, and the over-wide-intermediate deferral (the prior "nested division
  deferred" test now only pins the variable/negative/fractional/oversized `**`
  exponent); a `backend_compat` `A / B + C` program (wasm/jvm/clr accept the
  const/mul/div); a `lang_matrix` COBOL nested-division row across all seven
  columns. Full suite **120 green** (25 unit + 13 `backend_compat` + 82 `jit_e2e`).

### Added — v0.8.0: COMPUTE exponentiation (`**`) with a constant exponent (PL09 step 4)

`COMPUTE`'s last deferred operator now lowers for the case that covers almost all
real use: a `**` whose **exponent is a compile-time non-negative integer**.

- **Evaluation.** `AExpr::Pow` now carries its `(base, exponent)` subtrees (it was
  a bare placeholder that only earned a "later rung" error). `read_compute_factor`
  folds `A ** B ** C` **right-associatively** into `A ** (B ** C)`, matching the
  oracle's right-to-left `**`. `eval_pow` reads the exponent as a constant
  non-negative integer `e` (via `const_nonneg_int`, whose acceptance rule mirrors
  the oracle's `pow`: a non-zero fractional digit is rejected, and a negative sign
  only on a non-zero value) and **unrolls the power into `e − 1` register
  multiplies** of the base — because the oracle computes `base**e` by multiplying
  `1` by `base` `e` times, the result's magnitude is `base_scaled^e` and its scale
  is `e · base.scale`, exactly what the mul-chain produces. `x ** 0 = 1` is the
  constant integer one and never even reads the base (matching the oracle). The
  final product is guarded against the 18-digit i64 model, so an over-wide power is
  a clean [`CompileError::Unsupported`], never a silent wrap.
- **Portability.** The power lowers to a chain of plain `mul` ops — no new opcode
  and no dynamic strings — so it rides the same scaled-i64 substrate every other
  arithmetic backend already accepts (wasm / jvm / clr / native-AOT / LLVM / VM /
  JIT). A `**` whose exponent is a variable, a parenthesised expression, negative,
  fractional, or past the oracle's `MAX_POW_EXP` (1024) stays a later rung, as does
  a base+exponent whose conservative digit bound could exceed 18 digits.
- **Tests.** 6 new `jit_e2e.rs` cases (square / cube; `** 0` and `** 1`; a scaled
  base accumulating scale; a sub-expression base; `**` binding tighter than `*`;
  truncation of an overflowing power into a narrower receiver) each byte-identical
  to the oracle; unit tests for the literal-exponent lowering and for the newly
  pinned deferrals (variable / negative / fractional / oversized exponent, and the
  18-digit overflow guard); a `backend_compat` `A ** 3` program (wasm/jvm/clr accept
  the mul-chain); a `lang_matrix` COBOL `**` row across all seven columns. Full
  suite **112 green** (22 unit + 12 `backend_compat` + 78 `jit_e2e`).

### Added — v0.7.0: alphanumeric item MOVE and comparison (PL09 step 4)

The two most commonly-hit character-handling "later rung" errors now lower,
byte-identical to the oracle. Both reduce to **fixed-length** string ops because
a character item's stored image is always exactly its declared width and all
item sizes are known at compile time.

- **Character item-to-item `MOVE`** reshapes the source into the receiver's
  picture exactly as the oracle's `move_into_char`: keep the leftmost `N`
  characters when `N ≤ M` (one `str_slice`), else left-justify and space-pad on
  the right (one `str_concat`). Cross-category (`numeric↔alphanumeric`) moves
  stay a clean later rung.
- **Alphanumeric comparison in `IF`** — `emit_condition` now classifies each
  operand (numeric vs character) and dispatches. A character comparison
  space-pads both sides to their common (max) length and applies `str_cmp`
  (byte-lexical, matching the oracle's space-padded `String` compare);
  `SPACE`/`ZERO` figuratives expand to the other operand's length. `str_cmp`
  returns an `i64` ordering (−1/0/1), so the relation is applied with `cmp_* … 0`
  (no `Bool` mismatch). The relation→`cmp_*` mapping is shared with the numeric
  path via `relation_op`. A numeric operand compared with an alphanumeric one,
  and two figuratives with no fixed length to borrow, are later rungs.
- **Tests.** 8 new `jit_e2e.rs` cases (MOVE truncate / space-pad / same-size;
  equal / not-equal; lexicographic ordering; shorter-literal padding; `SPACES`
  figurative; move-then-compare round-trip) each byte-identical to the oracle;
  unit tests for validation and the deferred cross-category move; a
  `backend_compat` alphanumeric program (wasm/jvm/clr accept the `str_slice` /
  `str_concat` / `str_cmp` shapes); a `lang_matrix` COBOL row. Full suite **103
  green** (20 unit + 11 `backend_compat` + 72 `jit_e2e`).

### Added — v0.6.0: control flow, COMPUTE, ON SIZE ERROR, signed numerics (PL09 step 4)

A consolidated slice completing the COBOL-60 language surface this compiler
targets. Every feature is asserted **byte-identical to the `cobol-runtime`
oracle** via `jit_e2e.rs` (compile → run on the generic JIT → compare `DISPLAY`
bytes), and each deliberately-unimplemented corner is a clean
`CompileError::Unsupported`, never wrong output.

- **`GO TO para`** — an unconditional jump to a paragraph label. Every paragraph
  gets a `para_<name>` label; forward and back references both resolve.
- **`PERFORM`, all five forms** — `PERFORM p`, `p THRU q`, `n TIMES`,
  `UNTIL cond`, and `VARYING v FROM a BY b UNTIL cond`. The performed paragraph
  range is **inlined** at the call site, which reproduces COBOL's
  out-of-line-but-returns semantics exactly: a `STOP RUN` inside returns, a
  `GO TO` inside jumps away at top level. A recursive `PERFORM` or code-size
  blow-up trips a depth (`MAX_PERFORM_DEPTH`) / instruction (`MAX_EMIT_INSTRS`)
  bound as a clean error.
- **`ON SIZE ERROR`** on `ADD` / `SUBTRACT` / `MULTIPLY` / `DIVIDE` — routed
  through `store_scaled_handled`: when the (rounded, magnitude) result's integer
  part overflows the receiver, the handler statements run and the receiver is
  left unchanged; without a handler the high-order digits truncate silently
  (COBOL's handler-less rule). `DIVIDE` adds a zero-divisor guard that jumps to
  the handler (or faults, matching the oracle's `DivideByZero`, when there is
  none).
- **`COMPUTE target [ROUNDED] = <expr> [ON SIZE ERROR …]`** — the grammar's
  `arith_expr` precedence cascade lowered to the same scaled-`i64` model,
  evaluated bottom-up with a compile-time `(scale, int_bound)` on every node and
  an overflow guard on every combining step (an intermediate that could exceed 18
  digits is a clean error, never a silent wrap). `+ - *` and unary minus evaluate
  exactly (matching the oracle's exact `Decimal`); a top-level division reuses the
  `DIVIDE` verb's one-guard-digit rounding and zero-divisor branch. Division
  nested inside a larger expression, and `**`, are later rungs. The parser mirrors
  the oracle's associativity and shares its `MAX_EXPR_OPERANDS` stack-overflow
  bound.
- **Signed numerics (`PIC S9…`)** — a signed item keeps its sign in the `i64`
  slot (so `ADD` / `SUBTRACT` / `COMPUTE` and `IF` comparisons get signed
  semantics for free) and shows it as a trailing **overpunch** on `DISPLAY`
  (`{A-I}` / `{J-R}`, `'{'` / `'}'` for zero) via a synthesized
  `__cob_print_signed` helper. `MOVE` into an unsigned receiver drops the sign;
  silent high-order truncation re-applies the sign without relying on any
  backend's signed-remainder rule.
- **Tests.** 16 new `jit_e2e.rs` cases (9 GO TO/PERFORM, ON SIZE ERROR across the
  verbs, 9 COMPUTE, 6 signed) plus unit tests for validation and every deferred
  corner. Full suite **89 green** (19 unit + 6 `backend_compat` + 64 `jit_e2e`).

### Added — v0.5.0: scaled-decimal MULTIPLY / DIVIDE (PL09 step 4, PR3b)

- **Scaled `MULTIPLY`** on `PIC …V…` operands: the raw product of the two scaled
  `i64` slots carries scale `sa + sb`; `store_scaled` then rounds/truncates it to
  the receiver's scale. Each operand is ≤ 9 digits, so the product is `< 10^18`
  and never overflows `i64`.
- **Scaled `DIVIDE`**: the quotient is computed at working scale `w` = the
  receiver's `dec_digits` (plus one guard digit when `ROUNDED`) by scaling the
  dividend up before the truncating integer division —
  `floor(B·10^(sa+w−sb) / A)` — then `store_scaled` truncates or rounds `w →
  dec_digits`. One guard digit matches the oracle's `round()`, which inspects
  only the first dropped digit (half away from zero).
- **`ROUNDED`** is honoured on both, via the shared `store_scaled` bias-rounding.
- **Overflow-safe** (security-reviewed style): `DIVIDE` rejects a dividend with
  more fractional digits than the result precision, and any intermediate whose
  digit width would exceed 18 (`b_int + sa + w > 18`) — a clean `Unsupported`.
- **Still deferred**: `ON SIZE ERROR` on the arithmetic verbs (needs the `IF`
  rung's branch machinery).
- The now-redundant integer-only operand path (`int_operand` etc.) is removed —
  every arithmetic verb shares the scaled `Term` machinery and `store_scaled`.
- **Tests.** Unit tests for the new capability and the retained `ON SIZE ERROR`
  boundary; seven new `jit_e2e.rs` cases (fixed-point multiply truncate/round,
  BY-field update, divide truncate to receiver decimals, divide rounded, dividend
  update, decimal-field multiply) each byte-identical to the oracle; a scaled
  multiply/divide `backend_compat` program; and a `lang_matrix.rs` COBOL row
  (`20 / 3` rounded in `V99` → `0667`).

### Added — v0.4.0: IF / ELSE with relational conditions (PL09 step 4, PR4)

- **`IF condition then-stmts [ELSE else-stmts]`** → a three-way branch: the
  condition lowers to a boolean register, a `jmp_if_false` skips the then-branch
  to the else, and a `jmp` past it closes the then-branch. Nested `IF`s recurse.
- **Relational conditions** (`operand relop operand`): numeric comparison, with
  operands aligned to a common scale (the same implied-point machinery as ADD),
  then `cmp_gt` / `cmp_lt` / `cmp_eq`. **`NOT` inverts the relation directly**
  (NOT GREATER → `cmp_le`, NOT LESS → `cmp_ge`, NOT EQUAL → `cmp_ne`) — the
  `cmp_*` ops return a `Value::Bool` that `jmp_if_false` consumes, so inverting
  the boolean with an integer compare would be a type mismatch.
- **`STOP RUN` inside a branch** ends the program correctly (the branch's `ret`
  precedes the branch-closing jump, which is then unreachable).
- **Deferred** (clean `Unsupported`): alphanumeric comparison (space-padded
  string compare) is a later rung.
- **Tests.** Unit tests for the branch shape, the negation-as-cmp_le lowering,
  and the alphanumeric-comparison boundary; six new `jit_e2e.rs` cases
  (true/false branches, EQUAL/LESS/negated, multi-statement then, STOP-in-branch,
  scaled comparison by value, nested IF) each byte-identical to the oracle; an
  IF `backend_compat` program; and an IF `lang_matrix.rs` COBOL row (`5 > 3`
  → `BIG`).

### Added — v0.3.0: scaled-decimal ADD/SUBTRACT + item-to-item MOVE (PL09 step 4, PR3)

- **Scaled-decimal `ADD` / `SUBTRACT`** on `PIC …V…` fields. Terms are aligned to
  a common working scale (the largest fractional-digit count among the base field
  and operands, so every term scales up without loss), accumulated, then stored
  into the receiver at *its* scale.
- **`ROUNDED`** is now honoured on `ADD`/`SUBTRACT`: storing into a receiver with
  fewer decimals rounds **half away from zero** (via a sign-aware bias before the
  truncating divide); without it, the value truncates toward zero.
- **Numeric item-to-item `MOVE`** (`MOVE A TO B`) reshapes the source value into
  the receiver's picture — rescaling the implied point (truncating, never
  rounding). Alphanumeric item moves remain a later rung.
- **Unified store path.** A single `store_scaled` (rescale → magnitude → keep the
  low-order `int_digits + dec_digits` digits) backs every arithmetic verb and the
  item MOVE. `MULTIPLY`/`DIVIDE` now route through it too, so an integer product
  into a `V` receiver scales up correctly.
- **Honest boundaries** (clean `Unsupported`): **scaled** `MULTIPLY`/`DIVIDE`
  (a `V` operand) and their `ROUNDED`, plus `ON SIZE ERROR` (it needs the branch
  machinery of the `IF` rung), remain deferred.
- **Tests.** Unit tests for the new capability/error boundaries; six new
  `jit_e2e.rs` cases (implied-point alignment, higher-scale operand truncate vs
  round, unsigned decimal magnitude, cross-scale add, item MOVE reshape up/down)
  each asserted byte-identical to the oracle; a scaled `lang_matrix.rs` COBOL row.

### Added — v0.2.0: integer arithmetic (PL09 step 4, PR2)

- **Numeric items are now scaled `i64` slots** (PL09 D1): a `PIC 9…` item holds
  its value scaled by its fractional-digit count. This replaces v0.1's
  compile-time string image for numeric items (alphanumerics stay `str`), so
  values can be computed at run time. `MOVE`/`VALUE`/`DISPLAY` behaviour is
  unchanged and still oracle-exact (the scaled value is formatted through the new
  fixed-width digit helper `__cob_print_padded`).
- **`ADD` / `SUBTRACT` / `MULTIPLY` / `DIVIDE` (with `GIVING`)** on integer,
  unsigned fields → native `add` / `sub` / `mul` / `div` on the slots. The result
  is reduced to the receiver's field: magnitude (unsigned receivers drop the
  sign) and the low-order `int_digits` digits (COBOL's silent high-order overflow
  truncation). `DIVIDE` truncates toward zero.
- **Honest boundaries** (clean `CompileError::Unsupported`, never wrong output):
  scaled-decimal arithmetic (`PIC …V…`), `ROUNDED`, `ON SIZE ERROR`, arithmetic
  operands/receivers wider than 9 digits (`i64` product safety), and numeric
  fields wider than 18 digits (the `i64` value model).
- **Tests.** Unit tests for the arithmetic shape and error paths; `jit_e2e.rs`
  grows seven arithmetic cases (accumulate, GIVING, unsigned magnitude, multiply,
  truncating divide, silent overflow, a three-verb chain) each asserted
  byte-identical to the oracle; `backend_compat.rs` gains an arithmetic program;
  and a third `lang_matrix.rs` COBOL row (`ADD`/`MULTIPLY`/`SUBTRACT` → `20`).

### Added — v0.1.0: the `DISPLAY` / `MOVE` / `STOP RUN` slice (PL09 step 4)

- **New crate.** Lowers a parsed COBOL-60 program (the `cobol-parser` CST) into
  an `interpreter_ir::IIRModule` with a single `main` returning an i64 exit code,
  so COBOL runs on every LANG VM AOT backend. The COBOL sibling of
  `flow-matic-iir-compiler`.
- **PICTURE-typed data model (elementary items).** Each WORKING-STORAGE item with
  a `PICTURE` becomes one `str` register holding its stored picture image;
  `VALUE` initialises it. Group items and signed numerics (`PIC S9…`) are deferred
  with a clean error.
- **`MOVE <literal> TO item…`.** The literal is formatted into each receiver's
  picture — reusing `cobol-runtime`'s own `move_into_char` / `move_into_numeric`
  at compile time (this rung has no arithmetic, so every stored value is known
  statically) — and emitted as a fresh `str_const`. Byte-identical to the oracle.
- **`DISPLAY op…`.** Operand images `print_str`'d with no separator, then a
  `putchar('\n')` terminator. A literal prints its source text; a data-name prints
  its item register's stored image (so `DISPLAY 42` → `42` but `DISPLAY N` for
  `N PIC 9(5)=42` → `00042`).
- **`STOP RUN` → `ret 0`.**
- **Honest failure.** Arithmetic, `IF`, `PERFORM`, `GO TO`, `COMPUTE`,
  item-to-item `MOVE`, group items, and signed numerics each return a descriptive
  `CompileError::Unsupported` rather than wrong output — each lands on its own
  later PR.
- **Tests.** Unit tests for compile shape and every error path; `backend_compat.rs`
  proving the emitted IIR is accepted by the wasm / jvm / clr / beam validators;
  and `jit_e2e.rs` running each program on the generic JIT and asserting the
  DISPLAYed bytes equal the `cobol-runtime` oracle's.
- **`lang-aot` integration.** `Language::Cobol60` (aliases `cobol` / `cobol-60` /
  `cob`; extensions `.cob` / `.cbl`) dispatches to this frontend, with two proven
  rows added to `lang_matrix.rs`.
