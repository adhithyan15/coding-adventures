# BEAM03 — f64 lowering for the BEAM backend

**Status:** In progress — first bounded slice (this spec).
**Depends on:** `BEAM01-twig-on-real-erl.md`, `BEAM02-closure-lowering.md` (the
existing compact-term encoder and `iir-to-beam` lowering architecture).
**Selected by:** `code/specs/LANG-VM-NON-ALGOL-BACKLOG.md`'s "VM-040 Dartmouth
BASIC BEAM pure-string family" slice, which found `iir-to-beam` has **zero**
`f64` lowering support at all and recommended scoping it as its own
multi-slice track (like COBOL BEAM was).

## 1. Problem

`iir-to-beam/src/lower.rs` has no `f64`/`Float` handling anywhere:
`validate.rs` unconditionally rejects any `const` instruction whose operand is
`Operand::Float(_)`. Dartmouth BASIC routes **every** scalar numeric value —
even an integer-spelled literal like `PRINT 42` — through the shared `f64`
value track (BA7-1b), so this single gap blocks the numeric two-thirds of
BASIC's corpus (~28 of 51 rows) on BEAM, plus any other frontend that ever
emits a float IIR op on this backend.

## 2. Why this needed real research, not just a promotion

Every prior VM-040 BEAM slice (COBOL, Oct, Nib, FLOW-MATIC) only needed to
*declare* an op whose lowering already existed — reusing string/bitwise/
comparison infrastructure that was already correct. Float support needed
actual new backend knowledge:

- BEAM has **no immediate encoding for floats.** Unlike a small integer
  (which the compact-term `I`/`U` tags encode directly as a tagged term),
  every float value — even a compile-time constant — must be a **boxed
  term**, which on-disk means a reference into the module's **literal table**
  (the `LitT` chunk).
- The existing compact-term encoder (`ir-to-beam/src/encoder.rs`) only
  implements `BEAMTag::{U,I,A,X,Y,F}`. The extended tag (`Z = 7`, used for
  "literal", "list", "fr", "alloc list" sub-operands) was scaffolded
  ("not used in v1") but never implemented.
- The `LitT` chunk itself needed reverse-engineering: OTP 28+ stores it
  **uncompressed** (a `0:32` marker word), but this repo's CI pins **OTP
  27.3.4.11** (`.github/workflows/ci.yml`), which only accepts the **older,
  zlib-compressed** form (`beam_asm:build_literal_chunk/2`'s `compressed_literals`
  branch). Getting this wrong produces a module that loads locally (OTP 29)
  but fails in CI (OTP 27) — exactly the kind of gap this backlog's
  "verify on the actual pinned runtime" discipline exists to catch.
- Erlang's float semantics are **not** IEEE-754-complete: `X / 0.0` raises
  `badarith` (confirmed empirically below) and the ETF decoder **refuses**
  to construct a non-finite float term at all (`binary_to_term` on a
  `NEW_FLOAT_EXT` payload with an Inf/NaN bit pattern raises `badarg`).  This
  is a genuine, permanent platform gap versus the cross-backend oracle
  (`vm-core::dispatch`'s documented IEEE-754 contract, matched by LLVM/WASM/
  JVM/CLR `fdiv`) — not something a smarter lowering can paper over. See §6.

## 3. Research method: "probe before declaring", applied to the platform itself

Per this backlog's established discipline, every claim below was verified
against a real, locally-installed Erlang/OTP 29 runtime (`erl`/`erlc` on
PATH) before being relied on in the design — not assumed from memory of the
BEAM format:

1. **Literal-table binary layout.** Compiled a two-line Erlang module with a
   float, then read its `LitT`/`Code` chunks with `beam_lib:chunks/2` and
   `beam_disasm:file/1`. Confirmed byte-for-byte: `<<0:32, NumLiterals:32,
   (Size:32, ExternalTermFormat)...>>`, where each literal's ETF blob starts
   with the version byte `131` and a standalone float is `NEW_FLOAT_EXT`
   (tag `70`) + 8 big-endian IEEE-754 bytes. Verified against the known bit
   pattern of `3.14` (`0x40091EB851EB851F`) appearing exactly at the expected
   offset.
2. **Compact-term operand for a literal reference.** Read `beam_asm.erl`
   (shipped OTP source, `lib/compiler-10.0.3/src/beam_asm.erl`) directly:
   `encode_arg({literal, Lit}, Dict)` and `encode_arg({float, Float}, Dict)`
   **both** go through `encode_literal/2`, which emits `encode(?tag_z, 4)`
   (the sub-tag-4 "literal" extended operand, `tag_z = 7` per
   `beam_opcodes.hrl`) followed by `encode(?tag_u, Index)` — i.e. exactly the
   existing `encode_compact_term` machinery, called twice, with no new
   encoding logic required beyond adding the `Z` tag.
3. **OTP-27-compatible (compressed) `LitT` format.** Read
   `beam_asm:build_literal_chunk/2` directly: the `compressed_literals`
   branch (OTP ≤27) wraps the plain `<<NumLiterals:32, ...>>` blob in
   `zlib:compress/1` and prefixes it with the **uncompressed** size. Rather
   than add an external zlib/DEFLATE dependency (this repo's Rust packages
   universally hand-roll their binary encoders — no existing package uses
   `flate2`/`miniz_oxide`), implemented a minimal, dependency-free RFC 1950/
   1951 encoder that emits **stored (uncompressed) DEFLATE blocks** — a
   trivial, unambiguous special case of the general format, chunked at the
   format's 65535-byte block limit, wrapped in the standard 2-byte zlib
   header and a Adler-32 trailer. This is a **valid** zlib stream (any
   compliant decoder must accept stored blocks; that's the whole point of
   the RFC), not an approximation.
4. **End-to-end round-trip proof.** Verified the Adler-32 implementation
   against `erlang:adler32/1` on a known input, then verified the full
   stored-block zlib stream by feeding it to real `zlib:uncompress/1`
   (the exact primitive `beam_load.c` uses to inflate `LitT`). Finally,
   **patched a real compiled `.beam` file's `LitT` chunk** — replacing the
   OTP-29-native uncompressed form with a freshly-built OTP-27-style
   compressed one, keeping every literal's ETF bytes and the surrounding
   `Code` chunk (which already referenced literals via `{literal, Idx}`
   operands) byte-for-byte identical — and **loaded and ran it with real
   `erl`**, observing correct output (`3.14` then `4.140000000000001`).
   This proves the exact `LitT` binary format this spec implements is
   accepted by a real BEAM loader end-to-end, independent of trusting the
   OTP source comments alone.
5. **Comparison and conversion opcodes on floats.** Forced the compiler to
   emit runtime (non-constant-folded) float code by reading operands from
   `init:get_plain_arguments/0`, then disassembled with `erlc -S`. Confirmed
   `X < Y` and `X == Y` on float registers compile to the **same** generic
   `is_lt`/`is_eq` term-comparison tests already used for the existing i64
   `cmp_lt`/`cmp_eq` lowering — no float-specific comparison opcode exists
   or is needed. Also confirmed `float/1` and `trunc/1` are ordinary
   (`gc_bif`-eligible) BIFs, matching the shape of the already-implemented
   `neg`/`not` single-argument `gc_bif1` lowering.
6. **Division-by-zero divergence (the one real semantic gap found).**
   `1.0/0.0`, `-1.0/0.0`, and `0.0/0.0` all raise `{badarith, ...}` on real
   `erl` — Erlang's `/` operator has no IEEE-754 Inf/NaN result, ever. This
   is documented as a known, currently-unaddressed limitation (§6), not
   silently papered over.

## 3a. Discovery: VM-D034 — `div` needs a different BIF for f64 than i64

Writing the real-`erl` integration test for float arithmetic (§7) hit a real
`badarith` trap immediately: `erlang:div(10.0, 4.0)` — because the existing
i64 `"div"` lowering already used `erlang:div/2` (Erlang's **integer-only**
division operator, which requires both operands to be integers and traps on
a float), not the general `erlang:'/'/2` operator. Unlike `add`/`sub`/`mul`
(whose `erlang:'+'`/`'-'`/`'*'` BIFs are already polymorphic over int and
float), Erlang's division has **two distinct operators** with no shared
polymorphic form. Fixed by dispatching `"div"` to a new `import_fdiv`
(`erlang:'/'/2`) specifically when `type_hint == "f64"`, leaving the
existing i64 `import_div` (`erlang:div/2`) path untouched. `"mod"` has no
f64 case in the current corpus and was deliberately left unfixed (documented
in the same match arm) — a future frontend emitting f64 `mod` would hit an
analogous trap and should get the same treatment then.

This is a different, narrower issue than the §6 div-by-zero platform gap:
VM-D034 is a straightforward "wrong BIF selected" bug with an unambiguous
fix (proven by the real-`erl` test now passing); §6 is a genuine platform
representability question with no clean fix, only tradeoffs.

## 4. Design: what ships in this slice

### 4.1 `ir-to-beam` (encoder) — new, general infrastructure

- `BEAMTag::Z = 7` (the previously-scaffolded "extended" tag).
- `BEAMModule.literals: Vec<Vec<u8>>` — each entry a complete ETF blob
  (version byte included), in module-literal-table order.
- `etf_new_float(f64) -> Vec<u8>` — `[131, 70] ++ value.to_be_bytes()`.
- `literal_operand(index: u32) -> [BEAMOperand; 2]` — the two-part
  `{tag_z,4}`+`{tag_u,index}` encoding derived in §3.2.
- `build_litt_chunk` — assembles `<<NumLiterals:32, (Size,ETF)...>>`,
  compresses it with the new `zlib_store_compress`, and prefixes the
  original size, matching `beam_asm`'s OTP-27 `compressed_literals` format
  exactly. Omits the chunk entirely when there are no literals (matching
  `beam_asm:build_literal_chunk/2`'s own omission rule).
- `zlib_store_compress`/`adler32` — the dependency-free RFC 1950/1951
  stored-block encoder from §3.3, with its own unit tests (including a
  known-vector Adler-32 check and >64KiB multi-block chunking).

This infrastructure is general (any future literal — not just floats — can
reuse `literal_operand`/`build_litt_chunk`), but this slice only produces
float literals.

### 4.2 `iir-to-beam` (lowering) — the actual f64 opcodes

- `validate.rs`: `const` with an `Operand::Float` source is now **accepted**
  when `type_hint == "f64"` (previously rejected unconditionally). Every
  other float-adjacent check is unchanged.
- `lower.rs`:
  - New `LiteralPool` (mirrors the existing `AtomTable`/`ImportTable`
    dedup-by-key pattern): interns each distinct `f64` bit pattern once,
    returning a stable 0-based literal-table index.
  - `const` with `Operand::Float(v)`: intern `v`, emit
    `move {literal_operand(idx)} {x,rd}`.
  - New `int_to_real` / `real_to_int_trunc` opcodes, lowered via `gc_bif1`
    to `erlang:float/1` / `erlang:trunc/1` respectively — the **same**
    single-argument `gc_bif1` shape already used for `neg`/`not`, just two
    new import-table entries.
  - **No changes** to `add`/`sub`/`mul`/`div`/`cmp_eq`/`cmp_ne`/`cmp_lt`/
    `cmp_le`/`cmp_gt`/`cmp_ge`: these already lower generically over
    `Operand::Var` registers regardless of the term type the register holds
    (confirmed by §3.5 and by the fact that the existing i64 lowering for
    these ops never inspects `type_hint` beyond the `u4`/`u8` masking
    branch, which floats never match). Once `const` can put a valid boxed
    float into a register, the existing arithmetic/comparison lowering
    handles it correctly with zero further changes.

### 4.3 Promoted corpus rows

The two "numeric baseline" `DartmouthBasic` rows in
`lang-aot/tests/lang_matrix.rs` (filtered indices 0–1, immediately before the
18-row pure-string family VM-040 already promoted):

1. `10 PRINT 42\n20 END\n` → `"42"` — BA7-1b's paradigm case: an
   integer-spelled literal riding the shared f64 track through
   `__basic_print_real`.
2. `10 PRINT 6 ^ 2 + 6\n20 END\n` → `"42"` — literal-integer-exponent `^`
   (repeated `mul`, no `f64_pow`) plus a top-level `add`, then the same
   print path.

Both exercise the full op set this slice adds: `const`(f64), `mul`, `add`,
`cmp_lt`/`cmp_eq`/`cmp_ge`(f64, inside `__basic_print_real`'s sign/zero/
magnitude-bucket dispatch), `sub`, `div` (by compile-time-constant, nonzero
divisors only — see §6), `int_to_real`, `real_to_int_trunc`. This is
substantially more than a synthetic "does a float const load" proof: it is
two complete, real BASIC programs, chosen because they are the actual next
items in the corpus, not a bespoke test fixture.

## 5. Explicitly out of scope for this slice

- **General float division by zero** (§6) — not reachable by the two
  promoted rows (every divisor in `__basic_print_real`'s helper chain is a
  compile-time nonzero constant), but a real gap for arbitrary future BASIC
  programs (`PRINT 1/0`). Needs a design decision, not a bigger lowering —
  see §6.
- The remaining ~26 numeric/`INPUT` BASIC rows (general `LET`/`FOR`/user
  arithmetic/`RND`, which additionally need `neg` on f64, `f64_pow`, and the
  `INPUT` host-read design already tracked as VM-060b).
- `f32` — this backend only ever sees `f64` from any current frontend;
  `f32` is accepted nowhere else in the codebase's IIR either.
- Non-finite float literals — physically inexpressible on BEAM (§3.6);
  no frontend currently emits one.

## 6. Discovered platform limitation: BEAM floats cannot represent Inf/NaN

Real Erlang floats are **not** a full IEEE-754 binary64 implementation: the
language deliberately treats floats as a "real number" abstraction and
refuses to construct a non-finite value through arithmetic (`X/0.0` →
`badarith`) or even through raw term construction (`binary_to_term` on an
Inf/NaN-bit-patterned `NEW_FLOAT_EXT` → `badarg`). Every other LANG VM
backend (LLVM/WASM/JVM/CLR `fdiv`, and the shared `vm-core` oracle) commits
to full IEEE-754 semantics, so this is a **permanent, structural** backend
divergence, not a bug fixable by more lowering code.

Two directions exist for a future slice, and this is the kind of call that
needs a real product decision rather than a guess:

- **(a) Accept the divergence** and make BEAM `div`(f64) trap
  (`badarith`-equivalent) on a zero divisor — document it as a known,
  intentional BEAM-only difference from the cross-backend oracle, the same
  way this backend already fully lacks Inf/NaN representability elsewhere.
- **(b) Emulate IEEE-754** by reserving a sentinel encoding (e.g. a tagged
  2-tuple `{'$inf', Sign}` / `{'$nan'}`) that every f64 lowering path
  (`add`/`sub`/`mul`/`div`/comparisons/printing) would need to check for —
  a much larger, cross-cutting change touching every op this slice added
  and likely several more, and one that would make BEAM's float
  representation diverge structurally from a plain Erlang float (breaking
  interop with anything that inspects the term from outside this backend).

This slice does not pick between them because no promoted row exercises the
question yet; the next f64-lowering slice that adds general (non-constant-
divisor) division should raise this explicitly.

## 7. Validation

- `ir-to-beam` unit tests: Adler-32 known vector, stored-block chunking
  (including a >64KiB multi-block case), `literal_operand` byte-level
  encoding (`(4<<4)|7 = 0x47` header byte), `etf_new_float` byte layout,
  and an `encode_beam` round-trip test that a module with literals contains
  a `LitT` chunk in the documented compressed format.
- `iir-to-beam` unit tests: float `const` accepted by `validate_for_beam`;
  `int_to_real`/`real_to_int_trunc` lowering shape (import table entries,
  `gc_bif1` operand pattern); literal-pool dedup (two identical float
  constants intern to the same index).
- Real-`erl` integration tests in `iir-to-beam` (new, mirroring the
  existing `str_slice`/large-const real-`erl` tests added by prior VM-040
  slices): a standalone module exercising `const`/`add`/`sub`/`mul`/`div`/
  `cmp_*`/`int_to_real`/`real_to_int_trunc` runs and prints the expected
  values on real Erlang.
- The two promoted `lang_matrix` rows execute on real Erlang via
  `portable_text_stdout_dartmouth_basic_beam_numeric_baseline` before
  promotion (this backlog's "probe before declaring" gate), plus fresh-
  process single-cell reruns for both.
- Full `iir-to-beam`/`ir-to-beam`/`lang-aot` test suites and all-target
  Clippy (warnings denied) after the change.

## 8. Continuation slice: `neg`(f64) and `f64_pow`

**Status:** In progress — second bounded slice (this section), selected by
this spec's own §4.3 trailing note and `LANG-VM-NON-ALGOL-BACKLOG.md`'s
BEAM03 top-section reprioritization, which named these as candidates (a) and
(b) respectively.

### 8.1 `neg`(f64) — no lowering change needed

Re-reading `iir-to-beam/src/lower.rs`'s existing `"neg" | "not"` arm (§4.2's
prediction, now verified) confirms it is **already correct for f64** with
zero code changes:

- It dispatches unconditionally to `import_neg` (`erlang:-/1`), a `gc_bif1`
  call. Unlike `div` (VM-D034: two *different* Erlang operators for integer
  vs. float division), Erlang's unary `-` has exactly **one** operator that
  is already polymorphic over integer and float operands — confirmed on
  real `erl`: `erlang:'-'(3.14)` returns `-3.14` with no `badarith`.
- The only per-`type_hint` branch in that arm is the `u4`/`u8` narrowing
  mask (`band 15`/`band 255`), gated on `matches!(instr.type_hint.as_str(),
  "u4" | "u8")`. `"f64"` never matches that guard, so a float `neg` already
  skips masking and falls straight through — exactly the desired behavior
  (floats are never narrowed).
- `validate.rs` does not special-case `neg` at all: its only float-specific
  check (§ Checks table, `UnsupportedType` (float const)) applies solely to
  `op == "const"`. A `neg` instruction with `type_hint == "f64"` was never
  rejected.

This makes `neg`(f64) the "structurally trivial" case the backlog predicted
— but "trivial lowering" still needed **proof**, not just code reading: see
§8.3's real-`erl` test and §8.4's promoted corpus row, which is a completely
independent verification of the same claim.

### 8.2 `f64_pow` — a new `call_ext`, not `gc_bif2`

Unlike `neg`, `f64_pow` needed real design work, because the backlog's own
"unverified" flag on this candidate was warranted:

- `math:pow/2` is an ordinary Erlang function, **not** a loader-recognized
  guard BIF. The existing `gc_bif1`/`gc_bif2` opcodes only work for the
  fixed allowlist of guard BIFs the BEAM loader itself recognizes (the same
  set usable inside a guard expression) — confirmed by reading
  `beam_asm.erl`'s guard-BIF handling table (only names like
  `erlang:'+'/2`, `erlang:length/1`, etc. appear; `math:pow/2` is absent)
  and cross-checked empirically: compiling a call to `math:pow(2.0, 3.0)`
  and disassembling with `erlc -S` shows an ordinary `call_ext_only`/
  `call_ext` to the `math:pow/2` import, never a `gc_bif2` instruction.
- So `f64_pow` lowers the same way `iir-to-beam` already lowers other
  non-guard-BIF two-argument calls (`str_concat`'s `erlang:'++'/2`,
  `str_index`'s `lists:nth/2`): stage both source registers into scratch
  registers above `meta.next_reg` (avoiding the parallel-move hazard where
  writing `x1` could clobber a source still needed for `x0`, the same
  reasoning documented at `str_index`'s call site), `save_live_across_imported_call!`,
  move staged values into `x0`/`x1`, emit `call_ext 2 {u, import_pow}`, move
  the result (`x0`) to the destination register if it differs, then
  `restore_live_across_imported_call!`.
- New import: `math:pow/2` (module atom `"math"`, function atom `"pow"`,
  arity 2). `f64_pow` is added to the `live_across` call-list match (the
  "EVERY op that emits a `call_ext` must be listed here" invariant already
  documented at that match) — an omission there is the exact VM-D0xx-class
  bug class the comment warns about (values silently destroyed, not a
  crash).
- Semantics match the `vm-core` oracle exactly: `handle_f64_pow` documents
  `base.powf(exp)` (Rust `f64::powf`, "NaN / ±inf propagate per IEEE-754,
  same as libm `pow`"). Erlang's `math:pow/2` is also a direct libm `pow`
  binding, so the two agree on every finite input; the same §6 Inf/NaN
  representability gap that already applies to `div` applies identically
  here (unreachable by the promoted corpus row, whose base/exponent are
  both finite non-degenerate values) — no new platform-limitation surface,
  just the existing one.

### 8.3 Validation (continuation)

- `iir-to-beam` unit tests: `f64_pow` emits exactly one `call_ext` (not
  `gc_bif2`) and a new `math:pow/2` import-table entry; `neg` on `type_hint
  == "f64"` emits `gc_bif1` with **no** subsequent masking `gc_bif2` (the
  existing `narrow_operations_mask_but_i64_remains_unbounded`-style
  assertion, extended to `"f64"`).
- Real-`erl` integration tests (mirroring §7's existing `test_82`–`test_84`
  shape): `neg(3.5)` then `neg` of that result round-trips to `3.5`
  (proving polymorphic unary minus survives two applications, not just
  sign-flips into a coincidentally-still-valid case); `f64_pow(2.0, 10.0)`
  truncates to `1024`; a combined case chains `neg` and `f64_pow` in one
  module the way BASIC's `ABS`/general-`^` lowering actually does.
- Full `iir-to-beam` test suite and all-target Clippy (warnings denied)
  after the change.

### 8.4 Promoted corpus rows

Two more Dartmouth BASIC `lang_matrix.rs` rows, chosen because each is
already the minimal, single-purpose existing proof for exactly one of these
two ops (not a bespoke fixture written for this slice):

1. `10 PRINT ABS(-42)\n20 END\n` → `"42"`. `dartmouth-basic-iir-compiler`'s
   unary-minus lowering (`emit_unary`) emits `neg` for the literal `-42`
   itself, and `ABS`'s own inline `if X < 0 then -X else X` lowering
   (`"abs"` arm) emits a **second**, independent `neg` inside the taken
   branch — so this one row exercises `neg`(f64) twice, plus the
   `cmp_lt`/`jmp_if_false`/`mov`/`label` control-flow ops already proven
   working by earlier BEAM slices, plus the same `__basic_print_real` path
   BEAM03's first slice already validated.
2. `10 PRINT 4 ^ 0.5\n20 END\n` → `"2"`. The literal-integer-exponent fast
   path (repeated `mul`, already supported) only fires for exponents
   `literal_integer_exponent` recognizes as a nonnegative integer; `0.5`
   falls through to the general `f64_pow` runtime-call path — the smallest
   possible proof that isn't reachable by the fast path.

Both were executed on real Erlang via a dedicated `lang_matrix` test before
promotion, per this backlog's "probe before declaring" discipline — see the
`lang-aot` CHANGELOG entry for the exact test name and result.

### 8.5 Explicitly out of scope (continuation)

- The remaining BASIC numeric rows still needing `INPUT` (VM-060b, BEAM host
  input) — untouched by this slice.
- The general `mod`(f64) gap flagged (but deliberately left unfixed) by
  VM-D034 — no current frontend emits it; still just documented, not fixed.
- The §6 Inf/NaN representability design question — still open, still not
  reachable by either promoted row.
- `f64_pow` with a non-finite result (e.g. a negative base with a
  fractional exponent, which is `NaN` in IEEE-754) — same platform gap as
  §6, not exercised by the promoted row (base `4`, exponent `0.5`, exact
  finite result `2.0`).

## 9. VM-LOOP-24: probe-first sweep of the remaining non-`INPUT` BASIC rows

The BEAM03-continuation slice's own trailing note (§8 above, and the
backlog's own record) named five candidate groups still blocking BASIC's
remaining ~29 rows once the 5 `INPUT`-driven rows are set aside as
out-of-scope pending VM-060b: general `FOR`/`LET` arithmetic, one- and
two-dimensional numeric arrays, `DATA`/`READ`/`RESTORE`, `RND`'s full
DEF-FN-and-module-global chain, and `GOSUB`/`RETURN` — and explicitly
flagged that "nothing in this slice or its predecessor established that
`neg`/`f64_pow` alone unblocks them", so each needed its own real-`erl`
probe rather than a guessed promotion.

### 9.1 Method

A scratch probe (`lang_aot::compile_source_to_beam` + a real `erl` run,
discarded before this PR — not part of the permanent suite) ran EVERY
not-yet-`Beam` Dartmouth BASIC corpus row individually, unmodified from the
existing `lang_matrix.rs` source text, using only the `iir-to-beam`
capabilities that existed before this slice. This produced three groups:

### 9.2 Group A — zero new lowering (12 rows)

These ran correctly on real `erl` completely unchanged:

- General `FOR`/`NEXT` and `FOR … STEP` loops — `cmp_le`/`add` on the shared
  `f64` value track, the same ops the BEAM03-continuation slice's numeric
  baseline already proved; the loop-carried backward branch itself was
  already proven by other frontends (COBOL, Twig) on `Beam`.
- `IF … THEN <line>` + `GOTO`-style jumps — `cmp_gt` on `f64`.
- `DEF FN` — a same-module `call`, the exact calling convention ALGOL's
  value procedures already run on every backend including `Beam`.
- Multi-item `PRINT` with `;` (including a literal negative operand, i.e.
  `sub`) and `,` — pure `__basic_print_int`/separator control flow, no
  numeric op beyond what earlier BEAM slices proved.
- Ordinary scalar real arithmetic (`6.0 * 7.0`), BA7 fixed-decimal
  formatting, and six-significant-digit/`E`-notation formatting — all
  `__basic_print_real`/`__basic_print_fixed_mag`/`__basic_print_real_e`
  paths already exercised by the BEAM03 numeric-baseline row; these
  specific corpus rows simply hadn't been run individually before.
- Flat and nested `GOSUB`/`RETURN` — the E5 `array<i64>` return-address
  stack. This is an **integer** array (`alloc_array`/`array_set`/
  `array_get` at `type_hint == "i64"`), so it never touches the `atomics`
  float-storage gap in §9.3 below — `atomics:put/3` requires an integer
  value, and an `i64` return address already is one.
- `SGN` — an inline three-way `cmp`/`const` conditional, no new op (the
  same shape `ABS` and the `RND(X)` sign-dispatch already use).

Promoted straight to `Beam`, each executed on real `erl` via the new
`portable_text_stdout_dartmouth_basic_beam_general_arithmetic_and_control_flow`
`lang_matrix` test (12 programs).

### 9.3 Group B — bounded/mechanical: single-argument `math:*` + `erlang:floor/1` (5 rows)

`SQR`, `SIN`/`COS`/`LOG`/`EXP`, `ATN`, `TAN`, and `INT` all failed
`iir-to-beam` validation with `UnsupportedOp` for their respective IIR ops
(`f64_sqrt`/`f64_sin`/`f64_cos`/`f64_ln`/`f64_exp`/`f64_atan`/`f64_tan`/
`real_to_int_floor`) — no lowering arm existed at all, unlike the array
rows in §9.4 (which compiled and only failed at runtime).

Each was confirmed mechanical, the same "read the actual contract" way
`f64_pow` was in §8.2, by disassembling a real compiled call to each with
`erlc -S`:

- `math:sqrt/1`, `math:sin/1`, `math:cos/1`, `math:log/1` (natural log —
  BASIC's `LOG`/IIR `f64_ln`), `math:exp/1`, `math:atan/1`, `math:tan/1` all
  emit `call_ext`/`call_ext_only`, never `gc_bif1` — they are ordinary
  `math` module functions, not loader-recognized guard BIFs (the same
  reasoning as `math:pow/2` in §8.2). Each lowers with the exact `f64_pow`
  staging pattern, just with **one** operand staged instead of two: move
  the source into a scratch register above `meta.next_reg`,
  `save_live_across_imported_call!`, move into `x0`, `call_ext 1
  {u,import_idx}`, move the result to the destination,
  `restore_live_across_imported_call!`. All seven joined the `live_across`
  call-list match (the same invariant `f64_pow` joined in §8.2).
- `erlang:floor/1`, by contrast, disassembles to `{gc_bif,floor,{f,0},1,
  [{x,0}],{x,0}}` — the SAME generic `gc_bif1` shape `int_to_real`
  (`erlang:float/1`) and `real_to_int_trunc` (`erlang:trunc/1`) already use.
  So `real_to_int_floor` joins THAT match arm instead, not the `math:*`
  `call_ext` family — despite being adjacent in the frontend's own `INT(X)
  = real_to_int_floor + int_to_real` lowering, the two ops need different
  BEAM opcodes for the same reason `f64_pow` and `int_to_real` did in §8.

New imports: `math:sqrt/1`, `math:sin/1`, `math:cos/1`, `math:log/1`,
`math:exp/1`, `math:atan/1`, `math:tan/1`, `erlang:floor/1`.

Promoted to `Beam`, each executed on real `erl` via the new
`portable_text_stdout_dartmouth_basic_beam_math_builtins` `lang_matrix` test
(5 programs), plus dedicated `iir-to-beam` unit tests proving instruction
shape (`test_90`/`test_91`, mirroring `test_86`) and real-`erl` execution
(`test_92`/`test_93`, mirroring `test_88`).

### 9.4 Group C — deferred: genuine design questions, not guessed at

Three sub-groups compiled (or partly compiled) but are explicitly NOT
promoted in this slice, because each is the kind of "genuinely open design
question" this backlog's own instructions say to document rather than force:

1. **1-D/2-D numeric arrays, `DATA`/`READ`/`RESTORE`, and real aggregate
   `DATA`** (`10 DIM A(3)…`, `10 DIM A(1,2)…`, `10 DATA 21\n20 READ A…`,
   `10 DIM A(1)\n20 DATA 3.14, 0.25…` — 4 rows). Every one compiles cleanly
   through `iir-to-beam` validation, then **traps at runtime** on real `erl`
   with `{badarg,[{atomics,put,[Ref,Index,FloatValue],...}]}`. This is a
   genuine BEAM data-representation gap, not a missing opcode: `iir-to-beam`
   represents every `alloc_array`/`array_set`/`array_get` with Erlang's
   `atomics` module (see `lower.rs`'s own "Mutable memory: the `:atomics`
   module" comment), and **`atomics` is a fixed-size array of 64-bit
   INTEGERS only** — `atomics:put/3` raises `badarg` the instant it's given
   a float value (confirmed on real `erl`: `atomics:put(Ref, 1, 40.0)`
   traps; `atomics:put(Ref, 1, 40)` does not). BASIC's arrays are
   `array<f64>` (even `DIM A(3)` — BA7-1b routes every scalar numeric value
   through the shared `f64` track), so every BASIC array write traps. Fixing
   this needs a real architecture decision — e.g. storing the IEEE-754 bit
   pattern as an integer via BEAM's bit-syntax (`<<F:64/float>> =
   <<Bits:64>>`, itself new lowering `iir-to-beam` doesn't have for ANY op
   today) versus switching float-element arrays to a different BEAM
   substrate entirely (e.g. `ets`, which stores arbitrary terms but has
   different mutation/cost characteristics than `atomics`) — not a
   mechanical opcode addition. Left as an open follow-up; see the backlog's
   VM-LOOP-24 section for the explicit next-item framing.
2. **String arrays and mixed numeric/string `DATA`** (`10 DIM A$(2)…`,
   `10 DIM S$(1)\n20 DATA 20, "O", 22, "K"…` — 2 rows). These fail
   `iir-to-beam` VALIDATION outright (`UnsupportedType`: `array_set`/
   `array_get` with `type_hint == "str"` — "only the ASCII string subset is
   supported in this BEAM backend"), independent of the float-array gap
   above. `iir-to-beam` has no `str`-typed array element representation at
   all yet. A separate, likely larger design item than the float-array gap
   (it needs a BEAM representation for a *heterogeneous* element type, not
   just a wider scalar).
3. **`RND`**. Its own corpus row does not just need `real_to_int_floor` (the
   probe confirmed adding that lowering does NOT unblock it): it traps with
   `{badarith,[{erlang,'*',[undefined,48271],...}]}` inside the compiled
   `__basic_rnd` helper — a module-global `erlang:get/1` read returned the
   atom `undefined` instead of the expected seed integer. This is exactly
   the "RND's full DEF-FN-and-module-global chain" open design question the
   backlog flagged all the way back at VM-018, not a new discovery of this
   slice — this probe just confirms it is still unresolved and that nothing
   in this slice's scope (math builtins, `real_to_int_floor`) happens to fix
   it as a side effect. Left open.
