# coding-adventures-algol-lexer

A grammar-driven lexer for **ALGOL 60** — the 1960 language that invented block
structure, lexical scoping, recursion, and the formal grammar specification
(BNF) used by every programming language since.

## What Is ALGOL 60?

ALGOL (ALGOrithmic Language, 1960) was designed by an international committee
including John Backus, Peter Naur, and Edsger Dijkstra. Its contributions to
computer science are staggering:

- **BNF notation**: The grammar was the first ever written in Backus-Naur Form,
  the notation still taught in every compiler course.
- **Block structure**: `begin`...`end` creates a lexical scope. Variables
  declared inside a block are invisible outside it.
- **Recursion**: ALGOL was the first widely-used language to support recursive
  procedures, requiring the invention of the call stack.
- **Free-format syntax**: Unlike FORTRAN and COBOL (tied to 80-column punch
  cards), ALGOL source code can be formatted freely across lines.
- **`:=` for assignment**: Separating assignment from equality (`=`) prevents
  the C bug of writing `=` when you mean `==`.

ALGOL 60's family tree includes Pascal, C, Simula (first OOP language), and
through them essentially every modern language: Java, C#, Python, Ruby, Rust,
Go, Swift, Kotlin.

## How It Fits the Stack

```
algol.tokens (grammar definition)
      │
      ▼
grammar_tools.parse_token_grammar()
      │
      ▼
GrammarLexer  ←── algol-lexer (this package, thin wrapper)
      │
      ▼
list[Token]   ──► algol-parser
```

This package is a thin wrapper. The real work is done by:

- **`coding-adventures-grammar-tools`**: Parses `algol.tokens` into a
  `TokenGrammar` data structure.
- **`coding-adventures-lexer`**: The `GrammarLexer` engine that runs the
  token grammar against source text.

## Usage

```python
from algol_lexer import tokenize_algol, create_algol_lexer

# Simple tokenization
tokens = tokenize_algol("begin integer x; x := 42 end")
for token in tokens:
    print(token)
# Token(BEGIN, 'begin')
# Token(INTEGER, 'integer')
# Token(IDENT, 'x')
# Token(SEMICOLON, ';')
# Token(IDENT, 'x')
# Token(ASSIGN, ':=')
# Token(INTEGER_LIT, '42')
# Token(END, 'end')
# Token(EOF, '')

# Factory function (for more control)
lexer = create_algol_lexer("x := 3.14")
tokens = lexer.tokenize()
```

## Token Types

| Category | Tokens |
|----------|--------|
| Block | `BEGIN`, `END` |
| Control | `IF`, `THEN`, `ELSE`, `FOR`, `DO`, `STEP`, `UNTIL`, `WHILE`, `GOTO` |
| Declarations | `SWITCH`, `PROCEDURE`, `OWN`, `ARRAY`, `LABEL`, `VALUE` |
| Types | `INTEGER`, `REAL`, `BOOLEAN`, `STRING` |
| Boolean literals | `TRUE`, `FALSE` |
| Boolean operators | `NOT`, `AND`, `OR`, `IMPL`, `EQV` |
| Arithmetic | `DIV`, `MOD` |
| Values | `INTEGER_LIT`, `REAL_LIT`, `STRING_LIT`, `IDENT` |
| Assignment | `ASSIGN` (`:=`) |
| Exponentiation | `POWER` (`**`), `CARET` (`^`, `↑`) |
| Relational | `EQ` (`=`), `NEQ` (`!=`, `<>`, `≠`), `LT` (`<`), `GT` (`>`), `LEQ` (`<=`, `≤`), `GEQ` (`>=`, `≥`) |
| Arithmetic | `PLUS` (`+`), `MINUS` (`-`), `STAR` (`*`), `SLASH` (`/`) |
| Delimiters | `LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `SEMICOLON`, `COMMA`, `COLON` |

## ALGOL 60 Language Notes

### Assignment vs. Equality

ALGOL uses `:=` for assignment and `=` for equality. This is one of ALGOL's
most influential design decisions — it prevents the C bug:

```algol
x := 42      (* assignment: x gets the value 42 *)
if x = 42 then ...  (* equality test: is x equal to 42? *)
```

### Comments

ALGOL comments use the case-insensitive keyword `comment` followed by text up
to the next `;`:

```algol
comment this is a comment;
COMMENT this is also a comment;
x := 1; comment set x to one;
```

The comment (including its terminating semicolon) is consumed silently.
Identifiers that merely start with those letters, such as `commentary`, remain
identifiers.

### Keywords Are Case-Insensitive

`BEGIN`, `Begin`, and `begin` all produce the same `BEGIN` token. Keyword
matching is done after normalizing to lowercase.

### Publication Symbols

ALGOL 60 publication symbols normalize to the same token values as the ASCII or
word spellings used by the parser and compiler: `≤` to `<=`, `≥` to `>=`, `≠`
to `!=`, `↑` to `^`, `×` to `*`, `÷` to `/`, and `¬`, `∧`, `∨`, `⊃`, `≡`
to `not`, `and`, `or`, `impl`, `eqv`.

### String Literals

String literals may use single or double quotes. The delimiter cannot appear
inside the literal because the grammar intentionally does not define escape
sequences.

### Real Literals

```algol
3.14      (* decimal *)
1.5E3     (* 1500.0 *)
1.5E-3    (* 0.0015 *)
100E2     (* 10000.0 *)
```

## Development

```bash
pip install -e ".[dev]"
pytest
ruff check src/
```
