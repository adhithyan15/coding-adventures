# CodingAdventures::ExcelLexer

A grammar-driven Excel formula tokenizer for Perl.

## Overview

`CodingAdventures::ExcelLexer` tokenizes Excel formula strings into a flat
list of typed token hashrefs using the shared `excel.tokens` grammar file
and the grammar infrastructure from `CodingAdventures::GrammarTools`.

## Synopsis

```perl
use CodingAdventures::ExcelLexer;

my $tokens = CodingAdventures::ExcelLexer->tokenize('=SUM(A1:B10)');
for my $tok (@$tokens) {
    printf "%s  %s\n", $tok->{type}, $tok->{value};
}
# EQUALS  =
# NAME    sum
# LPAREN  (
# CELL    a1
# COLON   :
# CELL    b10
# RPAREN  )
# EOF
```

## Token types

| Type                  | Example                   | Notes                            |
|-----------------------|---------------------------|----------------------------------|
| `EQUALS`              | `=`                       | Formula prefix / equality        |
| `CELL`                | `a1`, `$b$2`              | A1-style cell reference          |
| `NAME`                | `sum`, `myrange`          | Function names and named ranges  |
| `NUMBER`              | `42`, `3.14`, `.5`        | Numeric literal                  |
| `STRING`              | `"hello"`                 | Double-quoted string             |
| `TRUE` / `FALSE`      | `true` / `false`          | Boolean keywords (normalized)    |
| `ERROR_CONSTANT`      | `#div/0!`, `#value!`      | Excel error value                |
| `REF_PREFIX`          | `sheet1!`                 | Cross-sheet prefix               |
| `STRUCTURED_KEYWORD`  | `[#headers]`              | Table structured keyword         |
| `STRUCTURED_COLUMN`   | `[amount]`                | Table column reference           |
| `SPACE`               | ` `                       | Intersection operator (kept)     |
| `PLUS` `MINUS` etc.   | `+` `-` `*` `/` `^` `&`  | Arithmetic                       |
| `EQUALS` `NOT_EQUALS` | `=` `<>`                  | Comparison                       |
| `PERCENT`             | `%`                       | Postfix percent                  |
| `AT`                  | `@`                       | Dynamic array spill              |
| `COMMA` `SEMICOLON`   | `,` `;`                   | Argument separators              |
| `COLON`               | `:`                       | Range separator                  |
| `LPAREN` `RPAREN`     | `(` `)`                   | Grouping                         |
| `LBRACE` `RBRACE`     | `{` `}`                   | Array constant                   |
| `EOF`                 |                           | Terminal sentinel                |

## Case insensitivity

The source string is lowercased before tokenizing.  All returned token
values are lowercase.

## SPACE as intersection operator

Excel's space character is the range-intersection operator:

```excel
=SUM(A1:B10 B5:C15)  # sum of intersection
```

Therefore SPACE tokens are emitted rather than silently consumed.  Only
tabs (`\t`), carriage returns (`\r`), and newlines (`\n`) are silently
skipped.

## A1 reference style

Cell references use the A1 style: column letter(s) followed by row number.
Dollar signs (`$`) make an axis absolute: `$A$1` (both absolute), `$A1`
(column absolute, row relative).

## Installation

```bash
cpanm --notest --quiet .
```

## Dependencies

- `CodingAdventures::GrammarTools`
- `CodingAdventures::Lexer`
- `File::Basename`
- `File::Spec`

## Testing

```bash
prove -l -v t/
```

## License

MIT
