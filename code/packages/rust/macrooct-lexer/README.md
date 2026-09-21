# `macrooct-lexer` (PREP01 slice 1)

Tokenizes **MacroOct** source text using the grammar-driven Rust lexer. Thin
wrapper around the generic [`GrammarLexer`](../lexer) over an auto-generated
token grammar (`src/_grammar.rs`, compiled from
[`code/grammars/macrooct/macrooct.tokens`](../../../grammars/macrooct/macrooct.tokens)
via `grammar-tools`).

## What MacroOct is

**[Oct](../../../specs/OCT00-oct-language.md), plus a preprocessor, and nothing
else.**

```macrooct
@include "ports.macrooct"
@if LED_PORT == 1
fn main() { out(1, 200); }
@else
fn main() { out(0, 200); }
@end
```

MacroOct exists as the proving ground for the generic preprocessor engine
specified in [PREP01](../../../specs/PREP01-generic-source-preprocessor.md). A
preprocessor runs *before* the parser, so a dialect that gains one needs no new
parser, type checker or backend — only a lexical grammar that can see its own
directives. This crate is that grammar, and it is the whole of MacroOct's
frontend divergence from Oct.

**Oct itself is not modified**, by any part of PREP01. It is the *reference*
MacroOct is checked against: the slice's acceptance criterion is that a MacroOct
program lowers to IIR **identical** to the hand-expanded Oct program it stands
for, on all eight LANG VM backends. A reference you are free to edit would make
that oracle worthless.

## How it fits in the stack

```text
    MacroOct source
        │
        ▼  macrooct-lexer          ← THIS CRATE
    [Token]  including AT_INCLUDE / AT_IF / AT_ELSE / AT_END / AT_DEFINE
        │
        ▼  source-preprocessor engine + MacroOctDialect
    [Token]  pure Oct — every directive token consumed
        │
        ▼  Oct's parser grammar, type checker and compile_ast, UNCHANGED
    IIRModule  →  all eight backends
```

The composition lives in [`macrooct-iir-compiler`](../macrooct-iir-compiler).

## The grammar: Oct's rules, plus seven names

`macrooct.tokens` is `oct.tokens` rule for rule, in the same first-match-wins
order, with two additions:

| Added | Why |
|---|---|
| `AT_INCLUDE` `AT_DEFINE` `AT_IF` `AT_ELSE` `AT_END` | The directives. `@`-prefixed and spelled `@end` rather than `@endif`, deliberately: if MacroOct adopted C's vocabulary the engine would be proven only against a C-shaped dialect, which tests nothing about genericity. |
| `STR_LIT` | Include paths. Oct has no string *type* — which is exactly why this is safe. A `STR_LIT` can only appear on an `@include` line, and the preprocessor consumes that whole line, so it is structurally incapable of reaching Oct's parser. |

Restating Oct's rules risks silently forking the language, so the crate carries
`macrooct_agrees_with_oct_on_every_non_directive_rule`: it parses **both**
`.tokens` files and asserts every other definition, every keyword and every skip
rule matches, in order. A second test asserts the compiled `_grammar.rs` is
really the compilation of that file, so the pair cannot pass while the lexer
uses something older.

## Usage

```rust
use coding_adventures_macrooct_lexer::tokenize_macrooct;

let tokens = tokenize_macrooct("@if 1\nfn main() { out(1, 42); }\n@end\n");
assert_eq!(tokens[0].value, "@if");
assert_eq!(tokens[0].line, 1);
```

[`try_tokenize_macrooct`] returns a `Result` instead of panicking. Prefer it
anywhere reachable from untrusted input: the text of an `@include`d file is
chosen by the program being compiled, and the engine's contract is that no
input may panic.

## A note on newlines

MacroOct inherits Oct's skip rules, so whitespace — newlines included — emits
no token. A line-oriented preprocessor still works, because every `Token`
carries the `line` it was lexed from and the engine groups its input into
logical lines by that field. The practical rule for a MacroOct author: a
directive must occupy its own line, exactly as in C.

## Regenerating the compiled grammar

```sh
code/scripts/generate-compiled-grammars.sh    # regenerates every language
```

or, for this one grammar alone:

```sh
grammar-tools compile-tokens code/grammars/macrooct/macrooct.tokens \
  -o code/packages/rust/macrooct-lexer/src/_grammar.rs
```

## Spec

[PREP01](../../../specs/PREP01-generic-source-preprocessor.md) §7, slice 1.
