# csv-parser

Hand-rolled CSV parser that follows RFC 4180 style quoting rules with support
for quoted commas, embedded newlines, blank fields, and ragged rows.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `CsvParser.parseCsv` for comma-separated input
- `CsvParser.parseCsvWithDelimiter` for TSV or other custom delimiters
- `UnclosedQuoteError` when the input ends inside a quoted field
- Ragged-row handling that pads missing columns with `""` and truncates extras

## Example

```fsharp
open CodingAdventures.CsvParser

let csv = "name,city\nAlice,\"New York, NY\"\n"
let rows = CsvParser.parseCsv csv

printfn "%s" rows[0]["city"] // New York, NY
```

## Development

```bash
# Run tests
bash BUILD
```
