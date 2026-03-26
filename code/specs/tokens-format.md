# Token Grammar File Format (`.tokens`)

## Overview

A `.tokens` file is a plain-text grammar definition that tells the grammar-driven
lexer how to tokenize source code. It is loaded at runtime by the `grammar-tools`
package in each language and compiled into a `TokenGrammar` data structure, which
is then passed to the `GrammarLexer` constructor.

`.tokens` files are language-agnostic — the same `sql.tokens` file is loaded by
the Go, Python, Ruby, TypeScript, Rust, and Elixir lexer wrappers identically.

---

## File Encoding

UTF-8. Line endings: LF (`\n`). The `.gitattributes` in this repository enforces
LF on all platforms.

---

## Comments

Any line whose first non-whitespace character is `#` is a comment and is ignored
by the parser:

```
# This entire line is a comment
NAME = /[a-zA-Z_]\w*/   # Inline comments are NOT supported — # only works at line start
```

Blank lines are also ignored.

---

## Magic Comments

A **magic comment** is a comment line that begins with `# @key value`. These lines
are parsed as configuration metadata before the rest of the file is processed.
Unknown `@key` names are silently ignored (forward compatibility).

### `# @version N`

```
# @version 1
```

Pins the grammar to format version `N` (a positive integer). This enables future
tooling to detect and migrate older grammar files when the format changes in a
backward-incompatible way.

- **Default when missing:** `0` (treated as the current/latest version).
- **Convention:** New grammar files should include `# @version 1`.
- **When to increment:** Only when the grammar format changes in a way that
  older `grammar-tools` versions cannot parse.

### `# @case_insensitive true|false`

```
# @case_insensitive true
```

When `true`, keyword and reserved-keyword lookup is case-insensitive. The lexer
normalizes the candidate NAME token's value with `ToUpper()` before checking the
keyword set. The emitted KEYWORD token's value is also normalized to uppercase,
so grammar rule literals like `"SELECT"` match regardless of how the user typed
the keyword (`select`, `SELECT`, `Select`, etc.).

- **Default when missing:** `false` (case-sensitive, original behavior).
- **Does NOT affect:** Regex patterns, string literals, or non-keyword identifiers.
- **Typical use:** SQL, CSS, and other case-insensitive languages.

### Magic Comment Position

By convention, magic comments appear at the top of the file, before any token
definitions. However, they are valid anywhere in the file.

---

## Token Definitions

The main body of a `.tokens` file consists of token definitions:

```
TOKEN_NAME = /regex/
TOKEN_NAME = "literal"
TOKEN_NAME = /regex/ -> ALIAS
TOKEN_NAME = "literal" -> ALIAS
```

### Name Rules

Token names use `UPPER_SNAKE_CASE`. They must match `[A-Z][A-Z0-9_]*`. Names
must be unique within the file (including across groups and sections).

### Pattern Types

| Syntax | Match type | Notes |
|--------|-----------|-------|
| `NAME = /regex/` | Regular expression | Anchored to current position (`^`) by the lexer |
| `NAME = "literal"` | Exact string match | Regex-escaped automatically by the lexer |

**Regex delimiter:** The regex is delimited by `/`. To include a literal `/`
inside a regex pattern, use the Unicode escape `\x2f`:

```
BLOCK_COMMENT = /\x2f\*([^*]|\*[^\x2f])*\*\x2f/   # matches /* ... */
```

### Aliases (`-> TYPE`)

An alias re-labels the token type at emission time. The definition name is used
for matching; the alias becomes the token's `TypeName` in the output:

```
STRING_SQ = /'([^'\\]|\\.)*'/ -> STRING
STRING_DQ = /"([^"\\]|\\.)*"/ -> STRING
```

Both patterns emit `STRING` tokens. The parser grammar uses `STRING` to refer
to either. Aliases allow multiple lexical forms to produce a single semantic type.

### Match Order

**First match wins.** Token definitions are tried in the order they appear in
the file. Longer or more specific patterns must come before shorter/generic ones:

```
STRICT_EQUALS = "==="   # Must come before EQUALS_EQUALS
EQUALS_EQUALS = "=="    # Must come before EQUALS
EQUALS        = "="
```

---

## Top-Level Directives

Top-level directives appear on their own line before any token definitions
(but after magic comments):

### `mode: indentation`

```
mode: indentation
```

Switches the lexer to **indentation mode**. In this mode, the lexer tracks
indentation levels and emits `INDENT`, `DEDENT`, and `NEWLINE` tokens based on
Python-style significant whitespace. Used by the Python and Starlark grammars.

- **Default when missing:** Standard mode (no indentation tracking).

### `escapes: none`

```
escapes: none
```

Disables escape sequence processing in string literals. When set, the lexer
strips quotes from strings but does NOT interpret `\n`, `\\`, etc. — they are
passed through as raw text. The semantic layer (parser or evaluator) handles
escapes. Used by grammars like TOML and CSS where different string types have
different escape semantics.

- **Default when missing:** Escape processing is enabled (`\n → newline`,
  `\t → tab`, `\\ → backslash`, `\" → double-quote`).

---

## Section Headers

Sections are introduced by a keyword followed by a colon, on their own line.
Indented lines inside a section define members of that section.

### `skip:` — Silent Whitespace / Comment Patterns

```
skip:
  WHITESPACE = /[ \t\r\n]+/
  LINE_COMMENT = /--[^\n]*/
```

Skip patterns are consumed without emitting tokens. They are tried before the
active group's token patterns at every position. If the grammar defines skip
patterns, they take over ALL whitespace handling (the lexer no longer silently
skips spaces/tabs by default).

### `keywords:` — Reserved Keywords

```
keywords:
  if
  else
  while
  def
  return
```

Keywords are listed one per line (case matters — use the exact case you want
matched). When the lexer matches a `NAME` token, it checks whether the value
appears in the keyword set. If so, the token type is reclassified to `KEYWORD`.

When `# @case_insensitive true` is set, the keyword set stores uppercase
versions and comparison uses `ToUpper(value)`. The emitted KEYWORD value is
also uppercased.

### `reserved:` — Reserved Words (Error on Use)

```
reserved:
  __dunder__
  __reserved__
```

Like `keywords:`, but a match raises a `LexerError` instead of producing a
KEYWORD token. Used to hard-block words that are syntactically forbidden as
identifiers.

### `errors:` — Custom Error Patterns

```
errors:
  INVALID_CHAR = /[^a-zA-Z0-9_\s]/
```

Error patterns are tried as a last resort if no token or skip pattern matches.
A match raises a `LexerError` with the pattern name. Useful for providing
better error messages than the generic "unexpected character" fallback.

---

## Pattern Groups

Pattern groups enable **context-sensitive lexing**. Different sets of patterns
become active depending on what tokens have been seen. Specified in detail by
spec `F04-lexer-pattern-groups.md`.

```
# Default group: patterns outside any group: section
TEXT         = /[^<&]+/
OPEN_TAG_START = "<"

group tag:
  TAG_NAME   = /[a-zA-Z_][\w.-]*/
  ATTR_EQUALS = "="
  TAG_CLOSE  = ">"

group cdata:
  CDATA_TEXT = /([^\]]|\](?!\]>))+/
  CDATA_END  = "]]>"
```

### Group Rules

- Patterns outside any `group:` section belong to the implicit `"default"` group.
- `group default:` is forbidden (reserved name).
- Group names use lowercase identifiers (`[a-z_][a-z0-9_]*`).
- Content within a group section is indented and follows the same
  `NAME = /pattern/` syntax. Aliases work inside groups.
- `skip:`, `keywords:`, `reserved:`, and `errors:` are **global** — not per-group.
- Group transitions are driven by on-token callbacks in lexer code, not by the
  grammar file itself (the file is declarative; callbacks are imperative).
- Reserved group names (cannot be used): `default`, `skip`, `keywords`,
  `reserved`, `errors`.

---

## Validation Rules

`grammar-tools` validates a parsed `TokenGrammar` before use. Violations raise
errors:

| Check | Error |
|-------|-------|
| Duplicate token name | `Duplicate token name: NAME` |
| `group default:` declared | `"default" is a reserved group name` |
| Group name conflicts with section name | `Group name "skip" conflicts with reserved section` |
| Group name format violation | `Invalid group name format: must match [a-z_][a-z0-9_]*` |
| Empty group | Warning (not an error) |
| Invalid regex pattern | `Failed to compile pattern for token NAME: ...` |

---

## Complete Example: `sql.tokens`

```
# SQL Token Grammar — ANSI SQL subset
# @version 1
# @case_insensitive true
#
# Note: keyword values are normalized to uppercase when @case_insensitive
# is true. Grammar literals like "SELECT" match select/SELECT/Select.

NAME          = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER        = /[0-9]+(\.[0-9]+)?/
STRING_SQ     = /'([^'\\]|\\.)*'/ -> STRING
QUOTED_ID     = /`[^`]+`/ -> NAME

LESS_EQUALS    = "<="
GREATER_EQUALS = ">="
NOT_EQUALS     = "!="
NEQ_ANSI       = "<>" -> NOT_EQUALS

EQUALS        = "="
LESS_THAN     = "<"
GREATER_THAN  = ">"
PLUS          = "+"
MINUS         = "-"
STAR          = "*"
SLASH         = "/"
PERCENT       = "%"

LPAREN        = "("
RPAREN        = ")"
COMMA         = ","
SEMICOLON     = ";"
DOT           = "."

keywords:
  SELECT FROM WHERE GROUP BY HAVING ORDER LIMIT OFFSET
  INSERT INTO VALUES UPDATE SET DELETE
  CREATE DROP TABLE IF EXISTS
  AND OR NOT NULL IS IN BETWEEN LIKE
  AS DISTINCT ALL UNION INTERSECT EXCEPT
  JOIN INNER LEFT RIGHT OUTER CROSS FULL ON
  ASC DESC TRUE FALSE CASE WHEN THEN ELSE END
  PRIMARY KEY UNIQUE DEFAULT

skip:
  WHITESPACE    = /[ \t\r\n]+/
  LINE_COMMENT  = /--[^\n]*/
  BLOCK_COMMENT = /\x2f\*([^*]|\*[^\x2f])*\*\x2f/
```

---

## Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| `02-lexer.md` | High-level lexer concepts and motivation |
| `03-parser.md` | How the token stream is consumed by the parser |
| `grammar-format.md` | The companion `.grammar` file format |
| `F04-lexer-pattern-groups.md` | Deep dive on pattern groups and callbacks |
| `lexer-parser-hooks.md` | Batch pre/post transform hooks (different from per-token callbacks) |
