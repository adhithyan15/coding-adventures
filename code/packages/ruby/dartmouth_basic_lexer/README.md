# coding_adventures_dartmouth_basic_lexer

A grammar-driven lexer for the 1964 Dartmouth BASIC language — the original BASIC
as implemented on the GE-225 mainframe at Dartmouth College by John Kemeny and
Thomas Kurtz.

## What Is Dartmouth BASIC?

Dartmouth BASIC (1964) was the first BASIC language. Its design goals were radical
for the era:

- **Accessible to non-scientists**: designed for liberal arts students, not engineers
- **Line-numbered**: every statement lives on a numbered line (`10 LET X = 5`)
- **Interactive**: results print immediately on a teletype as the program runs
- **Forgiving**: every variable is pre-initialized to 0; no declarations needed

Every home computer BASIC of the 1970s–1980s — Microsoft BASIC, Applesoft BASIC,
GW-BASIC, Commodore BASIC, ZX BASIC — descends from Kemeny and Kurtz's original design.

## What This Package Does

This package sits at Layer 1 of the Dartmouth BASIC processing pipeline:

```
BASIC source text
      │
      ▼
┌─────────────────────────────────┐
│   dartmouth_basic_lexer         │  ← this package
│   dartmouth_basic.tokens        │
└─────────────────────────────────┘
      │
      ▼  Array of Token objects
┌─────────────────────────────────┐
│   dartmouth_basic_parser        │  (future)
└─────────────────────────────────┘
```

The lexer breaks raw source text into a flat array of tokens. It knows nothing
about syntax, semantics, or program structure — those are the parser's concern.

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_dartmouth_basic_lexer", path: "../dartmouth_basic_lexer"
gem "coding_adventures_grammar_tools",         path: "../grammar_tools"
gem "coding_adventures_lexer",                 path: "../lexer"
```

## Usage

```ruby
require "coding_adventures_dartmouth_basic_lexer"

source = <<~BASIC
  10 LET X = 1
  20 FOR I = 1 TO 10
  30 LET X = X * I
  40 NEXT I
  50 PRINT X
  60 END
BASIC

tokens = CodingAdventures::DartmouthBasicLexer.tokenize(source)

tokens.each do |token|
  puts "#{token.type.ljust(12)} #{token.value.inspect}  (#{token.line}:#{token.column})"
end
```

Output:

```
LINE_NUM     "10"   (1:1)
KEYWORD      "LET"  (1:4)
NAME         "X"    (1:8)
EQ           "="    (1:10)
NUMBER       "1"    (1:12)
NEWLINE      "\\n"  (1:13)
LINE_NUM     "20"   (2:1)
KEYWORD      "FOR"  (2:4)
...
EOF          ""
```

## Token Types

| Type | Example value | Notes |
|------|---------------|-------|
| `LINE_NUM` | `"10"`, `"9999"` | First number on each line — line label |
| `NUMBER` | `"42"`, `"3.14"`, `"1.5E3"` | Numeric literal in an expression |
| `STRING` | `"HELLO WORLD"` | String content — surrounding quotes stripped |
| `KEYWORD` | `"PRINT"`, `"LET"`, `"IF"` | Reserved word (always uppercase) |
| `BUILTIN_FN` | `"SIN"`, `"LOG"`, `"RND"` | One of the 11 built-in math functions |
| `USER_FN` | `"FNA"`, `"FNZ"` | User-defined function: `FN` + one letter |
| `NAME` | `"X"`, `"A1"`, `"B9"` | Variable name: one letter + optional digit |
| `PLUS` | `"+"` | Addition |
| `MINUS` | `"-"` | Subtraction or unary negation |
| `STAR` | `"*"` | Multiplication |
| `SLASH` | `"/"` | Division |
| `CARET` | `"^"` | Exponentiation (right-associative) |
| `EQ` | `"="` | Assignment (LET) or equality (IF) — parser disambiguates |
| `LT` | `"<"` | Less than |
| `GT` | `">"` | Greater than |
| `LE` | `"<="` | Less than or equal |
| `GE` | `">="` | Greater than or equal |
| `NE` | `"<>"` | Not equal |
| `LPAREN` | `"("` | Open parenthesis |
| `RPAREN` | `")"` | Close parenthesis |
| `COMMA` | `","` | Print zone separator |
| `SEMICOLON` | `";"` | Tight print separator (no space between items) |
| `NEWLINE` | `"\\n"` | Statement terminator — significant in BASIC |
| `EOF` | `""` | Always the last token |
| `UNKNOWN` | `"@"` | Unrecognised character (error recovery) |

## Case Insensitivity

The 1964 teletypes only had uppercase characters. This lexer normalises all input
to uppercase before matching, so `print`, `PRINT`, and `Print` all produce the
same `KEYWORD("PRINT")` token:

```ruby
CodingAdventures::DartmouthBasicLexer.tokenize("10 print x\n")
# Same result as:
CodingAdventures::DartmouthBasicLexer.tokenize("10 PRINT X\n")
```

## LINE_NUM Disambiguation

In Dartmouth BASIC, the integer `10` at the start of a line means "this is line 10"
(a label), while `10` in the middle of a line means "the number ten" (a value).
Both look identical to the grammar's regex engine.

This package solves the ambiguity with a **post-tokenize hook** that walks the
token list and relabels the first `NUMBER` token on each line as `LINE_NUM`:

```
"30 GOTO 10\n"
→ LINE_NUM("30")  KEYWORD("GOTO")  NUMBER("10")  NEWLINE
     ^-- relabelled                  ^-- stays NUMBER (branch target)
```

## REM Comment Suppression

`REM` introduces a comment that runs to the end of the physical line. A second
post-tokenize hook suppresses all tokens between the `REM` keyword and the next
`NEWLINE`:

```
"10 REM THIS IS A COMMENT\n"
→ LINE_NUM("10")  KEYWORD("REM")  NEWLINE
                                  (comment text removed)
```

## Grammar File

The grammar is defined in `code/grammars/dartmouth_basic.tokens`. This file is
shared across all language implementations (Ruby, Python, TypeScript, Go, Rust)
so that lexical rules only need to be maintained in one place.

## Dependencies

| Package | Role |
|---------|------|
| `coding_adventures_grammar_tools` | Parses `dartmouth_basic.tokens` into a `TokenGrammar` struct |
| `coding_adventures_lexer` | Runs the `TokenGrammar` against source text, produces `[Token]` |

## Running Tests

```bash
bundle install
bundle exec rake test
```
