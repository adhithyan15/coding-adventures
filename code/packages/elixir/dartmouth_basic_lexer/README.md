# dartmouth_basic_lexer

Elixir lexer for the 1964 Dartmouth BASIC language. Converts raw BASIC source
text into a flat list of typed tokens, ready for the `dartmouth_basic_parser`.

## Background

In 1964, John Kemeny and Thomas Kurtz at Dartmouth College created BASIC
(Beginner's All-purpose Symbolic Instruction Code) to make computing accessible
to every student, not just computer science majors. It ran on a GE-225 mainframe
connected to teletypes that could only produce uppercase characters.

This lexer targets that original 1964 dialect: 20 keywords, 11 built-in
functions, line-numbered programs, and uppercase-only identifiers.

## Where This Fits

```
BASIC source text
      │
      ▼  tokenize/1
┌─────────────────────────────────┐
│   dartmouth_basic_lexer         │  ← this package
│   dartmouth_basic.tokens        │
└─────────────────────────────────┘
      │
      ▼  [{type, value, line, column}, ...]
┌─────────────────────────────────┐
│   dartmouth_basic_parser        │
└─────────────────────────────────┘
      │
      ▼  AST
┌─────────────────────────────────┐
│   dartmouth_basic_compiler      │
└─────────────────────────────────┘
```

## Usage

```elixir
{:ok, tokens} = CodingAdventures.DartmouthBasicLexer.tokenize("10 LET X = 5\n")

Enum.each(tokens, fn tok ->
  IO.puts("#{tok.type}: #{inspect(tok.value)}")
end)
# LINE_NUM: "10"
# KEYWORD: "LET"
# NAME: "X"
# EQ: "="
# NUMBER: "5"
# NEWLINE: "\n"
# EOF: ""
```

### Inspect the grammar

```elixir
grammar = CodingAdventures.DartmouthBasicLexer.create_lexer()
names = Enum.map(grammar.definitions, & &1.name)
# => ["LE", "GE", "NE", "LINE_NUM", "NUMBER", ...]
```

## Token Types

| Type        | Example           | Notes                                         |
|-------------|-------------------|-----------------------------------------------|
| `LINE_NUM`  | `"10"`, `"999"`   | First number on each physical line            |
| `NUMBER`    | `"3.14"`, `"1E3"` | Numeric literal in an expression              |
| `STRING`    | `"\"HELLO\""`     | Double-quoted; includes surrounding quotes    |
| `KEYWORD`   | `"PRINT"`, `"IF"` | Always uppercase                              |
| `BUILTIN_FN`| `"SIN"`, `"RND"`  | One of 11 built-in math functions             |
| `USER_FN`   | `"FNA"`, `"FNZ"`  | FN + one letter; defined with DEF             |
| `NAME`      | `"X"`, `"A1"`     | Variable: one letter + optional digit         |
| `PLUS`      | `"+"`             |                                               |
| `MINUS`     | `"-"`             |                                               |
| `STAR`      | `"*"`             | Multiplication                                |
| `SLASH`     | `"/"`             | Division                                      |
| `CARET`     | `"^"`             | Exponentiation                                |
| `EQ`        | `"="`             | Assignment (LET) and equality (IF)            |
| `LT`        | `"<"`             |                                               |
| `GT`        | `">"`             |                                               |
| `LE`        | `"<="`            | Less-than-or-equal                            |
| `GE`        | `">="`            | Greater-than-or-equal                         |
| `NE`        | `"<>"`            | Not-equal                                     |
| `LPAREN`    | `"("`             |                                               |
| `RPAREN`    | `")"`             |                                               |
| `COMMA`     | `","`             | PRINT: advance to next print zone             |
| `SEMICOLON` | `";"`             | PRINT: no space between items                 |
| `NEWLINE`   | `"\n"`            | Statement terminator — significant!           |
| `EOF`       | `""`              | Always the last token                         |

## Key Design Decisions

### Case Insensitivity

The grammar uses `@case_insensitive true`, which normalises the entire source
to uppercase before tokenising. This is historically accurate: 1964 teletypes
were uppercase-only. It means `print`, `Print`, and `PRINT` all produce
`KEYWORD("PRINT")`.

### LINE_NUM vs NUMBER

A bare integer serves two roles in BASIC:

```basic
10 LET X = 42     ← "10" is a line label; "42" is a value
20 GOTO 10        ← "10" is a jump target
```

Both look identical to the grammar. A post-tokenize hook (`relabel_line_numbers`)
walks the token list and relabels the first `NUMBER` token on each physical line
as `LINE_NUM`. Numbers elsewhere stay `NUMBER`.

### REM Comments

`REM` introduces a comment that runs to end of line. A second post-tokenize hook
(`suppress_rem_content`) drops all tokens between `KEYWORD("REM")` and the next
`NEWLINE`. The `REM` keyword and `NEWLINE` are preserved.

```basic
10 REM THIS IS IGNORED
```
→ `LINE_NUM("10")`, `KEYWORD("REM")`, `NEWLINE`

## Dependencies

- `coding_adventures_grammar_tools` — parses `dartmouth_basic.tokens`
- `coding_adventures_lexer` — runs the grammar against source text
- `coding_adventures_directed_graph` — transitive dep of grammar_tools

## Grammar File

The grammar lives at `code/grammars/dartmouth_basic.tokens` — a shared location
so all language implementations (Elixir, Python, Ruby, TypeScript, etc.) of
`dartmouth_basic_lexer` can reference the same canonical grammar.
