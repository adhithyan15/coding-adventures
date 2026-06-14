# Grammar Tools

**Declarative grammar definitions for lexers and parsers** — define language syntax in plain text files, validate them, and use them to drive code generation.

## What this package does

Grammar Tools provides two complementary file formats for describing programming language syntax:

1. **`.tokens` files** — define the *lexical* grammar: what tokens (words) exist in the language. Each line maps a token name to either a regex pattern or a literal string. There is also a `keywords:` section for reserved words.

2. **`.grammar` files** — define the *syntactic* grammar in EBNF notation: how tokens combine to form valid programs. Rules reference tokens by their UPPERCASE names from the `.tokens` file and other rules by lowercase names.

The package parses both formats, validates them individually, and cross-validates them together to ensure every token reference in the grammar actually exists in the token definitions, and warns about tokens that are defined but never used.

## Why two files?

Separating lexical and syntactic grammars mirrors how real compilers work. The lexer (scanner) handles character-level pattern matching — recognizing that `123` is a number and `while` is a keyword. The parser handles structure — recognizing that `while x > 0:` is a loop header. By defining both in declarative files rather than hand-coding them, we can:

- **Validate grammars before generating code** — catch typos and missing rules early
- **Generate lexers and parsers for multiple target languages** — same `.tokens` and `.grammar` files can produce Python, Ruby, and TypeScript implementations
- **Reason about the grammar** — compute FIRST/FOLLOW sets, detect ambiguities, identify left recursion

## Background: EBNF and grammar notation

EBNF (Extended Backus-Naur Form) is a notation for describing context-free grammars. It extends BNF with three conveniences:

| Notation    | Meaning                  | Example                          |
|-------------|--------------------------|----------------------------------|
| `{ x }`     | Zero or more repetitions | `{ statement }` — any number of statements |
| `[ x ]`     | Optional (zero or one)   | `[ ELSE block ]` — optional else clause |
| `( x \| y )` | Grouping with choice     | `( PLUS \| MINUS )` — either operator |

This is the same family of notation used by ANTLR, Yacc/Bison, and language specifications (Python, SQL, etc.).

## The chicken-and-egg problem

There is an amusing bootstrapping problem here: we need a parser to read `.grammar` files, but `.grammar` files define parsers. We solve this the same way every grammar tool does — the grammar file parser is hand-written using recursive descent. It is a small, self-contained parser that understands EBNF notation directly. Once we can read `.grammar` files, we can use them to *generate* parsers for arbitrary languages.

This is exactly how tools like ANTLR and Yacc work: they are hand-written parsers that read grammar descriptions and produce parsers as output.

## Comparison to existing tools

| Tool         | Lexer grammar | Parser grammar | This package     |
|--------------|---------------|----------------|------------------|
| Lex/Flex     | `.l` files    | —              | `.tokens` files  |
| Yacc/Bison   | —             | `.y` files     | `.grammar` files |
| ANTLR        | Combined `.g4`| Combined `.g4` | Separate files   |

We keep the files separate for clarity and because different target languages may share a parser grammar but need different lexer patterns (e.g., string escaping rules differ).

## File format: `.tokens`

```
# Comments start with #
NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
PLUS   = "+"
NUMBER = /[0-9]+/

keywords:
  if
  else
  while
```

- `TOKEN_NAME = /regex/` — regex pattern
- `TOKEN_NAME = "literal"` — literal string (auto-escaped for regex use)
- `keywords:` section — names that are reserved (matched as NAME but reclassified)
- Order matters: first match wins

## File format: `.grammar`

```
program    = { statement } ;
statement  = assignment | expression_stmt ;
assignment = NAME EQUALS expression NEWLINE ;
expression = term { ( PLUS | MINUS ) term } ;
term       = NUMBER | NAME | LPAREN expression RPAREN ;
```

- `rule_name = body ;` — each rule ends with a semicolon
- UPPERCASE names reference tokens from the `.tokens` file
- lowercase names reference other grammar rules (can be recursive)
- `|` for alternation, `{ }` for repetition, `[ ]` for optional, `( )` for grouping

## Installation

```bash
uv add coding-adventures-grammar-tools
```

## Usage

```python
from grammar_tools import (
    parse_token_grammar,
    parse_parser_grammar,
    validate_token_grammar,
    validate_parser_grammar,
    cross_validate,
)

# Parse a .tokens file
token_grammar = parse_token_grammar(open("my_lang.tokens").read())
warnings = validate_token_grammar(token_grammar)

# Parse a .grammar file
parser_grammar = parse_parser_grammar(open("my_lang.grammar").read())
warnings = validate_parser_grammar(parser_grammar, token_grammar.token_names())

# Cross-validate both together
issues = cross_validate(token_grammar, parser_grammar)
```

## Compile Grammars

The package can also embed parsed grammars into Python modules so runtimes can
import native data structures instead of opening grammar files at startup.

```bash
cd code/grammars

grammar-tools compile-tokens algol/algol60.tokens \
  -o ../packages/python/algol-lexer/src/algol_lexer/_grammar.py

grammar-tools compile-grammar algol/algol60.grammar \
  -o ../packages/python/algol-parser/src/algol_parser/_grammar.py
```

Omit `-o` to write the generated module to stdout.
