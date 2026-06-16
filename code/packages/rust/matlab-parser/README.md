# MATLAB Parser

A grammar-driven parser for the [MATLAB](https://en.wikipedia.org/wiki/MATLAB)
language. It tokenizes with [`matlab-lexer`](../matlab-lexer) and parses the
result with the generic `GrammarParser`, driven by the compiled
`matlab.grammar`. This is item **MA-3c** of the MATLAB frontend on
[`array-runtime`](../array-runtime); see
[`MA01-matlab-language.md`](../../../specs/MA01-matlab-language.md).

## What it does

Produces a `GrammarASTNode` tree whose `rule_name`s match the grammar rules, so
the (future) `matlab-runtime` walks it by dispatching on rule name — the same
pattern S/R use over the shared evaluator.

```rust
use coding_adventures_matlab_parser::parse_matlab;

let tree = parse_matlab("A = [1 2; 3 4]\n");
assert_eq!(tree.rule_name, "program");
```

Use `try_parse_matlab` for a `Result` instead of a panic.

## What it parses

- **The MATLAB precedence cascade** (loosest → tightest): `=` · `||` · `&&` · `|`
  · `&` · comparison (`== ~= < > <= >=`) · colon (`a:b`, `a:b:c`) · `+ -` ·
  `* / \ .* ./ .\` (matrix and element-wise multiply share a level) · unary
  `+ - ~` · power (`^ .^`) · postfix.
- **Matrix and cell literals** — `[1 2; 3 4]`, `[1, 2, 3]`, `[1; 2; 3]`,
  `{1, 'two'}`. Columns are juxtaposed (or comma-separated); rows are `;` or a
  newline (the lexer keeps bracket-interior newlines for exactly this).
- **Postfix**: transpose `'` / `.'`, calls and indexing `A(i, j)`, whole-column
  `A(:, k)`, cell indexing `C{i}`, field access `s.field`.
- **Control flow**: `if/elseif/else/end`, `for … end`, `while … end`,
  `switch/case/otherwise/end`, `try/catch/end`, `break`, `continue`, `return`,
  `global`/`persistent`.
- **Functions**: `function y = f(x) … end`, `function [a, b] = g() … end`, and
  anonymous `@(x) x.^2`.

### The `end` twist

`end` is both a **block terminator** (`if … end`) and the **last-index sentinel**
(`A(end)`, `A(2:end)`). A pre-parse hook retags every `end` *inside* `( )`/`[
]`/`{ }` to a `NAME` before parsing, so the grammar's `"end"` literals only ever
close blocks and the sentinel parses via the ordinary name path. The runtime
resolves a bracketed `end` to the dimension length.

### Known limitations (deferred)

- Whitespace-sensitive matrix rows: `[1 -2]` (two elements) vs `[1 - 2]` (one)
  collapse to one form because the lexer drops whitespace — write `[1, -2]` to
  force two elements.
- Multiple-assignment targets (`[a, b] = f()`) parse the right-hand side but not
  yet a bracketed left-hand side outside `function` returns.

## Regenerating the embedded grammar

`src/_grammar.rs` is generated from `code/grammars/matlab.grammar` with the
grammar-tools CLI — never hand-edit it:

```sh
grammar-tools compile-grammar code/grammars/matlab.grammar \
  -o code/packages/rust/matlab-parser/src/_grammar.rs
```

## Testing

```sh
cargo test -p coding-adventures-matlab-parser
```
