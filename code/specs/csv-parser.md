# CSV Parser Specification

## Overview

This document specifies the `csv-parser` package: a standalone CSV parser that converts
CSV text into a structured list of row maps. It has **no dependency on the SQL packages**
and can be used independently for any CSV-processing task.

CSV (Comma-Separated Values) has no single authoritative grammar. The closest standard
is **RFC 4180** (2005), but real-world CSV files deviate from it in many ways. This spec
defines our **dialect** — a pragmatic superset of RFC 4180 that handles the most common
real-world variations.

---

## The Challenge of CSV Grammars

Unlike SQL or JSON, CSV cannot be lexed by a context-free tokenizer. The reason:

```
field1,"field,with,commas",field3
```

The commas inside the quoted field are **not delimiters** — but the tokenizer can only
know that after it has seen the opening `"`. This makes CSV tokenization context-sensitive:
the meaning of a character depends on whether you are currently inside a quoted field.

Because of this, CSV parsers are typically implemented as **state machines** or hand-rolled
**character-by-character scanners**, not as grammar-driven lexer/parser pipelines. This
package follows that approach.

Think of parsing CSV like reading a book aloud: when you see a `"` character, you enter
"quoted mode" and must treat everything differently until the closing `"`. There is no way
to know ahead of time where the quote ends — you must track state as you go.

---

## Grammar (Formal Definition)

The following EBNF-style grammar defines our CSV dialect. It is not used to drive a
code-generated parser; it is here as a precise specification for human implementors.

```
file        = [header] { record }
header      = record                          # first row is the header
record      = field { COMMA field } NEWLINE
            | field { COMMA field } EOF       # last record may omit trailing newline
field       = quoted | unquoted
quoted      = DQUOTE { QCHAR | COMMA | NEWLINE | ESCAPED_QUOTE } DQUOTE
unquoted    = { UCHAR }                       # may be empty string

COMMA           = ","                         # default delimiter (configurable)
DQUOTE          = '"'
NEWLINE         = "\r\n" | "\n" | "\r"
ESCAPED_QUOTE   = DQUOTE DQUOTE              # "" inside a quoted field = literal "
QCHAR           = any char except DQUOTE     # inside a quoted field
UCHAR           = any char except COMMA, DQUOTE, NEWLINE, EOF  # inside unquoted
EOF             = end of input
```

Key points:

- **First row is always the header.** The header row defines the column names that all
  subsequent rows are keyed by.

- **Unquoted fields may be empty.** `a,,b` contains three fields: `"a"`, `""`, `"b"`.
  The empty string is a valid unquoted field.

- **Quoted fields may contain anything** — commas, newlines, even the delimiter — as
  long as double-quotes are escaped by doubling them: `"say ""hello"""` → `say "hello"`.

- **No support for `\`-escape sequences.** Only `""` escaping is recognized inside
  quoted fields. A backslash is treated as a literal character.

- **Configurable delimiter.** While `,` is the default, implementations must accept
  an alternate single-character delimiter (e.g., `\t` for TSV, `;` for European CSV).

- **Trailing newline is optional.** A file ending after the last record's final field
  (no newline) is valid.

- **Whitespace is significant.** Spaces around unquoted fields are part of the field
  value. `  hello  ` is the string `"  hello  "`, not `"hello"`. Callers that want
  trimming must do it themselves.

---

## Worked Examples

### Example 1 — Simple three-column table

Input:
```
name,age,city
Alice,30,New York
Bob,25,London
```

Output:
```
[
  {name: "Alice", age: "30", city: "New York"},
  {name: "Bob",   age: "25", city: "London"},
]
```

Notice that `age` is returned as a **string** `"30"`, not the integer `30`. The CSV
parser is type-agnostic — it returns all values as strings. Type coercion (e.g., for
SQL semantics) is done by the caller (see `sql-csv-source`).

### Example 2 — Quoted field with embedded comma

Input:
```
product,price,description
Widget,9.99,"A small, round widget"
Gadget,19.99,Electronic device
```

Output:
```
[
  {product: "Widget", price: "9.99",  description: "A small, round widget"},
  {product: "Gadget", price: "19.99", description: "Electronic device"},
]
```

The comma inside `"A small, round widget"` is part of the field value, not a delimiter.
The parser must track quote state to know this.

### Example 3 — Quoted field with embedded newline

Input (the second record spans two physical lines):
```
id,note
1,"Line one
Line two"
2,Single line
```

Output:
```
[
  {id: "1", note: "Line one\nLine two"},
  {id: "2", note: "Single line"},
]
```

The embedded newline is preserved literally. The parser must not treat it as a record
separator while inside a quoted field.

### Example 4 — Escaped double-quote

Input:
```
id,value
1,"She said ""hello"""
2,plain
```

Output:
```
[
  {id: "1", value: "She said \"hello\""},
  {id: "2", value: "plain"},
]
```

The sequence `""` inside a quoted field represents a single `"` character.

### Example 5 — Empty fields

Input:
```
a,b,c
1,,3
,2,
```

Output:
```
[
  {a: "1", b: "",  c: "3"},
  {a: "",  b: "2", c: ""},
]
```

Empty unquoted fields (two consecutive commas, or a trailing comma before newline) produce
the empty string `""`.

### Example 6 — Tab-delimited (TSV) with custom delimiter

Input (tabs as delimiter):
```
name\tage
Alice\t30
```

Calling `parse_csv(input, delimiter: "\t")` produces:
```
[
  {name: "Alice", age: "30"},
]
```

---

## State Machine Walkthrough

Implementations use a character-by-character state machine with these states:

```
                 START
                   │
    ┌──────────────┼──────────────┐
    │              │              │
    ▼              ▼              ▼
FIELD_START    (COMMA → empty   (NEWLINE → new record)
    │           field, next)
    │
    ├──── '"' ──────────────► IN_QUOTED_FIELD
    │                              │
    │                         ┌───┴───────────────────┐
    │                         │                       │
    │                    (any char        '"' → IN_QUOTED_MAYBE_END
    │                    except '"')              │
    │                    append to              '"' → append '"', back to IN_QUOTED_FIELD
    │                    buffer                 │
    │                         │               (COMMA/NEWLINE/EOF → end field)
    │
    └──── other char ──────► IN_UNQUOTED_FIELD
                                   │
                              ┌────┴────────────────────┐
                              │                         │
                         (any char except         (COMMA → end field, next)
                          COMMA/NEWLINE/EOF)       (NEWLINE → end field, new record)
                         append to buffer         (EOF → end field, end file)
```

States:
1. **`FIELD_START`** — between fields; decide whether this field is quoted or unquoted.
2. **`IN_UNQUOTED_FIELD`** — consuming a plain field until `,`, newline, or EOF.
3. **`IN_QUOTED_FIELD`** — consuming a quoted field; only `"` is special.
4. **`IN_QUOTED_MAYBE_END`** — just saw `"` inside a quoted field; next char decides
   whether this is `""` (escape, stay quoted) or end of quote (field done).

---

## Error Handling

| Condition | Behaviour |
|-----------|-----------|
| Unclosed quoted field (EOF inside quotes) | Return error: `UnclosedQuoteError` |
| Row with fewer fields than header | Missing fields filled with `""` |
| Row with more fields than header | Extra fields are discarded |
| Empty file (zero bytes) | Return empty list `[]` |
| Header-only file (one row, no data) | Return empty list `[]` |
| Non-UTF-8 bytes | Implementation-defined; error or replacement character |

Implementations should **not** raise errors for ragged rows (mismatched field counts)
because many real-world CSV files have inconsistent column counts. Instead, use the
header as the authoritative column list and pad/truncate data rows to match.

---

## Public API

### Primary Function

```
parse_csv(source: string) → [{column_name: string → value: string}]
```

- `source` — the full CSV text as a string (UTF-8)
- Returns a list of maps from column name (string) to field value (string)
- Raises/returns `{:error, reason}` | `ParseError` on malformed input (unclosed quote)

### With Options

```
parse_csv(source: string, delimiter: char) → [{column_name → value}]
```

- `delimiter` — single character, default `","`. Common alternatives: `"\t"`, `";"`, `"|"`

### Language-Specific Signatures

| Language | Signature |
|----------|-----------|
| Elixir | `CsvParser.parse_csv(source)` → `{:ok, rows}` \| `{:error, reason}` |
| Go | `csvparser.ParseCSV(source string) ([]map[string]string, error)` |
| Python | `parse_csv(source: str) -> list[dict[str, str]]` |
| Ruby | `CodingAdventures::CsvParser.parse_csv(source) → Array<Hash>` |
| Rust | `pub fn parse_csv(source: &str) -> Result<Vec<HashMap<String, String>>, CsvError>` |
| TypeScript | `parseCSV(source: string): Record<string, string>[]` |

With delimiter option:

| Language | Signature |
|----------|-----------|
| Elixir | `CsvParser.parse_csv(source, delimiter: ",")` |
| Go | `csvparser.ParseCSVWithDelimiter(source, delimiter rune)` |
| Python | `parse_csv(source, delimiter=",")` |
| Ruby | `CodingAdventures::CsvParser.parse_csv(source, delimiter: ",")` |
| Rust | `parse_csv_with_delimiter(source: &str, delimiter: char)` |
| TypeScript | `parseCSV(source: string, options?: { delimiter?: string })` |

---

## Implementation Notes

### Do Not Use stdlib CSV libraries

Each language has a standard-library or popular CSV parsing library:
- Python: `csv` module
- Ruby: `CSV` class
- Go: `encoding/csv`
- Rust: `csv` crate
- TypeScript: many npm packages

**Do not use these.** The point of this package is to implement CSV parsing from first
principles, exercising the same skills as building a lexer/parser for a programming
language. The implementation should be a hand-rolled state machine or recursive-descent
parser, with the state machine design documented inline as literate programming.

### Type Values

All values returned by `parse_csv` are **strings**. The CSV format itself has no type
system — everything is text. Type coercion (turning `"42"` into the integer `42`, or `""`
into `nil`/`NULL`) is the responsibility of the caller.

This clean separation means:
- `csv-parser` is a pure string-processing library
- `sql-csv-source` adds SQL-aware type coercion on top

### Literate Programming Style

Inline comments must explain the "why" of every non-obvious decision:
- Why the state machine has these states and not others
- What RFC 4180 says, and where we diverge
- Truth tables for the quoted-field escape logic
- Edge cases (empty file, trailing newline, ragged rows)

---

## Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| `sql-execution-engine.md` | The execution engine uses `DataSource.scan()` which returns typed maps; `sql-csv-source` bridges csv-parser (string maps) to that typed interface |
| `02-lexer.md` | Context: csv-parser does NOT use the grammar-driven lexer because CSV is context-sensitive. This is explicitly a different architecture. |
