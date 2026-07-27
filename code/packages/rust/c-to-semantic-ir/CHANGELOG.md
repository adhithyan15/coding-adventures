# Changelog

## Unreleased

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
