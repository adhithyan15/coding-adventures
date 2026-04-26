# coding_adventures_csv_parser (Kotlin)

A hand-rolled, RFC 4180-compatible CSV parser implemented in Kotlin.

See `code/specs/csv-parser.md` for the full specification.

## What is CSV?

CSV (Comma-Separated Values) is the world's most common data interchange
format. Spreadsheets export it, databases dump it, scientists share it. Despite
its ubiquity, CSV has no formal specification. RFC 4180 (2005) is the closest
standard, but real-world files deviate from it constantly.

## Why a Hand-Rolled Parser?

CSV cannot be tokenized with a simple regex. Consider:

```
field1,"field,with,commas",field3
```

The commas inside the quoted field are not delimiters — but the parser can only
know that after entering quoted mode. This context-sensitivity requires a
hand-rolled character-by-character state machine.

## The State Machine

```
         ┌──────────────┐
         │  FIELD_START │◄──────────────────────────────────┐
         └──────┬───────┘                                   │
                │                                           │
   ┌────────────┼──────────────────┐                        │
   │            │                  │                        │
  '"'        other char      DELIMITER or NEWLINE           │
   │            │                  │                        │
   ▼            ▼                  │  emit empty field      │
┌──────────┐ ┌──────────────┐      └───────────────────────┘
│IN_QUOTED │ │ IN_UNQUOTED  │
│  _FIELD  │ │   _FIELD     │
└──────┬───┘ └──────┬───────┘
       │            │
   '"' │       DELIMITER → end field
       ▼       NEWLINE   → end row
┌──────────────────┐
│ IN_QUOTED_MAYBE  │
│     _END         │
└──────────────────┘
```

## API

```kotlin
import com.codingadventures.csvparser.parseCSV
import com.codingadventures.csvparser.parseCSVWithDelimiter

// Parse CSV with default comma delimiter
val rows: List<Map<String, String>> = parseCSV(source)

// Parse with custom delimiter
val rows: List<Map<String, String>> = parseCSVWithDelimiter(source, '\t')
```

Both functions return a `List<Map<String, String>>` where:
- Each map represents one data row
- Keys are column names from the first (header) row
- Values are strings (no type coercion)
- Map key order matches the header column order

Both throw `CsvParseException` for unclosed quoted fields.

## Usage

```kotlin
import com.codingadventures.csvparser.parseCSV
import com.codingadventures.csvparser.parseCSVWithDelimiter

// Basic comma-separated CSV
val csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\n"
val rows = parseCSV(csv)
println(rows[0]["name"]) // "Alice"
println(rows[0]["age"])  // "30"

// Quoted fields with commas
val csv2 = "name,address\nAlice,\"123 Main St, Suite 4\"\n"
val rows2 = parseCSV(csv2)
println(rows2[0]["address"]) // "123 Main St, Suite 4"

// Tab-separated values (TSV)
val tsv = "name\tage\nAlice\t30\n"
val rows3 = parseCSVWithDelimiter(tsv, '\t')
```

## Behaviour Details

| Feature           | Behaviour                                          |
|-------------------|----------------------------------------------------|
| Header row        | Always the first row; not included in results      |
| Type coercion     | None — all values are strings                      |
| Quoted fields     | RFC 4180: commas, newlines, and `""` escapes work  |
| Ragged rows       | Short rows padded with `""`; long rows truncated   |
| Newline variants  | `\n`, `\r\n`, and `\r` all handled correctly       |
| Blank lines       | Silently ignored                                   |
| Whitespace        | Preserved (not trimmed)                            |
| Unclosed quote    | Throws `CsvParseException`                         |

## Kotlin Note

The parser is implemented as a top-level `parseCSV` function (not a class) and
a private `Parser` class for internal state management. This is idiomatic Kotlin
— package-level functions are preferred over utility classes for pure functions.

## Development

```bash
cd code/packages/kotlin/csv-parser
gradle test
```

Or use the repo build tool:

```bash
bash BUILD
```
