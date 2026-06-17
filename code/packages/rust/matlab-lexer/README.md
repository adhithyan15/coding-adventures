# MATLAB Lexer

A grammar-driven lexer (tokenizer) for the
[MATLAB](https://en.wikipedia.org/wiki/MATLAB) language — the matrix-laboratory
language (Cleve Moler, ~1979) whose only data type is the array. This is the
lexical layer (item **MA-3b**) of the MATLAB frontend on
[`array-runtime`](../array-runtime); see
[`MA01-matlab-language.md`](../../../specs/MA01-matlab-language.md).

## What it does

Tokenizes MATLAB source. Like every language frontend in this repo it doesn't
hand-write the bulk of tokenization: it loads the compiled `matlab.tokens`
grammar and feeds it to the generic `GrammarLexer` from the `lexer` crate. What
it *does* hand-write are the two context-sensitive rules a regex grammar cannot
express.

### The hard one: `'` is transpose **and** a string delimiter

A single quote is the (conjugate) transpose operator after a value, but the
char-array delimiter otherwise:

```matlab
A'              % transpose
'abc'           % a char array
A' * B'         % two transposes — NOT the string ' * B'
[1 'a']         % a string 'a' (the space before ' resets the context)
x = 'it''s'     % the char array  it's   ('' is an escaped quote)
```

MATLAB's rule: `'` is **transpose when a value-terminator immediately precedes
it** (an identifier, number, string, or a closing `)`/`]`/`}` or postfix `'`,
with no intervening whitespace) and **starts a string otherwise**. A regex
grammar can't see "the previous token", so a **pre-tokenize hook** resolves it
before the grammar runs: transpose quotes are left as bare `'` (lexed as
`TRANSPOSE`), and char-array literals are rewritten to `` `N` `` backtick
placeholders (lexed as `CHARARRAY` → `STRING`, then restored by a post-hook).
Backtick is not a MATLAB token, so it only ever appears as that placeholder.

### The inverted newline rule

Like S/R, a newline ends a statement and is insignificant inside `( )`. But
**unlike** S/R, a newline inside `[ ]`/`{ }` is significant — it separates matrix
and cell rows (`[1 2; 3 4]` and `[1 2\n3 4]` are the same matrix). So the
post-hook tracks only **parenthesis** depth and keeps bracket/brace-interior
newlines.

Also handled: the element-wise `.`-prefixed operators (`.* ./ .\ .^ .'`, kept
distinct from `*`/`/` and from a trailing decimal so `3.*4` is `3 .* 4`), `%`
line comments and `%{ %}` block comments, and `...` line continuations.

## Usage

```rust
use coding_adventures_matlab_lexer::tokenize_matlab;

let toks = tokenize_matlab("A'\n");
assert_eq!(toks[1].effective_type_name(), "TRANSPOSE"); // A, then transpose
```

Use `try_tokenize_matlab` for a `Result` instead of a panic.

## Regenerating the embedded grammar

`src/_grammar.rs` is generated from `code/grammars/matlab.tokens` with the
grammar-tools CLI — never hand-edit it:

```sh
grammar-tools compile-tokens code/grammars/matlab.tokens \
  -o code/packages/rust/matlab-lexer/src/_grammar.rs
```

## Testing

```sh
cargo test -p coding-adventures-matlab-lexer
```
