# Changelog

All notable changes to `cobol-iir-compiler` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/); this
crate predates any release, so everything lives under Unreleased until the first
tag.

## [Unreleased]

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
