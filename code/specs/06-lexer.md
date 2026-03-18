# 06 — Lexer

## Overview

The lexer (also called tokenizer or scanner) is the first stage of any compiler or interpreter. It takes raw source code text and breaks it into a sequence of tokens — the smallest meaningful units of the language.

The lexer does not understand what the tokens *mean* or how they relate to each other. It only identifies what they *are*: a number, a name, an operator, a keyword, etc.

The long-term goal is to make this lexer grammar-driven: provide a token grammar definition, and the lexer generates itself — like a simplified version of Lex/Flex or ANTLR's lexer.

This is Layer 6 of the computing stack. It has no package dependencies (standalone text processing).

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → [YOU ARE HERE] → Parser → Compiler → VM
```

**Input from:** Raw source code (a string).
**Output to:** Parser (consumes the token stream to build an AST).

## Concepts

### What is a token?

A token is a categorized chunk of text:

```python
"x = 1 + 2"

→ Token(type=NAME,    value="x",  line=1, column=1)
  Token(type=EQUALS,  value="=",  line=1, column=3)
  Token(type=NUMBER,  value="1",  line=1, column=5)
  Token(type=PLUS,    value="+",  line=1, column=7)
  Token(type=NUMBER,  value="2",  line=1, column=9)
  Token(type=EOF,     value="",   line=1, column=10)
```

### Token types for the MVP language

```
NAME       [a-zA-Z_][a-zA-Z0-9_]*    Variables and identifiers
NUMBER     [0-9]+                      Integer literals
STRING     "..."                       String literals
PLUS       +
MINUS      -
STAR       *
SLASH      /
EQUALS     =
LPAREN     (
RPAREN     )
COLON      :
NEWLINE    \n
EOF        end of input
```

### Keywords vs. Names

Keywords (if, else, while, def, return) are lexed as NAME tokens first, then checked against a keyword list. This is simpler than giving each keyword its own regex rule.

### Whitespace and comments

Whitespace (spaces, tabs) is consumed but not emitted as tokens (except NEWLINE which has syntactic meaning in Python-like languages). Comments (# ...) are consumed and discarded.

### Error recovery

When the lexer encounters an unexpected character, it should:
1. Record an error with line/column information
2. Skip the character
3. Continue lexing (don't stop at the first error)

### Grammar-driven lexing (future)

The ultimate goal is to define token rules in a grammar file:

```
NAME    = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER  = /[0-9]+(\.[0-9]+)?/
STRING  = /"[^"]*"/
PLUS    = "+"
MINUS   = "-"
SKIP    = /\s+/
COMMENT = /#.*/
```

And have the lexer generate itself from this grammar.

## Public API

```python
@dataclass
class Token:
    type: TokenType
    value: str
    line: int
    column: int

class TokenType(Enum):
    NAME = "NAME"
    NUMBER = "NUMBER"
    STRING = "STRING"
    PLUS = "PLUS"
    MINUS = "MINUS"
    STAR = "STAR"
    SLASH = "SLASH"
    EQUALS = "EQUALS"
    LPAREN = "LPAREN"
    RPAREN = "RPAREN"
    COLON = "COLON"
    NEWLINE = "NEWLINE"
    EOF = "EOF"
    # Keywords (detected after lexing NAME)
    IF = "IF"
    ELSE = "ELSE"
    WHILE = "WHILE"
    DEF = "DEF"
    RETURN = "RETURN"
    PRINT = "PRINT"

class Lexer:
    def __init__(self, source: str) -> None: ...

    def tokenize(self) -> list[Token]: ...
        # Tokenize the entire source, return all tokens

    def next_token(self) -> Token: ...
        # Return the next token (for streaming/lazy usage)

    @property
    def errors(self) -> list[LexError]: ...

@dataclass
class LexError:
    message: str
    line: int
    column: int
    character: str
```

## Data Flow

```
Input:  Source code (str)
Output: List of Token objects + list of errors
```

## Test Strategy

- Tokenize individual token types (each in isolation)
- Tokenize multi-token expressions: `1 + 2`, `x = 42`, `foo(1, 2)`
- Verify line and column tracking across multiple lines
- Verify keyword detection: `if` is IF, `iff` is NAME
- Verify string literal handling including edge cases
- Verify error recovery: invalid characters produce errors but lexing continues
- Verify EOF is always the last token
- Verify whitespace and comments are skipped

## Future Extensions

- **Grammar-driven lexer generator**: Define tokens in a grammar file, generate the lexer
- **Indentation tracking**: Emit INDENT/DEDENT tokens (needed for Python-like syntax)
- **Unicode support**: Handle non-ASCII identifiers
- **Interpolated strings**: Handle `f"hello {name}"`
