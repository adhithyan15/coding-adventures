# csv-parser (Go)

A from-scratch CSV parser for Go, implemented as a hand-rolled character-by-character state machine. No `encoding/csv` or any other standard-library CSV facility is used — the implementation exercises the same state-machine skills as building a lexer for a programming language.

## What it does

Converts CSV text into a slice of `map[string]string`, where the first row is always the header:

```go
rows, err := csvparser.ParseCSV("name,age,city\nAlice,30,New York\nBob,25,London\n")
// rows[0]["name"] == "Alice"
// rows[0]["age"]  == "30"      ← always a string, never an integer
// rows[0]["city"] == "New York"
```

## Key behaviours

- **First row is always the header** — defines map keys for all data rows
- **All values are strings** — no type coercion; `"42"` stays `"42"`
- **Quoted fields** can contain commas, newlines, and `""` (escaped double-quote)
- **Configurable delimiter** — default comma, also tab / semicolon / pipe
- **Ragged rows** — short rows padded with `""`, long rows truncated
- **Error on unclosed quote** — returns `*CsvError`

## Usage

```go
import csvparser "github.com/adhithyan15/coding-adventures/code/packages/go/csv-parser"

// Default comma delimiter
rows, err := csvparser.ParseCSV("name,age\nAlice,30\n")

// Custom delimiter (TSV)
rows, err := csvparser.ParseCSVWithDelimiter("name\tage\nAlice\t30\n", '\t')

// Quoted field with embedded comma
rows, err := csvparser.ParseCSV("a,b\n\"hello, world\",42\n")
// rows[0]["a"] == "hello, world"

// Error case
_, err = csvparser.ParseCSV("a\n\"unclosed")
// err != nil
```

## How it works

The parser uses a four-state machine: `StateFieldStart`, `StateInUnquotedField`, `StateInQuotedField`, and `StateInQuotedMaybeEnd`. State is tracked in a `parser` struct; a single `step(rune, atEOF)` method drives the transitions. See `csv_parser.go` for the full literate-programming walkthrough with state diagram, truth tables, and inline explanations.

All three newline conventions are handled: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac).

## Where it fits

- Pure string-processing library; no SQL awareness
- A higher-level `sql-csv-source` package adds SQL-aware type coercion on top
- Deliberately does **not** use `encoding/csv` — the goal is to learn state-machine construction from first principles
