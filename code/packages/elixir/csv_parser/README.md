# csv_parser

A from-scratch CSV parser for Elixir, implemented as a hand-rolled character-by-character state machine. No stdlib CSV library is used — the implementation exercises the same state-machine skills as building a lexer for a programming language.

## What it does

Converts CSV text into a list of row maps, where the first row is always the header:

```elixir
input = """
name,age,city
Alice,30,New York
Bob,25,London
"""

{:ok, rows} = CsvParser.parse_csv(input)
# => [
#      %{"name" => "Alice", "age" => "30", "city" => "New York"},
#      %{"name" => "Bob",   "age" => "25", "city" => "London"}
#    ]
```

## Key behaviours

- **First row is always the header** — defines column names for all data rows
- **All values are strings** — no type coercion; `"42"` stays `"42"`
- **Quoted fields** can contain commas, newlines, and `""` (escaped double-quote)
- **Configurable delimiter** — default comma, also works with tab / semicolon / pipe
- **Ragged rows** — short rows padded with `""`, long rows truncated
- **Error on unclosed quote** — returns `{:error, reason}`

## Usage

```elixir
# Default comma delimiter
{:ok, rows} = CsvParser.parse_csv("name,age\nAlice,30\n")

# Custom delimiter (TSV)
{:ok, rows} = CsvParser.parse_csv("name\tage\nAlice\t30\n", "\t")

# Quoted field with embedded comma
{:ok, rows} = CsvParser.parse_csv(~s(a,b\n"hello, world",42\n))
# => [%{"a" => "hello, world", "b" => "42"}]

# Error case
{:error, reason} = CsvParser.parse_csv(~s(a\n"unclosed))
```

## How it works

The parser is a four-state machine: `FIELD_START`, `IN_UNQUOTED_FIELD`, `IN_QUOTED_FIELD`, and `IN_QUOTED_MAYBE_END`. It processes the input as a list of Unicode codepoints, pattern-matching on `(state, character)` pairs to drive transitions. See `lib/csv_parser.ex` for the full literate-programming walkthrough with state diagram and truth tables.

## Where it fits

- This package is a pure string-processing library — it knows nothing about SQL types.
- A higher-level `sql-csv-source` package sits on top and adds SQL-aware type coercion.
- The implementation deliberately does **not** use Erlang/Elixir's standard CSV facilities, because the goal is to learn state-machine construction from first principles.
