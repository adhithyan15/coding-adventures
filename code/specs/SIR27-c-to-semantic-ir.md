# SIR27 — C → Semantic IR (Rust frontend)

## Status

A **frontend** for the narrow-waist Semantic IR
([SIR10](SIR10-narrow-waist-semantic-ir.md)) — the mirror of the existing
`ruby-to-semantic-ir` / `python-to-semantic-ir` frontends, but for a **strict,
typed** source language.  It is the last piece of the **C → SIR → Ruby**
initiative: with it, a C program lowers to SIR carrying its exact integer
width / wrapping / truncating semantics (via the
[`Convert`](SIR26-integer-conversions.md) node), and the Ruby backend then
reproduces the C program's results bit-for-bit.

Implemented across three crates, built on the repo's grammar framework:
`c-lexer` (wraps `lexer::GrammarLexer`), `c-parser` (wraps
`parser::GrammarParser`, yields the generic `GrammarASTNode` CST), and
`c-to-semantic-ir` (walks the CST, does C's static typing, and emits
`semantic_ir::Module`).  Grammars live in `code/grammars/c/{c.tokens,c.grammar}`
and compile to each crate's `src/_grammar.rs` via the `grammar-tools` CLI.

## Why C is the interesting frontend

Every existing frontend is a **loose** source (Ruby/Python/Twig) whose integers
are arbitrary-precision, so it emits the default `Int(Arbitrary)` and never a
`Convert`.  C is the first **strict** source: it *has* `int32_t`, `uint8_t`,
defined unsigned wraparound, and truncating casts.  So the C frontend is the
first to exercise SIR's type system — it assigns a concrete `IntSpec` to every
expression and inserts `Convert` nodes per C's conversion rules.

## Subset (v1 — integer core)

Deliberately small and honest, focused on the type/wrap/truncate story:

- **Top level:** a sequence of function definitions.  `int main(void)` is the
  entry.
- **Types:** the integer types only — `void`; the native specifiers `char`,
  `short`, `int`, `long`, `signed`, `unsigned` (and the multi-word combinations
  C allows, e.g. `unsigned int`, `long long`, `unsigned char`); and the
  `<stdint.h>` fixed-width typedef names `int8_t int16_t int32_t int64_t
  uint8_t uint16_t uint32_t uint64_t` plus `size_t`.
- **Declarations:** local variables with an optional initialiser
  (`int32_t x = 5;`), function parameters.
- **Expressions** with full C precedence: assignment `=`; `|| &&`; bitwise
  `| ^ &`; equality `== !=`; relational `< <= > >=`; shifts `<< >>`; additive
  `+ -`; multiplicative `* / %`; unary `- ~ !` and the **cast** `(T)e`; postfix
  function calls; primaries (identifier, integer literal with optional
  `u`/`l`/`ll` suffix, parenthesised expression).
- **Statements:** expression statement, `if`/`else`, `while`, `for`, `return`,
  compound `{ … }`.
- **Output:** a restricted `printf` — `printf("<fmt>", e)` where `<fmt>` is a
  single integer conversion (`%d`/`%u`/`%ld`/`%lld`/…) optionally followed by
  `\n` — lowers to the SIR `print`/`puts` builtin; and `putchar(e)`.
- **Preprocessor:** only `#include <stdint.h>` / `#include <stdio.h>` (ignored).
  No `#define`, no macros, no other directives.

### Dodging C's context-sensitivity

C is not context-free (the typedef/identifier ambiguity) and has a
preprocessor.  The v1 subset **side-steps both** so the regex `.tokens` + EBNF
`.grammar` suffice with no symbol-table feedback:

- The `<stdint.h>` names and `size_t` are lexed as **keywords**, not typedef
  names — so a type is always a keyword sequence and never an identifier.  (No
  user `typedef` in v1.)
- `#include <…>` lines are removed by a `c-lexer` **pre-tokenize hook**; line
  continuations (`\` at end of line) are spliced there too.  No other
  preprocessing.

## The type checker (in the lowering pass)

Like the Python/Ruby frontends, C's static analysis lives in the
`c-to-semantic-ir` lowering pass (the reusable `grammar-type-checker` targets
the IIR/VM path, not the SIR waist).  The lowerer keeps a symbol table
(name → `IntSpec`) and assigns a type to every expression, then **inserts
`Convert` nodes** at exactly the points C changes an integer's width:

**C type → `IntSpec`** (`IntWidth`×`signed`×`Overflow`, per SIR21):

| C type | `IntSpec` |
|---|---|
| `uint8_t` / `unsigned char` | `{W8, unsigned, Wrap}` |
| `int8_t` / `signed char` | `{W8, signed, Undefined}` |
| `uint16_t` / `unsigned short` | `{W16, unsigned, Wrap}` |
| `int16_t` / `short` | `{W16, signed, Undefined}` |
| `uint32_t` / `unsigned` / `unsigned int` | `{W32, unsigned, Wrap}` |
| `int32_t` / `int` | `{W32, signed, Undefined}` |
| `uint64_t` / `unsigned long`* / `size_t` | `{W64, unsigned, Wrap}` |
| `int64_t` / `long`* / `long long` | `{W64, signed, Undefined}` |
| `char` | `{W8, signed, Undefined}` (plain char treated as signed) |

(*`long` is modelled as 64-bit — the LP64 convention.  Signed overflow is
`Undefined`, rendered as two's-complement wrap: the whole initiative compiles
the reference C with `-fwrapv`, and the backends realise `Undefined` as wrap.)

**Where `Convert`s go** — modelling C's *integer promotions* and *usual
arithmetic conversions*:

1. **Integer promotion.** Any operand narrower than `int` (`W8`/`W16`) is
   promoted to `int` (`W32` signed) before use → a `Convert{i32}` around it.
2. **Usual arithmetic conversions.** For a binary `a ⊕ b`, both operands are
   converted to their common type `C` (the wider, unsigned-wins per C's rank
   rules); the operation is performed at `C`, and its result is a fresh value
   of type `C` → the sum/product is wrapped in `Convert{C}`.  Because SIR
   arithmetic stays **exact** (dynamic `+`/`-`/`*`), the `Convert{C}` after the
   op is what enforces C's fixed-width overflow — at *every* operation.
3. **Assignment / initialisation** `x = e` (x : T): `Convert{T}(e)`.
4. **Cast** `(T)e`: `Convert{T}(e)`.
5. **Call arguments / `return`:** `Convert{param-type}` / `Convert{return-type}`.

A `Convert` whose target equals the value's existing type is still emitted (the
backends render an in-range widen as the identity, so it is free), keeping the
lowering a mechanical, auditable transcription of C's rules.

Worked example — `uint8_t c = a + b;` (a,b : uint8_t):

```text
c := Convert{u8}( Convert{i32}( +( Convert{i32}(a), Convert{i32}(b) ) ) )
```

promotes a,b to int, adds exactly, (the i32 result Convert is identity here),
narrows to u8 → `300 → 44`.  The Ruby backend renders
`c = sir_u8(sir_i32(sir_i32(a) + sir_i32(b)))`; the C backend renders the
equivalent `_sir_convert(...)` chain; both agree with `clang -fwrapv`.

## Control flow & comparisons (milestone 2)

Milestone 1 delivered typed `+`/`-`/`*`, casts, declarations, `printf`, and a
trailing `return`.  Milestone 2 adds **comparisons and control flow**, whose one
subtlety is a **truthiness mismatch** between the two languages:

- In **C**, a condition is an integer: `0` is false, any non-zero is true, and a
  comparison (`a < b`) yields an `int` that is `0` or `1`.
- In **SIR**, only `nil`/`false` are falsy — **`0` is truthy** — and a comparison
  builtin yields a `bool`.

So the lowering must bridge in two directions:

1. **C condition → SIR bool** (used by `if`/`while`/`for`).  `lower_cond(e)`:
   - if `e` is syntactically a comparison (`relational`/`equality`), lower it to
     the SIR comparison builtin directly — that already yields a `bool`;
   - otherwise `e` is an integer expression, so emit `!=(e, 0)` — the SIR bool
     that is true exactly when C would treat `e` as true.  This is what stops
     `while (x)` from looping forever on `x == 0` (which SIR would call truthy).

2. **C comparison as an r-value → SIR int** (`int b = a < c;`).  A comparison has
   type `int` in C, so when its result is *used as a value* it lowers to
   `If(cmp, 1, 0)` typed `i32` — restoring C's `0`/`1`.  (`If`'s two branches are
   the SIR blocks `{1}` and `{0}`.)

Both comparison forms apply the **usual arithmetic conversions** to their
operands first (promote, then common type), exactly like arithmetic — so
`(uint8_t)200 < (uint8_t)100` compares the promoted `int` values, matching C.

**Statements added:**

| C | SIR |
|---|---|
| `if (c) S1 else S2` | `ExprStmt(If{ lower_cond(c), block(S1), block(S2) })` |
| `while (c) S` | `Stmt::While{ lower_cond(c), block(S) }` (feature `Loops`) |
| `for (init; c; step) S` | desugars to `init; While{ c′, block(S; step) }` |
| `x = e;` | `Stmt::Assign{ x, scope, Convert{type(x)}(e) }` (feature `MutableBindings`) |

`for` desugars to the `while` form: the init clause (a declaration or an
expression) is emitted before the loop, the step expression is appended to the
end of the loop body, and an absent condition becomes `true`.  A nested `{ … }`
block is spliced into the enclosing statement list (v1 does not model per-block
scopes; the flat symbol table is shared).

Logical `&& ||`, unary `!`, bitwise, and `/`/`%` remain deferred.

## Early `return` — return lifting (milestone 3)

SIR functions yield their block's **value**; there is no early-exit statement.
C exits early constantly (guard clauses), so milestone 3 **lifts** a returning
`if` into a value-producing `Expr::If`, making **the rest of the function the
continuation of the branch that does not return**:

```text
int fib(int n) {                     (function fib (n)
  if (n < 2) return n;                 (block
  return fib(n-1) + fib(n-2);            (if (< n 2)
}                                            (block n)
                                             (block (+ (fib (- n 1)) (fib (- n 2)))))))
```

The lowering walks a statement sequence (`lower_seq`) and, for each head:

| head | result |
|---|---|
| `return e;` | the block's value is `Convert{ret}(e)`; the rest is unreachable |
| `{ … }` | its items are spliced into this sequence (v1 has flat scoping) |
| `if` containing a `return` | `If(cond, then′, else′)` — see below |
| anything else | lower it as a statement, then continue with the tail |

For a lifted `if`, each branch is lowered with the continuation appended **only
if that branch can fall through** (`always_returns` is false).  So the
guard-clause shape — where the `then` always returns — attaches the tail to the
`else` alone and **never duplicates code**.

`always_returns` is deliberately conservative: an `if` without an `else` never
qualifies, and loops are not analysed (a `while` may iterate zero times).

The sequence walk is **iterative in two dimensions**, and both matter because
both are *flat* sequences the parser does not bound:

- per **statement** — a function body is an unbounded statement list;
- per **sibling guard clause** — `if (a) return 1; if (b) return 2; …` (the
  `sign()` idiom) is equally flat.

So a lifted `if` does not recurse into its continuation.  Its condition and its
*returning* branch are pushed on a stack, the falling-through branch is spliced
onto the work queue, and the nested `If` is folded bottom-up once the walk ends.
Recursion remains only for a *nested* sub-sequence, which the parser's rule-depth
guard bounds.

### Four shapes that are refused (rather than mis-handled)

1. **`return` inside a loop.**  Leaving a `while`/`for` early needs a
   break-with-value, which SIR has no node for.
2. **An `if` where neither branch returns on all paths but one contains a
   `return`.**  Lifting it would place the continuation in *both* branches.
   That is semantically fine (exactly one runs) but the duplication compounds
   through nesting — N chained guards of this shape produce **4^N** IR nodes, so
   well under 1 KB of C can emit hundreds of megabytes.  A future version can
   hoist the continuation into a synthesized function called from both branches,
   making the transform linear; until then it is a positioned error.
   (Shadowing / name re-use *used* to be refused here; milestone 7 makes it work
   — see below.  Only re-declaring a name in the **same** block is still an
   error.)
3. **An emitted tree deeper than the budget.**  Every consumer of the IR walks
   it recursively, so the frontend caps how deep a tree it will build.  Depth
   accumulates from **three** independent sources that all add in the same tree
   and the same recursion, so they share **one** budget:
   - **flat operator chains** — `x + 1 + 1 + …` is one node with N operands that
     folds left into an N-deep tree, and nothing else bounds N;
   - **expression nesting** — `((((x))))`;
   - **statement nesting** — nested `if`/`while`/`for` bodies and blocks
     (weighted 3×, matching its measured ~3× lowering-stack cost per level).

   Two subtleties, both found the hard way: a chain's width must be **held**
   while its operands are lowered, not merely checked on entry (otherwise widths
   at different nesting levels each restart from the same base and *multiply* —
   ~14× the cap, aborting on a 369-byte input); and the sources must be budgeted
   *jointly* (64 guards each returning a 50-term chain passed two independent
   caps and still overflowed).

   The cap is calibrated empirically against the most hostile realistic
   configuration — a **debug** build on a **1 MiB** stack.  Calibrating against
   a test-harness thread instead is exactly how earlier versions looked safe
   while crashing in the wild.
4. **More than that many lifted early returns in one function.**  Each lifted guard
   nests the emitted IR one level deeper, and every consumer of that IR walks it
   *recursively* — the validator, all five backends, the text printer, even
   `Drop`.  Measured: 150 chained guards lower, validate and emit fine while 250
   abort the process inside the validator.  The bound that matters is therefore
   on the **output** depth, not the lowering, so the frontend caps it and reports
   a positioned error.  Lifting the cap means making those consumers iterative —
   a cross-cutting change well beyond this frontend.

## Logical operators (milestone 4)

`&&`, `||`, and unary `!` — the short-circuiting logical operators — reuse the
same C-vs-SIR truthiness bridge as milestone 2.  In C each operand is tested for
truthiness (`!= 0`) and the result is an `int` `0`/`1`; SIR has short-circuiting
`and`/`or` builtins (and `not`) whose operands are evaluated under SIR
truthiness.  So the two directions mirror the comparison handling:

- **As a condition** (`if (a && b)`), each operand is lowered *as a condition*
  (`lower_cond`, which already yields the right SIR bool for a comparison or an
  `!= 0`), and the operator becomes the matching short-circuiting builtin:
  `a && b → and(cond(a), cond(b))`, `a || b → or(cond(a), cond(b))`,
  `!a → not(cond(a))`.  Left-associative, so `a && b && c` is `and(and(a,b),c)` —
  exactly C's evaluation order, and the SIR builtins short-circuit just as C
  does (the backends render `and`/`or` as `&&`/`||` in Ruby and as a
  short-circuiting `if` chain in C).
- **As a value** (`int r = a && b;`), the bool is wrapped back to C's `int`
  `0`/`1` with `If(bool, 1, 0)` — the same `if_int` used for a bare comparison.

A logical operator chain folds into a tree as deep as it is wide, so — like an
arithmetic chain — its width is charged against the shared depth budget.

`Feature::ShortCircuit` is added to the module manifest (both backends already
accept it and render `and`/`or`; the C backend gains a `_sir_not` for `!`).

Bitwise and division/modulo remain deferred (see below).

## Bitwise & shifts (milestone 5)

`& | ^` and unary `~` follow the ordinary path: promote, take the usual
arithmetic conversions to a common type, apply the operator there, and wrap the
result in a `Convert` to enforce the width — identical in shape to `+ - *`.

**Shifts (`<< >>`) are the one exception to the usual arithmetic conversions.**
C does *not* bring the two operands to a common type: each is promoted on its
own, the result has the type of the promoted **left** operand, and the right
operand is only a count.  So `uint8_t x; x << c` is performed at `int` (x's
promoted type) and stays `int` until it is used — narrowed to `uint8_t` only at
the assignment, exactly as C does.  `>>` is arithmetic on a signed operand and
logical on an unsigned one.  This *almost* falls out for free — but the backends
store every value in a signed `int64`, and a `uint64_t`/`size_t` with its top
bit set is a *negative* int64, on which a native `>>` would sign-extend.  So the
frontend picks the shift builtin by the promoted left operand's signedness:
signed `>>` → `>>` (arithmetic); unsigned `>>` → **`u>>`**, which the C backend
renders as a `uint64_t` shift (logical for every width) and Ruby renders as a
plain `>>` (its unsigned value is already a non-negative Integer).

The six operators lower to the builtins `&`, `|`, `^`, `~`, `<<`, `>>`.  Both
backends gain them: Ruby renders the native `Integer` operators; the C backend
gains `_sir_band`/`_sir_bor`/`_sir_bxor`/`_sir_bnot`/`_sir_shl`/`_sir_shr` runtime
helpers over `int64_t` (`<<` through `uint64_t` so a shift into the sign bit is
not UB; both mask the count `& 63` defensively — a count ≥ width is UB in C
anyway).

Division/modulo (`/ %`) are handled in milestone 6 (below).

## Division & modulo (milestone 6)

`/` and `%` are the one place C and SIR **disagree on rounding**: C division
*truncates toward zero* and `%` takes the sign of the dividend (`-7 / 2 == -3`,
`-7 % 2 == -1`), whereas SIR/Ruby `/`/`%` and the C backend's existing
`_sir_ifloordiv` *floor* toward −∞ (`-7 / 2 == -4`, `-7 % 2 == 1`).  So they
lower to **dedicated `tdiv`/`tmod` builtins**, never the flooring `/`/`%`.

- **C backend** — C's native `int64_t /` and `%` already truncate, so
  `_sir_itdiv`/`_sir_itmod` are thin wrappers with two guards: division by zero
  (UB in C; fail loudly) and `INT64_MIN / -1` (signed-overflow UB, and x86
  hardware traps on it — return the two's-complement wrap `-fwrapv` would give,
  which the width `Convert` then narrows).
- **Ruby backend** — `Integer#/` floors, so `sir_tdiv`/`sir_tmod` recover
  truncation exactly: `Integer#remainder` is already C's `%` (dividend sign), and
  `(a - a.remainder(b)) / b` is an exact multiple of `b`, so flooring it yields
  the truncated quotient.

Both agree with `clang -fwrapv` across all four sign combinations.  `INT_MIN /
-1` is deliberately *not* a conformance case — it is UB and the reference program
traps — but the backends stay defined so they never crash on it.

## Per-block scoping (milestone 7)

Earlier milestones kept a **flat** symbol table and *refused* any declaration
that re-used a live name — so shadowing, and even two sequential
`for (int i = …)` loops, were errors.  Milestone 7 replaces it with a **scope
stack**: a scope is pushed on entering a `{ }` block, an `if`/`else`/loop body,
or a `for`'s init+body region, and popped on leaving it.  A declaration binds in
the innermost scope; a reference resolves innermost-outward.

Because SIR's namespace is flat while C's is not, every declaration is given a
**unique SIR name** — the C name itself, or `name__2`, `name__3`, … if it
shadows an outer binding or re-uses one a sibling block used.  References
resolve to the binding's SIR name, so two distinct C variables that share a
spelling never collide, and the emitted C/Ruby is correct.  This *removes* the
milestone-3 shadowing miscompile hazard by construction: distinct variables have
distinct names, so one can never clobber another.

The subtlety is the interaction with early-return lifting, which merges a branch
body and the continuation into one SIR block.  The lifting trampoline carries a
**`PopScope` marker** on its work queue: a spliced block pushes a scope, queues
its items, then a `PopScope`, so the block's declarations go out of scope exactly
at its `}` and the following continuation is lowered in the enclosing scope —
correct lifetime even though the two are concatenated.

Still enforced: re-declaring a name in the **same** block is a C error; a
variable is undeclared once its block has closed.  (A self-referential
initializer like `int v = v + 1;` in a shadowing block reads the *uninitialized*
inner `v` per C's scope rule — that is UB, and not something the translator
conforms to.)

## Floating point (milestone 9)

Every earlier milestone tracked one thing per expression — an `IntSpec`.  Floats
add a **second value track**.  The lowering now carries a `CType` for each
expression:

```
enum CType { Int(IntSpec), Double }
```

`float`, `double` (and, conceptually, `long double`) all map to the one
`CType::Double`, which lowers to `SirType::Float` — SIR has a single 64-bit IEEE
float, so `float`'s narrower 32-bit rounding is *not* modelled (a `float` is
treated as a `double`).  A floating-point literal (`3.14`, `.5`, `1e10`, `1.0f`)
lowers to `Expr::FloatLit`; the `f`/`l` storage suffix is dropped.

**The usual arithmetic conversions, extended.**  When an operator mixes an
integer and a `double`, C converts the integer operand to `double` and does the
operation in floating point.  The frontend inserts that conversion explicitly as
a `to_f` builtin, so `1 + 2.5` lowers to `+(to_f(1), 2.5)` — a *float* add whose
result is a `double`.  Crucially:

- **No width `Convert`.**  Integer results are wrapped to their width after every
  op (that is the whole point of the integer track); a `double` has no width to
  wrap to, so a float op emits the bare `+`/`-`/`*`/`/` builtin.
- **`/` is true division.**  On integers `/` lowers to the truncating `tdiv`
  (C truncates toward zero); on `double` it is real division, the plain `/`
  builtin.  The backends already promote to float when either operand is a float
  (`_sir_divide_v` in C, native `/` in Ruby), so `7.0 / 2.0 == 3.5`.
- **`%`, `&`, `|`, `^`, `<<`, `>>`, `~` are rejected on `double`** — they are not
  defined on floating point in C, so lowering errors with a clear message rather
  than mis-emitting.

**Casts and conversions** flow through two builtins: `to_f` (int → double, C's
implicit widening) and `to_i` (double → int, *truncating toward zero*, exactly
like C's `(int)double`).  A `double`→integer cast is `Convert(to_i(e), spec)` —
truncate, then narrow to the destination width — so a float→int cast reproduces C
bit-for-bit for values that fit the destination.  Both backends render these
natively: Ruby `.to_f`/`.to_i`, C `_sir_to_f`/`_sir_to_i`.

**Conditional feature flag.**  `Feature::Floats` is declared **only** when the
program actually used a float type or literal.  An integer-only program stays
float-free, so its SIR — and every backend's output — is byte-for-byte what it
was before this milestone.

**Conformance and the display convention.**  Reference C's `printf("%f", …)`
prints `3.140000`; the backends' float display prints `3.14`.  That divergence is
a *display* question, orthogonal to the arithmetic, so the conformance corpus
sidesteps it: every float program casts its result to `(int)` **inside the C
source** before `printf("%d", …)`.  All three legs then format the same integer,
and what is actually proven is the floating-point *computation* — mixed
promotion, true division, loop accumulation, and float→int truncation — is
identical across reference C, emitted Ruby, and emitted C.

## Pipeline

```text
C source
  │  c_lexer::tokenize_c        (GrammarLexer + #include/line-continuation hook)
  ▼
Vec<Token>
  │  c_parser::parse_c          (GrammarParser → generic CST)
  ▼
GrammarASTNode  (rule_name + children)
  │  c_to_semantic_ir::compile  (symbol table, C typing, Convert insertion)
  ▼
semantic_ir::Module            (source_language = "c"; validated)
```

## Public API

```rust
// c-to-semantic-ir
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<semantic_ir::Module, CLowerError>;
pub fn compile_source(source: &str, module_name: &str) -> Result<semantic_ir::Module, CLowerError>;
pub struct CLowerError { pub message: String, pub line: usize, pub column: usize }
```

## Verification

- **Parser:** `c-lexer` / `c-parser` unit tests over the subset.
- **Round-trip:** `c-to-semantic-ir` dev-depends on `semantic-ir-to-c`; a C
  program → SIR → C compiles and runs (the C→SIR→C loop).
- **End-to-end (the payoff):** a corpus of small C programs, each run three
  ways — the **reference C compiled `clang -fwrapv`**, the **emitted Ruby**
  (`semantic-ir-to-ruby` via `ruby`), and the **emitted C**
  (`semantic-ir-to-c`) — asserting **byte-identical stdout**, focusing on
  `uint8`/`int32` overflow, narrowing casts, and promotion order.

## Rollout (one PR per stage)

1. **Spec + parser** (this spec) — `code/grammars/c/{c.tokens,c.grammar}` +
   `c-lexer` + `c-parser`, tokenise/parse the subset, tests green.
2. **Lowering** — `c-to-semantic-ir` (symbol table, C typing, `Convert`
   insertion) + `semantic-ir-to-c` round-trip tests.  Grow in milestones
   (literals → typed expressions → control flow → functions).
3. **Conformance** — the three-way byte-identical corpus above.

## Out of scope (v1)

- Pointers, arrays, structs, unions, enums, `typedef`.
- `long double` (SIR has a single 64-bit `Float`; `float`/`double`/`long double`
  all map to it — see the floating-point section).  Precise `float` (32-bit)
  rounding is therefore not modelled: `float` is treated as `double`.
- The full preprocessor (`#define`, macros, conditional compilation).
- Multiple translation units / real headers (only `#include <stdint.h|stdio.h>`
  is recognised and ignored).
- Full `printf` format-string semantics (only a single integer conversion).
