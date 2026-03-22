# F03 — TOML Lexer & Parser

## Overview

TOML (Tom's Obvious Minimal Language) is a configuration file format designed
to be easy to read. It maps unambiguously to a hash table and is used by
Cargo (`Cargo.toml`), Python (`pyproject.toml`), and many other tools. Parsing
TOML is a stepping stone toward HTML and YAML — it introduces **two-phase
parsing** (context-free grammar parse followed by semantic validation), which
is the same pattern those formats require.

TOML sits between JSON and CSS in complexity:
- **JSON**: 4 grammar rules, no keywords, no comments, trivially context-free
- **TOML**: ~12 grammar rules, newline-sensitive, 4 string types, date/time
  literals, context-sensitive constraints (key uniqueness, table path
  consistency) — requires a semantic validation pass
- **CSS**: 36 grammar rules, complex token ordering, error recovery

The key challenge is that TOML's **syntax** is context-free (and can be parsed
with a CFG), but its **semantics** are not. For example, defining the same key
twice is syntactically valid but semantically illegal. We handle this with a
two-phase approach:

1. **Phase 1**: Grammar-driven parse (`toml.tokens` + `toml.grammar`) produces
   a generic `ASTNode` tree — reuses the existing `GrammarLexer` and
   `GrammarParser` infrastructure
2. **Phase 2**: Semantic validation walks the AST, enforces TOML constraints,
   and converts to a `TOMLDocument` (an ordered dict)

## Layer Position

```
    Grammar Files (.tokens, .grammar)
            │
    Grammar Tools (parse token/parser grammars)
            │
    Lexer (grammar-driven: .tokens → token stream)
            │
    Parser (grammar-driven: .grammar → AST)
            │
    ┌───────┼──────────┬──────────────────┐
    │       │          │                  │
   JSON    CSS     TOML (you are here)  Future: HTML, YAML
  (4 rules) (36 rules) (~12 rules)
```

**Depends on:** `grammar-tools`, `lexer`, `parser` (and transitively
`state-machine`, `directed-graph`).
**Used by:** Future configuration file handling, build tool config parsing.

## Part 1: Token Definitions (`toml.tokens`)

### Design Decisions

**Escape mode: `escapes: none`**

TOML has four string types with different escape semantics:
- Basic strings (`"..."`) — full escape processing (`\n`, `\t`, `\uXXXX`, etc.)
- Multi-line basic strings (`"""..."""`) — same escapes plus line-ending backslash
- Literal strings (`'...'`) — no escape processing at all
- Multi-line literal strings (`'''...'''`) — no escape processing

The generic lexer's escape processor only handles one mode. Rather than
extending the infrastructure, we use `escapes: none` (same as CSS) and defer
all string processing to the semantic layer. The lexer strips quotes but
leaves escape sequences as raw text.

**Token ordering: first-match-wins**

The `GrammarLexer` tries token patterns in definition order, taking the first
match. This ordering is critical for TOML because many patterns overlap:

| Priority | Token | Example | Why it must come first |
|----------|-------|---------|----------------------|
| 1 | ML_BASIC_STRING | `"""hello"""` | `"""` must match before `"` |
| 2 | ML_LITERAL_STRING | `'''hello'''` | `'''` must match before `'` |
| 3 | BASIC_STRING | `"hello"` | After multi-line variants |
| 4 | LITERAL_STRING | `'hello'` | After multi-line variants |
| 5 | OFFSET_DATETIME | `1979-05-27T07:32:00Z` | Before LOCAL_DATETIME, dates, times |
| 6 | LOCAL_DATETIME | `1979-05-27T07:32:00` | Before LOCAL_DATE (shares prefix) |
| 7 | LOCAL_DATE | `1979-05-27` | Before INTEGER (digits + hyphens) |
| 8 | LOCAL_TIME | `07:32:00` | Before INTEGER (digits) |
| 9 | FLOAT (special) | `inf`, `nan`, `+inf` | Before BARE_KEY |
| 10 | FLOAT (exp/dec) | `3.14`, `1e10` | Before INTEGER |
| 11 | INTEGER (hex/oct/bin) | `0xDEAD`, `0o755` | Before decimal INTEGER |
| 12 | INTEGER (decimal) | `42`, `+42` | Before BARE_KEY |
| 13 | TRUE / FALSE | `true`, `false` | Before BARE_KEY |
| 14 | BARE_KEY | `my-key` | LAST — matches `[A-Za-z0-9_-]+` |

**Newline significance**

TOML is newline-sensitive — key-value pairs are terminated by newlines. The
skip pattern includes only spaces and tabs (NOT `\n`), so the lexer emits
NEWLINE tokens. This is the same approach used by Starlark.

**No DOUBLE_LBRACKET token**

`[[array-of-tables]]` uses two separate `LBRACKET` tokens. The grammar
disambiguates from nested arrays (`[[1, 2], [3, 4]]`) by context — table
headers appear at document top-level, arrays appear after `=`.

**Bare key ambiguity**

TOML bare keys are `[A-Za-z0-9_-]+`, which matches `true`, `42`,
`1979-05-27`, etc. Since BARE_KEY is defined last, these match their
specific token types instead. The grammar's `simple_key` rule lists all
tokens that can appear as keys, handling the case where `true` appears on the
left side of `=`.

**Date-time space separator**

TOML allows `1979-05-27 07:32:00` (space instead of `T`). The datetime regex
includes `[T ]` to match either. This works because token patterns are tried
before skip patterns at each position — the datetime regex consumes the space
as part of the match.

### Token Definitions

```
# Token definitions for TOML (v1.0.0 — https://toml.io/en/v1.0.0)
#
# TOML tokenization requires careful ordering because many patterns overlap:
# multi-line strings vs single-line, dates vs bare keys, floats vs integers.
# The first-match-wins rule means more specific patterns must come first.

# ── Skip patterns ──────────────────────────────────────────────────────────
# Comments: hash to end of line (NOT including the newline itself).
# Whitespace: spaces and tabs only — newlines are significant in TOML.

skip:
  COMMENT    = /#[^\n]*/
  WHITESPACE = /[ \t]+/

# ── Escape mode ────────────────────────────────────────────────────────────
# TOML has four string types with different escape rules. We defer all string
# processing to the semantic layer (same approach as CSS).

escapes: none

# ── Multi-line strings (MUST come before single-line) ──────────────────────

ML_BASIC_STRING   = /"""([^\\]|\\.|\n)*?"""/
ML_LITERAL_STRING = /'''[\s\S]*?'''/

# ── Single-line strings ───────────────────────────────────────────────────

BASIC_STRING      = /"([^"\\\n]|\\.)*"/
LITERAL_STRING    = /'[^'\n]*'/

# ── Date/Time literals (MUST come before BARE_KEY and numbers) ─────────────
# Most specific first: offset datetime > local datetime > local date > time.

OFFSET_DATETIME = /\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})/
LOCAL_DATETIME  = /\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?/
LOCAL_DATE      = /\d{4}-\d{2}-\d{2}/
LOCAL_TIME      = /\d{2}:\d{2}:\d{2}(\.\d+)?/

# ── Float literals (MUST come before INTEGER) ──────────────────────────────
# Special values first (inf, nan), then scientific notation, then decimal.

FLOAT_SPECIAL = /[+-]?(inf|nan)/ -> FLOAT
FLOAT_EXP     = /[+-]?([0-9](_?[0-9])*)(\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*/ -> FLOAT
FLOAT_DEC     = /[+-]?([0-9](_?[0-9])*)\.([0-9](_?[0-9])*)/ -> FLOAT

# ── Integer literals (hex/oct/bin BEFORE decimal) ──────────────────────────

HEX_INTEGER = /0x[0-9a-fA-F](_?[0-9a-fA-F])*/ -> INTEGER
OCT_INTEGER = /0o[0-7](_?[0-7])*/ -> INTEGER
BIN_INTEGER = /0b[01](_?[01])*/ -> INTEGER
INTEGER     = /[+-]?[0-9](_?[0-9])*/

# ── Boolean literals ──────────────────────────────────────────────────────

TRUE  = "true"
FALSE = "false"

# ── Bare keys (MUST come LAST — matches everything above) ─────────────────

BARE_KEY = /[A-Za-z0-9_-]+/

# ── Delimiters ────────────────────────────────────────────────────────────

EQUALS   = "="
DOT      = "."
COMMA    = ","
LBRACKET = "["
RBRACKET = "]"
LBRACE   = "{"
RBRACE   = "}"
```

## Part 2: Grammar Rules (`toml.grammar`)

### Grammar Design

The grammar is context-free — all context-sensitive rules (key uniqueness,
table path consistency) are enforced in the semantic validation pass.

```
# Parser grammar for TOML (v1.0.0)
#
# TOML has ~12 grammar rules. The grammar is context-free; semantic
# constraints (key uniqueness, table path consistency, inline table
# immutability) are enforced in a post-parse validation pass.
#
# Key insight: TOML is newline-sensitive. Key-value pairs are separated
# by newlines, not semicolons. The grammar uses NEWLINE tokens to
# delimit expressions.

# ── Document structure ─────────────────────────────────────────────────
# A TOML document is a sequence of expressions separated by newlines.
# Blank lines (consecutive NEWLINEs) are allowed anywhere.

document           = { NEWLINE | expression } ;

expression         = table_header | array_table_header | keyval ;

# ── Key-value pairs ───────────────────────────────────────────────────

keyval             = key EQUALS value ;

# ── Keys ──────────────────────────────────────────────────────────────
# A key is one or more simple keys separated by dots (dotted key).
# a.b.c = 1 is equivalent to [a.b] \n c = 1

key                = simple_key { DOT simple_key } ;

# A simple key is a bare key, a quoted key, or any token that could
# appear as a bare key (since bare keys include digits, hyphens, and
# the strings "true", "false", "inf", "nan").

simple_key         = BARE_KEY | BASIC_STRING | LITERAL_STRING
                   | TRUE | FALSE | INTEGER | FLOAT
                   | OFFSET_DATETIME | LOCAL_DATETIME
                   | LOCAL_DATE | LOCAL_TIME ;

# ── Table headers ─────────────────────────────────────────────────────

table_header       = LBRACKET key RBRACKET ;

# Array-of-tables uses two brackets: [[products]]
# The parser distinguishes this from nested arrays by context —
# array_table_header appears in expression (top-level), while
# arrays appear in value (after =).

array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;

# ── Values ────────────────────────────────────────────────────────────

value              = BASIC_STRING | ML_BASIC_STRING
                   | LITERAL_STRING | ML_LITERAL_STRING
                   | INTEGER | FLOAT | TRUE | FALSE
                   | OFFSET_DATETIME | LOCAL_DATETIME
                   | LOCAL_DATE | LOCAL_TIME
                   | array | inline_table ;

# ── Arrays ────────────────────────────────────────────────────────────
# Arrays can span multiple lines. NEWLINEs are allowed between elements.
# Trailing commas are permitted.

array              = LBRACKET array_values RBRACKET ;

array_values       = { NEWLINE }
                     [ value { NEWLINE }
                       { COMMA { NEWLINE } value { NEWLINE } }
                       [ COMMA ]
                       { NEWLINE } ] ;

# ── Inline tables ─────────────────────────────────────────────────────
# Inline tables are intended to be single-line. The grammar allows the
# syntax; the semantic layer enforces the single-line constraint.

inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE ;
```

## Part 3: toml-lexer Package

Thin wrapper around `GrammarLexer`, following the `json-lexer` pattern.

### Public API (Python reference)

```python
from toml_lexer import create_toml_lexer, tokenize_toml

# Create a lexer for fine-grained control
lexer = create_toml_lexer('[server]\nhost = "localhost"\nport = 8080')
tokens = lexer.tokenize()

# Or use the all-in-one function
tokens = tokenize_toml('[server]\nhost = "localhost"\nport = 8080')
```

### Implementation

Each language's `toml-lexer` is a thin wrapper (~50 lines) that:
1. Locates `toml.tokens` from `code/grammars/`
2. Parses it with `grammar_tools.parse_token_grammar()`
3. Creates a `GrammarLexer` configured for TOML
4. Exposes `create_toml_lexer(source)` and `tokenize_toml(source)`

## Part 4: toml-parser Package

Two layers: grammar-driven parse and semantic validation.

### Layer 1: Grammar Parse

Thin wrapper around `GrammarParser`, same as `json-parser`:

```python
from toml_parser import parse_toml_ast

ast = parse_toml_ast('[server]\nhost = "localhost"')
# Returns: ASTNode(rule_name="document", children=[...])
```

### Layer 2: Semantic Validation & Conversion

Walks the `ASTNode` tree and produces a `TOMLDocument`:

```python
from toml_parser import parse_toml, TOMLDocument

doc = parse_toml("""
[server]
host = "localhost"
port = 8080
enabled = true

[database]
connection = "postgresql://localhost/mydb"
pool_size = 5
""")

assert doc["server"]["host"] == "localhost"
assert doc["server"]["port"] == 8080
assert doc["database"]["pool_size"] == 5
```

### Types

```python
from datetime import datetime, date, time
from typing import Union

# The recursive type for TOML values
TOMLValue = Union[
    str, int, float, bool,
    datetime, date, time,
    list,           # list[TOMLValue]
    "TOMLDocument", # nested table
]

class TOMLDocument(dict):
    """An ordered dict representing a TOML document or table."""
    pass
```

### Semantic Constraints Enforced

1. **Key uniqueness** — No duplicate keys within a table.
   ```toml
   name = "first"
   name = "second"  # ERROR: key 'name' already defined
   ```

2. **Table path consistency** — Cannot define a table header for a path
   that is already a non-table value.
   ```toml
   a = 1
   [a]        # ERROR: 'a' is already defined as integer
   ```

3. **Inline table immutability** — Keys in inline tables cannot be extended.
   ```toml
   server = { host = "localhost" }
   [server]   # ERROR: cannot extend inline table 'server'
   ```

4. **Array-of-tables consistency** — Cannot mix `[name]` and `[[name]]`.
   ```toml
   [products]
   [[products]]  # ERROR: 'products' defined as table, not array of tables
   ```

### Value Conversions

| TOML Type | Token(s) | Python Type | Notes |
|-----------|----------|-------------|-------|
| Basic string | BASIC_STRING | `str` | Process `\n`, `\t`, `\\`, `\"`, `\uXXXX`, `\UXXXXXXXX` |
| ML basic string | ML_BASIC_STRING | `str` | Same escapes + trim first newline, line-ending `\` continuation |
| Literal string | LITERAL_STRING | `str` | No escape processing |
| ML literal string | ML_LITERAL_STRING | `str` | No escapes + trim first newline |
| Integer | INTEGER | `int` | Remove `_`, handle `0x`/`0o`/`0b` prefixes |
| Float | FLOAT | `float` | Remove `_`, handle `inf`/`nan` |
| Boolean | TRUE/FALSE | `bool` | Direct mapping |
| Offset Date-Time | OFFSET_DATETIME | `datetime` | With timezone info |
| Local Date-Time | LOCAL_DATETIME | `datetime` | Without timezone |
| Local Date | LOCAL_DATE | `date` | Date only |
| Local Time | LOCAL_TIME | `time` | Time only |
| Array | `[...]` | `list` | Recursive |
| Inline Table | `{...}` | `TOMLDocument` | Recursive, marked immutable |

### Dotted Key Expansion

Dotted keys create intermediate tables:

```toml
a.b.c = 1
```

Equivalent to:

```python
{"a": {"b": {"c": 1}}}
```

The converter tracks which tables were implicitly created (by dotted keys or
super-tables) vs explicitly defined, to enforce consistency rules.

## Part 5: Languages (All 6)

| Language | toml-lexer | toml-parser | Pattern follows |
|----------|-----------|-------------|-----------------|
| Python | `code/packages/python/toml-lexer/` | `code/packages/python/toml-parser/` | json-lexer/json-parser |
| Ruby | `code/packages/ruby/toml_lexer/` | `code/packages/ruby/toml_parser/` | json_lexer/json_parser |
| TypeScript | `code/packages/typescript/toml-lexer/` | `code/packages/typescript/toml-parser/` | json-lexer/json-parser |
| Go | `code/packages/go/toml-lexer/` | `code/packages/go/toml-parser/` | json-lexer/json-parser |
| Rust | `code/packages/rust/toml-lexer/` | `code/packages/rust/toml-parser/` | json-lexer/json-parser |
| Elixir | `code/packages/elixir/toml_lexer/` | `code/packages/elixir/toml_parser/` | json_lexer/json_parser |

Each language implements both the grammar-driven tokenizer/parser (thin
wrappers) and the semantic validation/conversion layer. The grammar files
(`toml.tokens`, `toml.grammar`) are shared across all languages.

## Build Order

| Step | What | Dependencies |
|------|------|-------------|
| 1 | `toml.tokens` + `toml.grammar` | Grammar tools |
| 2 | toml-lexer (all languages) | Step 1 |
| 3 | toml-parser grammar layer (all languages) | Step 2 |
| 4 | toml-parser converter + types (all languages) | Step 3 |

Steps 2-4 can be done per-language. Python is the reference implementation.

## Verification

1. `grammar-tools validate toml.tokens toml.grammar` passes
2. Token ordering edge cases tested: `true` → TRUE, `1979-05-27` → LOCAL_DATE,
   `3.14` → FLOAT, `42` → INTEGER, `my-key` → BARE_KEY
3. All four string types tokenize and convert correctly
4. All number formats (decimal, hex, octal, binary, underscore separators)
5. All date/time formats (offset, local datetime, date, time)
6. Semantic validation catches: duplicate keys, table path conflicts, inline
   table extension, array-of-tables inconsistency
7. Python output matches `tomllib.loads()` for valid inputs
8. All linters pass (ruff, standardrb, go vet, cargo clippy)
9. Coverage ≥ 95% per package
