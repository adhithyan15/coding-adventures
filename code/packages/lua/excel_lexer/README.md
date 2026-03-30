# coding-adventures-excel-lexer

A grammar-driven Excel formula tokenizer for Lua.

## Overview

`coding_adventures.excel_lexer` tokenizes Excel formula strings into a flat
stream of typed tokens using the shared `excel.tokens` grammar file and the
grammar-driven `GrammarLexer` from `coding-adventures-lexer`.

Excel formulas are the mini-language embedded in spreadsheet cells that begins
with `=` and describes a computation. Examples:

```
=A1+B2
=SUM(A1:B10)
=IF(A1>0, "positive", "negative")
=Sheet1!A1
=Table1[Amount]*1.1
```

## Token types produced

| Token type          | Example                   | Notes                            |
|---------------------|---------------------------|----------------------------------|
| `EQUALS`            | `=`                       | Formula prefix / comparison      |
| `CELL`              | `A1`, `$B$2`, `AB100`     | A1-style cell reference          |
| `NAME`              | `SUM`, `MyRange`          | Function names and named ranges  |
| `NUMBER`            | `42`, `3.14`, `.5`, `1e3` | Numeric literals                 |
| `STRING`            | `"hello"`, `"say ""hi"""` | Double-quoted strings            |
| `TRUE` / `FALSE`    | `TRUE`, `false`           | Boolean keywords (case-insensitive) |
| `ERROR_CONSTANT`    | `#DIV/0!`, `#VALUE!`      | Excel error values               |
| `REF_PREFIX`        | `Sheet1!`, `'My Sheet'!`  | Cross-sheet / workbook prefix    |
| `STRUCTURED_KEYWORD`| `[#Headers]`, `[#All]`    | Table structured keyword         |
| `STRUCTURED_COLUMN` | `[Amount]`                | Table column reference           |
| `SPACE`             | ` `                       | Intersection operator (preserved)|
| `PLUS` `MINUS` ...  | `+` `-` `*` `/` `^` `&`  | Arithmetic operators             |
| `EQUALS` `NOT_EQUALS` etc. | `=` `<>` `<=`   | Comparison operators             |
| `PERCENT`           | `%`                       | Postfix percentage               |
| `AT`                | `@`                       | Dynamic array spill prefix       |
| `COMMA` `SEMICOLON` | `,` `;`                   | Argument separators              |
| `COLON`             | `:`                       | Range separator                  |
| `LPAREN` `RPAREN`   | `(` `)`                   | Grouping / function call         |
| `LBRACE` `RBRACE`   | `{` `}`                   | Array constant delimiters        |
| `EOF`               |                           | Terminal sentinel                |

## Case insensitivity

Excel has been case-insensitive since its ancestor Multiplan (1982). The
`excel.tokens` grammar declares `@case_insensitive true`. This lexer handles
that by normalizing the source string to lowercase before tokenizing, so all
returned token values are in lowercase form.

## A1 vs R1C1 reference styles

Excel supports two notation styles for cell references:

**A1 style** (default, handled by this lexer):
- Column is a letter (A–XFD), row is a number (1–1,048,576)
- `A1` = relative, `$A$1` = absolute, `$A1` = absolute column only

**R1C1 style** (optional, used in VBA):
- Both row and column are numbers: `R1C1`, `R[-1]C[2]`
- This style is not covered by the current grammar

## Space as intersection operator

Unlike most languages, Excel's space character is the **range intersection
operator** when it appears between two range references:

```excel
=SUM(A1:B10 B5:C15)  ' returns the sum of cells in the intersection
```

Therefore, the `excel.tokens` grammar emits `SPACE` tokens rather than
silently consuming spaces. Only non-space whitespace (tabs, CR, LF) is
silently dropped.

## Usage

```lua
local excel_lexer = require("coding_adventures.excel_lexer")

local tokens = excel_lexer.tokenize("=SUM(A1:B10)")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value)
end
-- EQUALS  =
-- NAME    sum
-- LPAREN  (
-- CELL    a1
-- COLON   :
-- CELL    b10
-- RPAREN  )
-- EOF
```

## Installation

```bash
luarocks make --local coding-adventures-excel-lexer-0.1.0-1.rockspec
```

## Dependencies

- `lua >= 5.4`
- `coding-adventures-state-machine`
- `coding-adventures-directed-graph`
- `coding-adventures-grammar-tools`
- `coding-adventures-lexer`

## Testing

```bash
cd tests && busted . --verbose --pattern=test_
```

## License

MIT
