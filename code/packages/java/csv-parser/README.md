# coding_adventures_csv_parser (Java)

A hand-rolled, RFC 4180-compatible CSV parser implemented in Java 21.

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
know that after entering quoted mode. This context-sensitivity means CSV parsers
are typically hand-rolled character-by-character state machines.

## The State Machine

The parser uses exactly four states:

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

```java
import com.codingadventures.csvparser.CsvParser;

// Parse CSV with default comma delimiter
List<Map<String, String>> rows = CsvParser.parseCSV(source);

// Parse with custom delimiter (tab, semicolon, pipe, etc.)
List<Map<String, String>> rows = CsvParser.parseCSVWithDelimiter(source, '\t');
```

Both methods return a `List<Map<String, String>>` where:
- Each map represents one data row
- Keys are column names from the first (header) row  
- Values are strings (no type coercion)
- Map key order matches the header column order (`LinkedHashMap`)

Both throw `CsvParser.CsvParseException` (checked) for unclosed quoted fields.

## Usage

```java
import com.codingadventures.csvparser.CsvParser;

// Basic comma-separated CSV
String csv = "name,age,city\nAlice,30,NYC\nBob,25,LA\n";
List<Map<String, String>> rows = CsvParser.parseCSV(csv);

System.out.println(rows.get(0).get("name")); // "Alice"
System.out.println(rows.get(0).get("age"));  // "30"

// Quoted fields with commas
String csv2 = "name,address\nAlice,\"123 Main St, Suite 4\"\n";
List<Map<String, String>> rows2 = CsvParser.parseCSV(csv2);
System.out.println(rows2.get(0).get("address")); // "123 Main St, Suite 4"

// Tab-separated (TSV)
String tsv = "name\tage\nAlice\t30\n";
List<Map<String, String>> rows3 = CsvParser.parseCSVWithDelimiter(tsv, '\t');
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

## Development

```bash
cd code/packages/java/csv-parser
gradle test
```

Or use the repo build tool:

```bash
bash BUILD
```
