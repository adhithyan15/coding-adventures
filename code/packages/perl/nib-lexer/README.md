# CodingAdventures::StarlarkLexer (Perl)

A Starlark tokenizer that tokenizes Starlark source text into a flat list of typed token hashrefs. It reads the shared `starlark.tokens` grammar file and uses `CodingAdventures::GrammarTools` to compile the token definitions into Perl regexes.

## What is Starlark?

Starlark is a deterministic subset of Python designed for use as a configuration language (used in Bazel BUILD files). It has significant indentation (like Python) and a restricted feature set.

## What it does

Given the input `def foo(x):`, the tokenizer produces:

| type   | value | line | col |
|--------|-------|------|-----|
| DEF    | `def` | 1    | 1   |
| NAME   | `foo` | 1    | 5   |
| LPAREN | `(`   | 1    | 8   |
| NAME   | `x`   | 1    | 9   |
| RPAREN | `)`   | 1    | 10  |
| COLON  | `:`   | 1    | 11  |
| NEWLINE| `\n`  | 1    | 12  |
| EOF    |       | 2    | 1   |

Whitespace and `#` comments are silently consumed. Indented blocks produce `INDENT`, `DEDENT`, and `NEWLINE` tokens automatically.

## Token types

Keywords: AND, BREAK, CONTINUE, DEF, ELIF, ELSE, FOR, IF, IN, LAMBDA, LOAD, NOT, OR, PASS, RETURN, TRUE, FALSE, NONE

Literals: NAME, INT (decimal/hex/octal), FLOAT, STRING (all variants)

Three-char operators: DOUBLE_STAR_EQUALS, LEFT_SHIFT_EQUALS, RIGHT_SHIFT_EQUALS, FLOOR_DIV_EQUALS

Two-char operators: DOUBLE_STAR, FLOOR_DIV, LEFT_SHIFT, RIGHT_SHIFT, EQUALS_EQUALS, NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS, PLUS_EQUALS, MINUS_EQUALS, STAR_EQUALS, SLASH_EQUALS, PERCENT_EQUALS, AMP_EQUALS, PIPE_EQUALS, CARET_EQUALS

Single-char operators: PLUS, MINUS, STAR, SLASH, PERCENT, EQUALS, LESS_THAN, GREATER_THAN, AMP, PIPE, CARET, TILDE

Delimiters: LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE, COMMA, COLON, SEMICOLON, DOT

Indentation: INDENT, DEDENT, NEWLINE

## Usage

```perl
use CodingAdventures::StarlarkLexer;

my $tokens = CodingAdventures::StarlarkLexer->tokenize("def foo(x):\n    return x\n");
for my $tok (@$tokens) {
    printf "%s  %s\n", $tok->{type}, $tok->{value};
}
```

## How it fits in the stack

```
starlark.tokens  (code/grammars/)
    ↓  parsed by GrammarTools
TokenGrammar
    ↓  compiled to Perl regexes by
StarlarkLexer  ← you are here
    ↓  feeds
starlark_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `starlark.tokens`
- `CodingAdventures::Lexer` — provides shared Lexer infrastructure

## Running tests

```bash
prove -l -v t/
```
