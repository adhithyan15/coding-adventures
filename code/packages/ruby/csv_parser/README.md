# coding_adventures_csv_parser

A hand-rolled CSV parser for Ruby that converts CSV text into an array of row hashes
(column name => value). Built from first principles as a character-by-character state
machine — Ruby's standard library `CSV` class is NOT used.

## Where It Fits

```
csv_parser          ← this gem (pure string processing)
      │
      ▼
sql_csv_source      (adds SQL-aware type coercion on top)
      │
      ▼
sql_execution_engine
```

The CSV parser is deliberately type-agnostic: it returns every field as a String. Type
coercion (turning `"42"` into `42`, or `""` into `nil`) is left to the caller.

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_csv_parser"
```

Or install directly:

```bash
gem install coding_adventures_csv_parser
```

## Usage

```ruby
require "coding_adventures/csv_parser"

# Basic usage — comma-delimited
rows = CodingAdventures::CsvParser.parse_csv("name,age\nAlice,30\nBob,25")
# => [{"name"=>"Alice", "age"=>"30"}, {"name"=>"Bob", "age"=>"25"}]

# Custom delimiter — tab-separated (TSV)
rows = CodingAdventures::CsvParser.parse_csv("name\tage\nAlice\t30", delimiter: "\t")
# => [{"name"=>"Alice", "age"=>"30"}]

# Quoted fields with embedded commas
rows = CodingAdventures::CsvParser.parse_csv('product,description\nWidget,"A small, handy widget"')
# => [{"product"=>"Widget", "description"=>"A small, handy widget"}]

# Escaped double-quotes inside quoted fields
rows = CodingAdventures::CsvParser.parse_csv('id,quote\n1,"She said ""hello"""')
# => [{"id"=>"1", "quote"=>'She said "hello"'}]
```

## Error Handling

```ruby
require "coding_adventures/csv_parser"

begin
  CodingAdventures::CsvParser.parse_csv('a,b\n"unclosed,value')
rescue CodingAdventures::CsvParser::UnclosedQuoteError => e
  puts e.message  # => Unclosed quoted field at end of input
end
```

## Behaviour Reference

| Scenario | Behaviour |
|----------|-----------|
| Empty string | Returns `[]` |
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
                           (delimiter/newline/EOF → end field)
                           (other → append to buffer)
```

## Development

```bash
bundle install
bundle exec rake test
```
