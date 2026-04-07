# CodingAdventures::AlgolLexer (Perl)

A grammar-driven ALGOL 60 tokenizer. Reads the shared `algol.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes ALGOL 60 source into a flat list of typed tokens.

## What it does

Given `begin integer x; x := 42 end`, produces:

| type        | value     | line | col |
|-------------|-----------|------|-----|
| BEGIN       | `begin`   | 1    | 1   |
| INTEGER     | `integer` | 1    | 7   |
| IDENT       | `x`       | 1    | 15  |
| SEMICOLON   | `;`       | 1    | 16  |
| IDENT       | `x`       | 1    | 18  |
| ASSIGN      | `:=`      | 1    | 20  |
| INTEGER_LIT | `42`      | 1    | 23  |
| END         | `end`     | 1    | 26  |
| EOF         |           | 1    | 29  |

Whitespace and comments (`comment ... ;`) are consumed silently. The last token is always `EOF`.

## Token types

### Value tokens

| Token       | Example              | Notes                                |
|-------------|----------------------|--------------------------------------|
| INTEGER_LIT | `0`, `42`, `1000`    | One or more decimal digits           |
| REAL_LIT    | `3.14`, `1.5E3`      | Decimal or exponent; matched first   |
| STRING_LIT  | `'hello'`            | Single-quoted; no escape sequences   |
| IDENT       | `x`, `sum`, `A1`     | Letter followed by letters or digits |

### Keywords (reclassified from IDENT, case-insensitive)

| Token     | Keyword     | Category        |
|-----------|-------------|-----------------|
| BEGIN     | `begin`     | Block structure |
| END       | `end`       | Block structure |
| IF        | `if`        | Control flow    |
| THEN      | `then`      | Control flow    |
| ELSE      | `else`      | Control flow    |
| FOR       | `for`       | Control flow    |
| DO        | `do`        | Control flow    |
| STEP      | `step`      | For loop        |
| UNTIL     | `until`     | For loop        |
| WHILE     | `while`     | For loop        |
| GOTO      | `goto`      | Control flow    |
| SWITCH    | `switch`    | Declaration     |
| PROCEDURE | `procedure` | Declaration     |
| ARRAY     | `array`     | Declaration     |
| VALUE     | `value`     | Declaration     |
| INTEGER   | `integer`   | Type            |
| REAL      | `real`      | Type            |
| BOOLEAN   | `boolean`   | Type            |
| STRING    | `string`    | Type            |
| TRUE      | `true`      | Boolean literal |
| FALSE     | `false`     | Boolean literal |
| NOT       | `not`       | Boolean op      |
| AND       | `and`       | Boolean op      |
| OR        | `or`        | Boolean op      |
| IMPL      | `impl`      | Boolean op      |
| EQV       | `eqv`       | Boolean op      |
| DIV       | `div`       | Arithmetic op   |
| MOD       | `mod`       | Arithmetic op   |

### Operators

| Token  | Symbol | Notes                           |
|--------|--------|---------------------------------|
| ASSIGN | `:=`   | Assignment (matched before `:`) |
| POWER  | `**`   | Exponentiation (before `*`)     |
| LEQ    | `<=`   | Less-or-equal (before `<`)      |
| GEQ    | `>=`   | Greater-or-equal (before `>`)   |
| NEQ    | `!=`   | Not-equal                       |
| PLUS   | `+`    |                                 |
| MINUS  | `-`    |                                 |
| STAR   | `*`    |                                 |
| SLASH  | `/`    |                                 |
| CARET  | `^`    | Alternative exponentiation      |
| EQ     | `=`    | Equality test (not assignment)  |
| LT     | `<`    |                                 |
| GT     | `>`    |                                 |

### Delimiters

| Token     | Symbol |
|-----------|--------|
| LPAREN    | `(`    |
| RPAREN    | `)`    |
| LBRACKET  | `[`    |
| RBRACKET  | `]`    |
| SEMICOLON | `;`    |
| COMMA     | `,`    |
| COLON     | `:`    |

## Usage

```perl
use CodingAdventures::AlgolLexer;

my $tokens = CodingAdventures::AlgolLexer->tokenize('begin integer x; x := 42 end');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## Keywords are case-insensitive

```perl
# All three produce type "BEGIN":
CodingAdventures::AlgolLexer->tokenize('begin');   # type => 'BEGIN'
CodingAdventures::AlgolLexer->tokenize('BEGIN');   # type => 'BEGIN'
CodingAdventures::AlgolLexer->tokenize('Begin');   # type => 'BEGIN'

# Partial matches are IDENT, not keywords:
CodingAdventures::AlgolLexer->tokenize('beginning');  # type => 'IDENT'
```

## Comments

```perl
# "comment ... ;" is consumed silently:
my $toks = CodingAdventures::AlgolLexer->tokenize('comment init;  x := 1');
# toks: IDENT(x), ASSIGN(:=), INTEGER_LIT(1), EOF
```

## How it fits in the stack

```
algol.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::AlgolLexer  ← you are here
    ↓  feeds
CodingAdventures::AlgolParser
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `algol.tokens`

## Running tests

```bash
prove -l -v t/
```
