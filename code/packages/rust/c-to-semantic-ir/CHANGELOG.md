# Changelog

## [0.3.0] - 2026-08-17

### Changed — SIR21 T3b-2 Slice 6: `combine()`'s division reconciliation

Part of the SIR21 T3b-2 arc (splitting the overloaded `/` builtin into
`div_floor`/`div_trunc`/`udiv_trunc`/`div_true` — see
`code/specs/SIR21-type-system-and-integer-semantics.md` §E3). All 7
backends implement all four ops (Slice 2, merged). This crate is its own
slice because `combine()` needed two *different* renames in the same
function — more mistake surface than the mechanical single-op-name swaps
every other frontend slice did.

**Behavior change 1 — the float (`double`) arm**: `/` no longer lowers to
bare `BuiltinCall("/", args)`. It now lowers to `BuiltinCall("div_true",
args)`. C's usual arithmetic conversions always promote both operands to
`double` before a floating division runs, so there is no int/float
distinction left to conflate `/` with once this arm is reached — it always
true-divides, the exact shape `div_true` exists for.

**Behavior change 2 — the integer arm**: `/` no longer lowers to the
`tdiv`/`utdiv` builtin names. It now lowers to `div_trunc` (signed) /
`udiv_trunc` (unsigned) — the exact same underlying semantics (truncate
toward zero, matching C's own `/` on integers), just under T3b-2's
canonical name family instead of this crate's own pre-existing ad hoc
names. This is a pure rename, not new logic: `semantic-ir-to-c`'s
`div_trunc`/`udiv_trunc` dispatch entries point at the *same*
`_sir_itdiv`/`_sir_utdiv` runtime functions the old `tdiv`/`utdiv` entries
already called (unchanged by this PR). `tdiv`/`utdiv` themselves are now
dead dispatch entries — no frontend emits them anymore — left in place for
Slice 7 (task #29) to delete, per this arc's established
additive-then-cleanup discipline. `%`/`tmod` are untouched; this arc
explicitly does not touch modulo (see the spec's own forward pointer to a
future, unnumbered milestone).

Verified via this crate's own `tests::double_division_is_true_division_
not_truncating` and `tests::division_and_modulo_lower_to_truncating_
builtins` (both updated to assert the new names), `tests::int_min_div_is_
guarded` (unchanged — runtime-behavior assertion, not name-based, still
passes: the `INT64_MIN/-1` two's-complement-wrap guard lives in
`_sir_itdiv` itself and this rename doesn't touch that function), and the
full `tests/three_way_conformance.rs` corpus (byte-identical against real
`cc`-compiled output, unchanged and still green).

`c-to-semantic-ir` 0.2.0 -> 0.3.0.

## [0.2.0] - 2026-08-11

### Changed — SIR28 Slice 6 (batch 3): `printf` lowers to `__sys_write__`

Part of the SIR28 arc (`__sys_write__`, the general syscall-primitive
family — see `code/specs/SIR28-syscall-primitives.md`). All 7 backends
already implement it (Slice 3); this crate is the last frontend in
Slice 6.

**Behavior change**: `printf("<fmt>", args…)` no longer lowers to
`BuiltinCall("print", parts)`. It now lowers to
`BuiltinCall("__sys_write__", [StrLit("stdout"), StrLit("none"),
BoolLit(false), ...parts])` — `terminator: "none"` is the exact fit for
`printf`'s own semantics: write every value back-to-back with no added
separator and no added newline (a `\n` only appears when the format
string itself contains one, already baked into `parts` as a literal
`StrLit` chunk by the existing format-string parser). `Feature::ConsoleIO`
is declared whenever `printf` is used (new `uses_console_io` flag,
mirroring the existing conditional `uses_strings`/`uses_floats`/
`uses_arrays` flags). Also now sets `Effect::MayPrint` (previously
unset, the same gap fixed in every other frontend this arc has touched).

Verified byte-identical against real `cc`-compiled output across the
full `tests/three_way_conformance.rs` corpus (~101 `printf` call sites)
— this frontend's own oracle test, unchanged and still green.

`c-to-semantic-ir` 0.1.0 -> 0.2.0.

### Fixed — `<<` lowered to a SIR builtin name that collided with Ruby's `<<`

`ruby-to-semantic-ir` gained a polymorphic `<<` operator (Array push /
String concat / saturating-Integer-shift), lowered to the SAME bare
`BuiltinCall("<<", ...)` name this crate already used for C's raw bitwise
left shift. `semantic-ir-to-c`'s emit-time dispatch had to pick ONE
meaning for that shared name and picked Ruby's — so a C program's `<<`
silently got Ruby's saturating semantics instead of a raw bit-shift.
Caught by `three_way_conformance`'s "uint64 logical right shift of a
high-bit value" case: `uint64_t x = 1; uint64_t y = x << 63;` needs the
true bit pattern `0x8000000000000000`, which isn't representable without
saturating, so it silently clamped to `INT64_MAX` and the subsequent
`y >> 32` read back the wrong high word (`2147483647` instead of the
correct `2147483648`).

Fixed by lowering C's `<<` to a distinct `c<<` builtin name instead
(mirroring the existing `>>`/`u>>` signed/unsigned split, which never had
this collision — Ruby doesn't implement `>>` at all). `semantic-ir-to-c`
and `semantic-ir-to-ruby` both updated to recognize `c<<` alongside `<<`;
see their own CHANGELOGs.

`>>`/`u>>` were never affected (Ruby has no `>>` operator), so this only
touches `<<`.

### Fixed (CI — Linux link failure)

- Three C-compiling test helpers (`tests/three_way_conformance.rs`'s two,
  plus `src/lib.rs`'s inline unit-test `run_c`) didn't link `-lm`. Harmless
  until `semantic-ir-to-c`'s Numeric Collections slice
  (0.31.0) made `floor`/`ceil`/`round`/`abs` the first `<math.h>` calls the
  embedded runtime template makes — pasted into every generated `.c` file
  regardless of source content, so any program compiled through this
  crate's C round-trip now fails to link on Linux (`undefined reference to
  'floor'`) without it. Fixed alongside the same gap in `sir-conformance`
  and `sir-bench`.

### Milestone 11 — fixed-size arrays

The first of the four aggregate axes (arrays → structs → pointers → strings).
A C array maps to a SIR **sequence**, which the core and both backends already
render (`SeqLit`/`SeqIndex`/`SeqSet`, `Feature::Sequences`), so this is a
**frontend-only** slice — no backend changes.

- **Grammar:** `init_declarator` gains an array dimension and a brace
  initializer (`int a[3]`, `int a[3] = {1,2,3}`, `int a[] = {…}`); `postfix`
  gains `index_suffix` so `a[i]` parses as an rvalue and an assignment target.
- **Lowering:** array-ness lives only in the symbol table (`Binding` gains
  `array_len: Option<usize>`), so every expression still has a scalar `CType`
  and the expression machinery is unchanged.
  - `int a[N] = {…}` → a `SeqLit` of the initializers (each converted to the
    element type), **zero-filled** to `N` (C's aggregate rule); an uninitialised
    `int a[N];` is all-zero.  Size is `N`, or the initializer count for
    `int a[] = {…}`.
  - `a[i]` → `SeqIndex` (result = element type); `a[i] = e` → `Stmt::SeqSet`
    with `e` converted to the element type, so a narrowing store still wraps
    (`uint8_t a[…]; a[0] = 200 + 100;` stores 44).
  - **`Feature::Sequences` is declared only when an array is used**, so a
    scalar-only program is unchanged.
  - Refused (positioned error): whole-array assignment, a bare array name used
    as a value (no pointer decay yet), indexing a non-array, a non-integer
    index, chained/mixed postfix (`a[i][j]`, `f(x)[i]`), too many initializers,
    and a length past `MAX_ARRAY_LEN` (a DoS guard).
- Verified: 51 frontend unit tests (8 new arrays) + three-way conformance
  (8 new array cases: literal/loop reads, indexed write, loop fill, computed
  index, partial-init zero-fill, per-element `uint8` wraparound) byte-identical
  across reference C (`clang -fwrapv`), emitted Ruby, and emitted C; emitted C
  compiles clean on clang + gcc + MSVC.

### Milestone 10 — faithful `printf`

`printf` now reproduces C's formatting exactly, so a program can print a
`double` with `%f` and get byte-identical output across reference C, emitted
Ruby, and emitted C (milestone 9's corpus had to cast to `(int)` to dodge float
display — that dodge is gone).

- The frontend parses the format string (`parse_printf_format`) into literal
  chunks and typed conversions; a backend never receives a source-derived format
  string (no format-string vulnerability). `printf("<fmt>", args…)` lowers to a
  single `print(seg₀, seg₁, …)`.
- **Literal chunks** → `StrLit` (C escapes decoded; `%%` → `%`), re-escaped by
  each backend's string emitter. **`%d`/`%i`/`%u`** → the integer value passed
  straight through (`print` renders it as decimal). **`%f`/`%F`/`%e`/`%E`/`%g`/
  `%G`** (optional `.precision`, default 6) → a new `fmt_float(value, precision,
  kind)` builtin both backends implement with C-compatible `sprintf`/`snprintf`.
- `print` concatenates with no separator and no auto-newline, so a `\n` in the
  format is emitted as literal text (the old lowering auto-added a newline and
  discarded the format's literal content). A literal-only `printf("hi\n")` now
  works (was an error).
- **Refused** (positioned error, not mis-formatted): field width (`%5d`), flags
  (`%-`/`%+`/`%0`/`%#`), and `%c`/`%s`/`%x`/`%p`/`%n`. Precision is capped
  (`MAX_PRINTF_PRECISION = 512`) as a DoS guard.
- `Feature::Strings` is declared only when a `printf` actually emits literal
  text / `fmt_float`, so a program that never prints stays string-free.
- Verified: 49 frontend unit tests (6 new printf) + three-way conformance (8 new
  float-display cases, fixed `%f`/`%.Nf` notation) byte-identical across all
  three legs; emitted C compiles clean on clang + gcc + MSVC. Exponent forms
  `%e`/`%g` are supported but kept out of the byte-identical corpus (their
  exponent digit count varies by platform: Windows libc `e+004` vs Ruby `e+04`).

### Milestone 9b — floating-point value track

The lowering gains a **second value track** for floating point.  Each expression
now carries a `CType` (`Int(IntSpec)` or `Double`) instead of a bare `IntSpec`;
`float`/`double` map to `CType::Double` → `SirType::Float` (SIR's single 64-bit
IEEE float, so `float` is treated as `double`).

- **Float literals** (`3.14`, `.5`, `1e10`, `1.0f`) now lower to `Expr::FloatLit`
  (the `f`/`l` suffix is dropped) rather than being rejected.
- **Mixed int/double arithmetic** promotes the integer operand to `double` with a
  `to_f` builtin (the usual arithmetic conversions), and the float op emits the
  bare `+`/`-`/`*`/`/` builtin with **no** width `Convert` (a `double` has no
  width to wrap to).
- **`/` on `double` is true division** (the plain `/` builtin), not the integer
  truncating `tdiv`/`utdiv`.  `%`, `&`, `|`, `^`, `<<`, `>>`, `~` are **rejected**
  on `double` (undefined on floating point in C).
- **Conversions:** a float↔int cast flows through `to_f` (int→double) and `to_i`
  (double→int, truncating toward zero like C's `(int)double`); a double→int cast
  is `Convert(to_i(e), spec)` — truncate then narrow.
- **`Feature::Floats` is declared only when the program uses floating point**, so
  an integer-only program's SIR (and every backend's output) is unchanged.
- Verified by the three-way conformance corpus (8 new float cases: mixed
  promotion, true division, loop accumulation, float→int truncation, `double`
  parameters/returns) — byte-identical across reference C (`clang -fwrapv`),
  emitted Ruby, and emitted C, the last also compile-clean on clang + gcc + MSVC.
- Supersedes the milestone-9a "lowering rejects floats" boundary below.

### Milestone 9a — floating-point grammar (foundation)

- The C grammar now recognises **`float`/`double`** type keywords and
  floating-point **literals** (`3.14`, `.5`, `10.`, `1e10`, `2.5e-3`, `1.0f`).
  `FLOAT_LIT` precedes `INT_LIT` and requires a `.` or an exponent, so a plain
  `42` still lexes as an integer.
- The `c.tokens`/`c.grammar` sources were extended and the embedded
  `c-lexer`/`c-parser` `_grammar.rs` regenerated via `grammar-tools`.
- Lowering is still **integer-only**: `float`/`double` types and float literals
  are rejected with a clear "not yet supported" error rather than being
  mis-typed as `int` — the honest boundary until the floating-point value track
  (arithmetic, int↔float conversions) lands in a following slice.

### Milestone 7 — per-block scoping

- The flat symbol table is replaced with a **scope stack** (pushed at a `{ }`
  block, an `if`/`else`/loop body, and a `for`'s init+body region).  Shadowing
  and re-used names — including two sequential `for (int i = …)` loops — now
  compile instead of being rejected.
- Because SIR's namespace is flat, every declaration gets a **unique SIR name**
  (`name`, then `name__2`, …), so two distinct C variables sharing a spelling
  never collide.  This removes the milestone-3 shadowing miscompile hazard *by
  construction* — distinct variables have distinct names.
- The early-return lifting trampoline carries a **`PopScope` marker** on its work
  queue, so a spliced block's declarations go out of scope at its `}` and the
  continuation is lowered in the enclosing scope — correct lifetime even though
  the two are merged into one SIR block.
- Still enforced: re-declaring a name in the same block is a C error; a variable
  is undeclared once its block closes.  (A self-referential initializer `int v =
  v + 1;` in a shadowing block is UB in C — reads the uninitialized inner `v` —
  and is not conformed to.)
- Corpus grows with well-defined shadowing programs (nested-block shadow,
  shadow in a lifted `else`, two sequential `for` loops, shadowing a parameter) —
  byte-identical across reference `clang -fwrapv`, emitted Ruby, and emitted C
  (and clang+gcc+MSVC).

### Milestone 6 — division & modulo (`/ %`, truncating)

- C `/` **truncates toward zero** and `%` takes the dividend's sign (`-7/2 = -3`,
  `-7%2 = -1`), unlike SIR/Ruby `/`/`%` and the C backend's `_sir_ifloordiv`,
  which floor.  So they lower to dedicated `tdiv`/`tmod` builtins, never the
  flooring ones.
- **C backend** — `_sir_itdiv`/`_sir_itmod`: C's native `int64_t /`/`%` already
  truncate, so these are thin wrappers, guarded against division by zero and
  `INT64_MIN / -1` (returns the two's-complement wrap, which the width `Convert`
  narrows — x86 hardware traps on it otherwise).  As with `>>`, an unsigned
  common type routes to `utdiv`/`utmod` (`_sir_utdiv`/`_sir_utmod`, done over
  `uint64_t`), because a `uint64_t` ≥ 2^63 is a negative int64 on which a signed
  division would be wrong.
- **Ruby backend** — `sir_tdiv`/`sir_tmod`: `Integer#remainder` is already C's
  `%`, and `(a - a.remainder(b)) / b` recovers the truncated quotient exactly.
- Both backends' builtin allowlists (`SUPPORTED_BUILTINS` / `fixed_helper` +
  dispatch) extended with `tdiv`/`tmod`.
- Corpus grows with all four sign combinations, an unsigned division, and a real
  Euclid `gcd` (`%` in a loop) — byte-identical across reference `clang -fwrapv`,
  emitted Ruby, and emitted C (and clang+gcc+MSVC).  `INT_MIN / -1` is UB (the
  reference traps) so it is not a conformance case, but a unit test confirms the
  emitted C is guarded and does not crash.
- With this, **every C integer operator is implemented** — arithmetic, bitwise,
  shifts, comparisons, logical, and now division/modulo.

### Milestone 5 — bitwise `& | ^ ~` and shifts `<< >>`

- `& | ^` and unary `~` take the usual arithmetic conversions and wrap the
  result in a `Convert`, exactly like `+ - *`.
- **Shifts are the exception:** C does not common-type the operands — each is
  promoted alone, and the result has the promoted **left** operand's type (the
  right operand is only a count).  So `uint8_t x; x << c` is done at `int` and
  narrows to `uint8_t` only at the assignment.  `>>` is arithmetic on a signed
  operand and logical on an unsigned one.  Because both backends store values in
  a signed `int64`, an unsigned `>>` is routed to a distinct `u>>` builtin (C
  renders it as a `uint64_t` shift, Ruby as a plain `>>`), so a `uint64_t`/
  `size_t` with its top bit set shifts logically instead of sign-extending.
- Both backends gain the six builtins `& | ^ ~ << >>`: Ruby renders the native
  `Integer` operators; the C backend adds `_sir_band`/`_sir_bor`/`_sir_bxor`/
  `_sir_bnot`/`_sir_shl`/`_sir_shr` over `int64_t` (`<<` through `uint64_t` to
  avoid sign-bit UB; count masked `& 63`).  Both backends' builtin allowlists
  were extended.
- Corpus grows with 10 programs (and/or/xor, `~` promotion and narrowing, left
  shift + wrap into a `uint8`, logical vs arithmetic `>>`, masking) — all
  byte-identical across reference `clang -fwrapv`, emitted Ruby, and emitted C
  (and clang+gcc+MSVC).
- Division/modulo (`/ %`) still deferred — they need the truncate-vs-floor split
  (C truncates toward zero; SIR/Ruby floor) — and remain a clean positioned
  error rather than a silently-wrong floor.

### Milestone 4 — logical operators (`&&`, `||`, `!`)

- The short-circuiting logical operators lower to SIR's `and`/`or`/`not`
  builtins, reusing the milestone-2 truthiness bridge.  As a **condition** each
  operand is lowered *as a condition* (`a && b → and(cond(a), cond(b))`,
  left-associative — C's evaluation order, and the builtins short-circuit like
  C); as a **value** the resulting bool is wrapped back to C's int `0`/`1` with
  `If(bool, 1, 0)`.  So `if (x >= 0 && x < n)`, `return a && b`, and `!(x > 5)`
  all translate now.
- Manifest declares `Feature::ShortCircuit`.  Both backends already accept it and
  render `and`/`or`; the C backend gains a `_sir_not` runtime helper for `!`
  (Ruby already had `not`).
- A logical operator chain folds into a tree as deep as it is wide, so its width
  is charged against the shared depth budget, and `!!!…` recursion is bounded
  too — a 200-long `&&` chain is a clean positioned error, not a crash.
- Corpus grows with 8 logical programs (`&&` range checks in/out of range, `||`
  out-of-band, `&&`/`!` as values, `!` in a condition, a chained `&&`, and mixed
  `|| / && / !` precedence) — all byte-identical across reference `clang -fwrapv`,
  emitted Ruby, and emitted C (and clang+gcc+MSVC).
- Still deferred: bitwise (`& | ^ ~`, `<< >>`) — needs new backend builtins — and
  division/modulo (`/ %`) — needs the truncate-vs-floor split.

### Milestone 3 — early `return` (return lifting)

- A returning `if` is now **lifted** into a value-producing `Expr::If`, with the
  rest of the function becoming the continuation of the branch that does not
  return.  SIR functions yield a block value with no early-exit statement, so
  this is what makes C's guard-clause idiom — and therefore idiomatic recursion
  like `fib` — translatable at all.
- The continuation attaches only to a branch that can fall through, so the
  common guard-clause shape **never duplicates code**.
- `lower_seq` replaces the old "return must be the last statement" body walk;
  nested `{ }` blocks splice into the enclosing sequence, and `always_returns`
  drives the branch analysis (conservative: an `if` with no `else` never
  qualifies, loops are not analysed).  The walk is **iterative in two
  dimensions** — per statement *and* per sibling guard clause, both of which are
  flat sequences the parser does not bound.  (Recursing per statement overflowed
  at ~350; recursing per guard overflowed the `sign()` idiom at ~150.)  A lifted
  `if` pushes its condition and returning branch on a stack, splices the
  falling-through branch onto the work queue, and the nested `If` is folded
  bottom-up at the end.
- Four shapes are **refused** rather than mis-handled, each a positioned error:
  - `return` inside a loop (needs a break-with-value, which SIR lacks);
  - an `if` where neither branch returns on all paths but one contains a
    `return` — lifting would duplicate the continuation into both branches, and
    chained that is **4^N** IR nodes (<1 KB of C emitted ~185 MB before this
    was caught);
  - a declaration that **re-uses a name already in scope** — the symbol table is
    flat and nested blocks are spliced into the enclosing sequence, so two
    bindings collapse into one, silently taking the wrong type *and* emitting C
    that fails with `redefinition of 'v'`.  Early return sharpens the same
    hazard (the continuation is lowered inside the falling-through branch), so
    the check lives on the declaration itself and covers every binding path —
    blocks, branches, loop bodies, `for`-inits, and the lifted continuation.
    Two sequential `for (int i = …)` loops are the everyday form of this;
  - an **emitted tree deeper than the budget** — every IR consumer walks it
    recursively, and depth accumulates from three sources that all add in the
    same tree: flat operator chains (`x + 1 + 1 + …` folds left into a tree as
    deep as it is wide, and nothing else bounds it), expression nesting, and
    statement nesting (weighted 3× to match its measured stack cost).  All three
    share **one** budget.  A chain's width is *held* while its operands are
    lowered — merely checking it let widths at different nesting levels multiply
    (~14× the cap, crashing on 369 bytes) — and budgeting the sources separately
    also failed (64 guards each returning a 50-term chain passed two independent
    caps and still overflowed).  The cap is calibrated against a **debug** build
    on a **1 MiB** stack, not a roomier test-harness thread;
  - **more than that many lifted early returns in one function** — each one nests the
    emitted IR a level deeper and every IR consumer walks it recursively, so 250
    chained guards aborted the process inside the validator.  The cap makes that
    a clean error; raising it means making the validator and backends iterative.
- Corpus grows with 8 early-return programs: **recursive `fib(20)` → 6765**,
  chained guards, unbraced guards, both-branches-return, statements before a
  return, a nested `if` in an `else`, an early return of a wrapped `uint8`, and
  early return combined with a loop result — all byte-identical across reference
  `clang -fwrapv`, emitted Ruby, and emitted C (and clang+gcc+MSVC).

### Milestone 2 — control flow & comparisons

- Comparisons `< > <= >= == !=` (with the usual arithmetic conversions on their
  operands).  As a condition they lower to a SIR bool directly; as a **value**
  they lower to `If(cmp, 1, 0)`, restoring C's int-typed `0`/`1`.
- `if`/`else` → an `Expr::If` evaluated as a statement; `while` → `Stmt::While`;
  `for` desugars to `init; while (cond) { body; step }`; re-assignment `x = e`
  → `Stmt::Assign` (RHS converted to `x`'s declared type).
- The **C-vs-SIR truthiness bridge**: a non-comparison condition `e` becomes
  `!=(e, 0)` (C treats `0` as false; SIR treats it as truthy), so `while (n)`
  terminates correctly.
- Manifest now declares `Loops` + `MutableBindings`.
- Early `return` (anywhere but the function's last statement) is a clean,
  positioned error — SIR functions yield a block value with no early exit.
- Conformance corpus grows with control-flow programs (accumulator `for`,
  `while` countdown, `if/else` min, factorial, **uint8 wraparound accumulated in
  a loop → 232**, comparison-as-value, equality branch) — all byte-identical
  across reference `clang -fwrapv`, emitted Ruby, and emitted C.

- Tests: `tests/three_way_conformance.rs` — a three-way conformance corpus that,
  for each milestone-1 C program, asserts byte-identical stdout across (1)
  reference C compiled `clang -fwrapv`, (2) emitted Ruby run with `ruby`, and (3)
  emitted C compiled and run.  Covers unsigned overflow at u8/u16/u32, signed
  overflow via cast and multiply, narrowing casts, promotion order, operator
  precedence, and multi-function calls.  Each leg skips gracefully if its
  toolchain is absent.  This is the payoff of the C→SIR→Ruby initiative: a C
  program and its Ruby translation produce the same output, wraparound included.

## 0.1.0 — C→SIR lowering, milestone 1 (SIR27)

- `compile_source` / `compile` — C CST → `semantic_ir::Module` (source_language
  "c"), the first frontend to exercise SIR's type system.
- A symbol table assigns a concrete `IntSpec` to every expression; `Expr::Convert`
  nodes are inserted per C's integer promotions (narrower-than-int → i32), the
  usual arithmetic conversions (common type), assignment/initialisation, `(T)e`
  casts, and call-argument/return conversions.  Arithmetic stays exact (dynamic
  `+`/`-`/`*`), so the Convert after each width-bounded op reproduces C's
  fixed-width overflow at every step.
- Supports functions with typed params, declarations & assignments, `+`/`-`/`*`
  (unary `-` as `0 - x`), casts, `printf("%d"[\n], e)` → `print`/`puts`, and
  `return`.
- Verified: C→SIR→Ruby (real `ruby`) AND C→SIR→C (real `cc`) produce identical
  output including `uint8_t`/`int32_t` wraparound (200+100→44, 2e9+2e9→-294967296)
  and function calls.
