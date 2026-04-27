# csv-parser

Hand-rolled CSV parser that follows RFC 4180 style quoting rules with support
for quoted commas, embedded newlines, blank fields, and ragged rows.

## Layer 1

This package is part of Layer 1 of the coding-adventures computing stack.

## What It Includes

- `ParseCsv` for comma-separated input
- `ParseCsvWithDelimiter` for TSV or other delimiter-separated variants
- `UnclosedQuoteError` when the input ends inside a quoted field
- Ragged-row handling that pads missing columns with `""` and truncates extras

## Example

```csharp
using CodingAdventures.CsvParser;

var csv = "name,city\nAlice,\"New York, NY\"\n";
var rows = CsvParser.ParseCsv(csv);

Console.WriteLine(rows[0]["city"]); // New York, NY
```

## Development

```bash
# Run tests
bash BUILD
```
