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

**Early `return` is still out of scope.**  SIR functions yield their block's
*value* — there is no early-return statement — so `return` is accepted **only as
the function's last statement** (where it supplies that value).  A `return`
inside an `if`/`while`/`for` body is a clear, positioned error rather than a
silent miscompile; lifting it into nested `If` values is a later milestone.
Logical `&& ||`, unary `!`, bitwise, and `/`/`%` also remain deferred.

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

- Pointers, arrays, structs, unions, enums, `typedef`, floats.
- The full preprocessor (`#define`, macros, conditional compilation).
- Multiple translation units / real headers (only `#include <stdint.h|stdio.h>`
  is recognised and ignored).
- Full `printf` format-string semantics (only a single integer conversion).
