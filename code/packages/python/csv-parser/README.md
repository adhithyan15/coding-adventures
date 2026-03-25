# coding-adventures-csv-parser

A hand-rolled CSV parser for Python that converts CSV text into a list of row maps
(dictionaries). Built from first principles as a character-by-character state machine —
no standard library `csv` module used.

## Where It Fits

```
csv-parser          ← this package (pure string processing)
      │
      ▼
sql-csv-source      (adds SQL-aware type coercion on top)
      │
      ▼
sql-execution-engine
```

The CSV parser is deliberately type-agnostic: it returns every field as a string. Type
coercion (turning `"42"` into `42`, or `""` into `None`) is left to the caller.

## Installation

```bash
pip install coding-adventures-csv-parser
```

## Usage

```python
from csv_parser import parse_csv

# Basic usage — comma-delimited
rows = parse_csv("name,age\nAlice,30\nBob,25")
# → [{"name": "Alice", "age": "30"}, {"name": "Bob", "age": "25"}]

# Custom delimiter — tab-separated (TSV)
rows = parse_csv("name\tage\nAlice\t30", delimiter="\t")
# → [{"name": "Alice", "age": "30"}]

# Quoted fields with embedded commas
rows = parse_csv('product,description\nWidget,"A small, handy widget"')
# → [{"product": "Widget", "description": "A small, handy widget"}]

# Escaped double-quotes inside quoted fields
rows = parse_csv('id,quote\n1,"She said ""hello"""')
# → [{"id": "1", "quote": 'She said "hello"'}]
```

## Error Handling

```python
from csv_parser import parse_csv
from csv_parser.errors import UnclosedQuoteError

try:
    parse_csv('a,b\n"unclosed,value')
except UnclosedQuoteError as e:
    print(e)  # Unclosed quoted field at end of input
```

## Behaviour Reference

| Scenario | Behaviour |
|----------|-----------|
| Empty file | Returns `[]` |
| Header-only file | Returns `[]` |
| Empty unquoted field (`a,,b`) | `""` for the empty field |
| Quoted field with comma | Comma is part of the value |
| Quoted field with newline | Newline is part of the value |
| Escaped double-quote (`""`) | Produces a single `"` in the value |
| Row shorter than header | Missing fields filled with `""` |
| Row longer than header | Extra fields discarded |
| Unclosed quoted field | Raises `UnclosedQuoteError` |
| Trailing newline | Ignored (no empty row produced) |

## State Machine

The parser is a four-state machine:

```
FIELD_START
    │
    ├── '"' ──────────────────► IN_QUOTED_FIELD
    │                               │
    │                          ┌────┴───────────────────────┐
    │                          │                            │
    │                   (any char except '"')          '"' ──► IN_QUOTED_MAYBE_END
    │                   append to buffer                         │
    │                                                   '"' ──► append '"', back
    │                                                   (other) ─► end field
    │
    └── other char ──────────► IN_UNQUOTED_FIELD
                                    │
                           (COMMA/NEWLINE/EOF → end field)
                           (other → append to buffer)
```

## Development

```bash
uv venv
uv pip install -e ".[dev]"
pytest tests/ -v
```
