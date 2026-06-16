# MA01 — MATLAB: a numeric-array language frontend on `array-runtime`

## Status

Active spec. MATLAB is the **first numerical/array-language frontend** of the
historical math-languages roadmap ([`HML00`](HML00-historical-math-languages-roadmap.md)),
built on the shared array substrate ([`MA00`](MA00-array-runtime.md), now merged:
the `Array` value model + CPU reference ops + `matrix-ir` lowering + end-to-end
`execute()` on the CPU executor). MATLAB is to `array-runtime` what S/R are to
the shared statistical evaluator: a thin lexer → parser → runtime over a value
model that already exists.

This spec is **MA-3a**, the first of a sequence of one-PR items (§7). It commits
the language design — including the lexer strategy whose subtlety (§3) is the
whole reason the lexer gets its own carefully-specified item — before any
frontend code lands.

## §1 Why MATLAB, and why it fits `array-runtime` exactly

MATLAB (Cleve Moler, ~1979; "MATrix LABoratory") began as a teaching interface
to LINPACK/EISPACK: an interactive language in which *the matrix is the only
data type* and everything — scalars, vectors, strings — is a special case of a
2-D (later N-D) array. That is precisely the `array-runtime` value model.

Two design choices in `array-runtime` were made **for this frontend**:

- **Column-major storage.** `array-runtime`'s `Array` is column-major (element
  `(r,c)` at `c*nrows + r`) — Fortran/MATLAB order. MATLAB's `reshape`, linear
  indexing (`A(k)`), and `[a; b]` literal semantics all assume column-major, so
  the frontend maps **directly** onto the substrate with no re-ordering.
- **GPU dispatch by lowering.** A MATLAB `A * B` lowers to `array-runtime`'s
  `execute(MatMul, …)`, which plans the op and runs it on the cheapest backend
  (CPU today; a GPU executor the moment one is registered) — `gpuArray` for
  free, by cost, with no language-level GPU syntax. ([`MA00`](MA00-array-runtime.md) §2.)

So the numeric core is *done*. MATLAB adds **syntax** (matrix literals, ranges,
element-wise operators, indexing, `end`, control flow) and a small amount of
language semantics (1-based indexing, command/function dual, char arrays).

## §2 Language scope (the historical core)

The target is the expression-and-script core of classic MATLAB, faithful enough
to run textbook linear-algebra sessions. In scope across MA-3:

- **Everything is a matrix.** A scalar is `1×1`, a row vector `1×n`, a column
  `n×1`. Built on `array-runtime::Array`.
- **Matrix literals.** `[1 2 3]` (row), `[1; 2; 3]` (column), `[1 2; 3 4]`
  (2×2). Inside `[ ]`, **whitespace and `,` separate columns** and **`;` or a
  newline separates rows** — so the lexer/parser must keep those significant
  (§3, §6). Concatenation: `[A B]`, `[A; B]`.
- **Ranges (the colon).** `1:5` → `1 2 3 4 5`; `0:2:10` → step 2; `10:-1:1`.
  `a:b` and `a:step:b`. Lowers to an `array-runtime` row vector.
- **Operators.**
  - Matrix: `*` (matmul → `execute(MatMul)`), `/` `\` (right/left divide), `^`
    (matrix power), `'` (conjugate transpose), `.'` (plain transpose).
  - Element-wise: `.*` `./` `.\` `.^` and `+ -` (which are already element-wise),
    each lowering to `execute(Elementwise, …)` / `array-runtime::ops`.
  - Comparison/logical: `== ~= < <= > >=`, `& | ~`, short-circuit `&& ||`.
- **Indexing (1-based).** `A(i)`, `A(i,j)`, `A(:,k)` (whole column), `A(end)`,
  `A(2:end)`. `end` is the index of the last element along that dimension — a
  context-sensitive keyword that is only `end` inside `( )`/`[ ]` indexing.
- **Assignment & statements.** `x = expr`. A trailing `;` suppresses display;
  without it the result auto-prints as `ans =` / `x =` (MATLAB's echo).
- **Control flow.** `if/elseif/else/end`, `for v = range … end`,
  `while … end`, `break`, `continue`, `switch/case/otherwise/end`.
- **Functions.** `function y = f(x) … end` and anonymous `@(x) x.^2`. Built-ins
  map onto `array-runtime`: `zeros`, `ones`, `eye`, `size`, `length`, `numel`,
  `reshape`, `sum`, `mean`, `max`, `min`, `transpose`.
- **Strings.** Char arrays `'abc'` (a `1×n` of characters) and, modern, string
  scalars `"abc"`. The numeric substrate stays `f64`; strings are a thin layer.
- **Comments.** `% line comment`, `%{ … %}` block comments, `...` line
  continuation.

**Deferred (post-MA-3):** cell arrays `{}`, structs, classdef/OO, N-D arrays
beyond rank 2, sparse matrices, the full built-in library, `.m` file loading,
command-syntax built-ins beyond the common cases, complex numbers (the substrate
is real `f64` for now — mirrors how R's `1i` is parsed-but-unsupported).

## §3 The lexer's hard problem: `'` is transpose *and* a string

This is the reason the lexer is its own item (MA-3b) and is specified here.

In MATLAB a single quote `'` is **both** the (conjugate) transpose operator and
the char-array delimiter, disambiguated **only by the preceding token**:

```matlab
A'              % transpose of A
'hello'         % a char-array literal
A' * B'         % (A transpose) * (B transpose)  — NOT the string ' * B'
x = [1 2]'      % transpose of a row → a column
y = 'it''s'     % the string  it's   (doubled '' is an escaped quote)
```

The rule MATLAB uses: **`'` is the transpose operator when the previous
significant token is a value-terminator** — an identifier, a number, a string,
a closing `)` `]` `}`, or another postfix `'`/`.'` — and **starts a string
otherwise** (after an operator, an open bracket, a comma/semicolon, or at the
start of input).

A pure regex token grammar (the `grammar-tools` `.tokens` format used by
`s-lexer`/`r-lexer`) **cannot** express this: a greedy `/'[^']*'/` rule lexes
`A' * B'` as `A` then the string `' * B'`. So MA-3b will **not** put `'`-strings
in the regex grammar. Two implementable strategies, in preference order:

1. **A context hook in `matlab-lexer`** (preferred). The `.tokens` grammar
   defines `TRANSPOSE = "'"` (single char) and `DQ_STRING = /"…"/` only. A
   pre-tokenize scan in the crate (analogous to `r-lexer`'s
   `drop_bracketed_newlines` post-hook, but running *before* the regex lexer)
   walks the source, and at each `'` decides — from the previous emitted
   token's type — whether to consume a char-array literal (handling the `''`
   escape) or emit a bare `TRANSPOSE`. The regex lexer never sees an ambiguous
   `'`. This keeps char arrays *and* transpose correct.
2. **Lexer modes** (`grammar-tools` `ModeTransition`/`TransitionAction`, already
   in the `TokenGrammar` model): value-producing tokens transition into an
   "expect-transpose" mode where `'` is `TRANSPOSE`; operators/open-brackets
   transition into an "expect-string" mode where `'…'` is a string. More
   declarative but verbose; fallback if the hook proves awkward.

MA-3b will implement strategy 1 and document the decision table. This spec
fixing the rule up front is what lets that PR be small and correct.

Other lexer specifics (MA-3b):

- **`.` is overloaded:** the element-wise prefix (`.*` `./` `.\` `.^` `.'`),
  the field-access dot (`s.field`), and the decimal point (`3.14`). Longest-
  match ordering (the `.op` operators before bare `.`) resolves it.
- **`...` line continuation** splices the next physical line (drop the
  continuation + newline).
- **`%` line comments and `%{ %}` block comments** (block markers must be alone
  on their lines, as in MATLAB).
- **Significant newlines and `;`/`,`** as statement/row terminators, suppressed
  inside `( )` (call/index args) but **kept inside `[ ]`/`{ }`** because there
  they separate matrix rows — the inverse of the S/R bracket rule, and another
  reason the hook is MATLAB-specific.

## §4 Reuse strategy (mirror S → R)

- **Lexer/parser:** the `grammar-tools` frontend, exactly as S/R. `matlab.tokens`
  / `matlab.grammar` compile to committed `_grammar.rs` in `matlab-lexer` /
  `matlab-parser` via the grammar-tools CLI (`compile-tokens` / `compile-grammar`).
- **Runtime:** `matlab-runtime` walks the parse tree and computes over
  `array-runtime::Array`, lowering arithmetic/matmul through
  `array-runtime::{ops, execute}`. The value model is shared, not reinvented.
- **REPL & binary:** `matlab-repl` + a `matlab` binary, mirroring `s-repl`/`R`.

## §5 Octave

GNU Octave is to MATLAB roughly what R is to S — a compatible reimplementation
with small additions (`#` comments, `++`/`--`, `endif`/`endfor`, `!=`). Once the
MATLAB frontend exists, Octave is a thin sibling: the same `array-runtime`
runtime with an `octave.tokens`/`octave.grammar` that adds those forms (the S→R
playbook). Tracked for after MA-3.

## §6 Crate layout

```
matlab-lexer/    src/{lib.rs, _grammar.rs}      ← MA-3b (+ code/grammars/matlab.tokens)
matlab-parser/   src/{lib.rs, _grammar.rs}      ← MA-3c (+ code/grammars/matlab.grammar)
matlab-runtime/  src/{lib.rs, eval.rs, value.rs, builtins.rs}   ← MA-3d
matlab-repl/     src/{lib.rs, main.rs}          ← MA-3d (the `matlab` binary)
```

Each new crate carries the standard `Cargo.toml` (src layout), `BUILD`,
`BUILD_windows`, `README.md`, `CHANGELOG.md`, `required_capabilities.json`, and
is registered in `code/packages/rust/Cargo.toml` members.

## §7 Rollout (one item = one PR)

- **MA-3a — this spec.** The language design + the `'` lexer strategy (§3).
- **MA-3b — `matlab-lexer`.** `code/grammars/matlab.tokens` + the crate, with
  the `'` transpose/string context hook, `.op` element-wise operators, `%`/`%{
  %}` comments, `...` continuation, and the `[ ]`/`{ }` newline rule. Tokenizes
  a textbook session.
- **MA-3c — `matlab-parser`.** `matlab.grammar`: matrix literals (`[a b; c d]`),
  ranges (`a:b:c`), the operator-precedence cascade (element-wise vs matrix,
  transpose postfix, unary), indexing `A(i,j)`/`A(:,k)`/`A(end)`, and control
  flow.
- **MA-3d — `matlab-runtime` + `matlab-repl` + the `matlab` binary.** A working
  REPL: evaluate matrix expressions over `array-runtime`, with `*`→`execute`
  (MatMul) and element-wise ops lowering to the substrate, 1-based indexing,
  `ans =`/`x =` echo, and the core built-ins.
- **MA-3e+ — Octave** (§5), then the wider built-in library, then APL/Maxima/
  Wolfram per [`HML00`](HML00-historical-math-languages-roadmap.md).

## §8 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MA00`](MA00-array-runtime.md) (the substrate), [`R00`](R00-r-language.md) and
[`S00`](S00-s-language.md) (the frontend-on-shared-runtime playbook this mirrors).
External: Moler, *MATLAB User's Guide* (1980s); the column-major storage and
colon/`end` semantics are the load-bearing historical details.
