# csv-parser (Rust)

A hand-rolled **state machine** CSV parser following RFC 4180 semantics. No external
dependencies — pure Rust standard library.

## What it does

Converts CSV text into a `Vec<HashMap<String, String>>` where:

- The **first row** is always the header (column names).
- All values are returned as **strings** — no type coercion.
- Quoted fields may contain commas, newlines, and escaped double-quotes (`""`).
- The delimiter is configurable (default: `,`; common alternatives: `\t`, `;`, `|`).

## Where it fits

```
CSV text
    │
    ▼
csv-parser           ← this crate
    │
    ▼
Vec<HashMap<String, String>>   (all values are strings)
    │
    ▼
sql-csv-source       (adds SQL-aware type coercion for the query engine)
```

This crate is intentionally type-agnostic. It is the building block for any pipeline
that ingests CSV data, regardless of what those values mean semantically.

## Usage

```rust
use coding_adventures_csv_parser::{parse_csv, parse_csv_with_delimiter};

// Default comma delimiter
let csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
let rows = parse_csv(csv)?;
println!("{}", rows[0]["name"]); // "Alice"
println!("{}", rows[0]["age"]);  // "30" (string, not integer)

// Custom tab delimiter (TSV)
let tsv = "name\tage\nAlice\t30\n";
let rows = parse_csv_with_delimiter(tsv, '\t')?;
println!("{}", rows[0]["name"]); // "Alice"
```

## API

| Function | Description |
|----------|-------------|
| `parse_csv(source: &str)` | Parse with default comma delimiter |
| `parse_csv_with_delimiter(source: &str, delimiter: char)` | Parse with custom delimiter |

Both return `Result<Vec<HashMap<String, String>>, CsvError>`.

## Error handling

| Condition | Behaviour |
|-----------|-----------|
| Unclosed quoted field | Returns `Err(CsvError::UnclosedQuote)` |
| Row shorter than header | Missing fields filled with `""` |
| Row longer than header | Extra fields silently discarded |
| Empty file | Returns `Ok(vec![])` |
| Header-only file | Returns `Ok(vec![])` |

## State machine

The parser uses a four-state machine:

```
FIELD_START
    ├── '"' → IN_QUOTED_FIELD
    │             ├── '"' → IN_QUOTED_MAYBE_END
    │             │             ├── '"' → append '"', back to IN_QUOTED_FIELD
    │             │             └── delimiter/newline/EOF → end field
    │             └── other → append to buffer
    └── other → IN_UNQUOTED_FIELD
                  ├── delimiter → end field
                  ├── newline/EOF → end field, end row
                  └── other → append to buffer
```

## Running tests

```bash
cargo test --package coding-adventures-csv-parser
```
