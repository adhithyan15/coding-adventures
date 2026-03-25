# csv-parser (TypeScript)

A hand-rolled **state machine** CSV parser following RFC 4180 semantics. No external
dependencies — pure TypeScript/JavaScript.

## What it does

Converts CSV text into a `Record<string, string>[]` where:

- The **first row** is always the header (column names).
- All values are returned as **strings** — no type coercion.
- Quoted fields may contain commas, newlines, and escaped double-quotes (`""`).
- The delimiter is configurable (default: `,`; common alternatives: `\t`, `;`, `|`).

## Where it fits

```
CSV text
    │
    ▼
csv-parser           ← this package
    │
    ▼
Record<string, string>[]   (all values are strings)
    │
    ▼
sql-csv-source       (adds SQL-aware type coercion for the query engine)
```

## Usage

```typescript
import { parseCSV, parseCSVWithDelimiter, UnclosedQuoteError } from '@coding-adventures/csv-parser';

// Default comma delimiter
const csv = "name,age,city\nAlice,30,New York\nBob,25,London\n";
const rows = parseCSV(csv);
console.log(rows[0].name);  // "Alice"
console.log(rows[0].age);   // "30" (string, not number)

// Custom tab delimiter (TSV)
const tsv = "name\tage\nAlice\t30\n";
const rows2 = parseCSVWithDelimiter(tsv, "\t");

// Handle errors
try {
  parseCSV('name,value\n1,"unclosed');
} catch (e) {
  if (e instanceof UnclosedQuoteError) {
    console.error("Malformed CSV:", e.message);
  }
}
```

## API

| Export | Description |
|--------|-------------|
| `parseCSV(source)` | Parse with default comma delimiter |
| `parseCSVWithDelimiter(source, delimiter)` | Parse with custom delimiter |
| `UnclosedQuoteError` | Error class thrown for unclosed quoted fields |
| `CsvRow` | Type alias for `Record<string, string>` |
| `ParseState` | Union type of the four state machine states |

Both parsing functions return `CsvRow[]` (i.e., `Record<string, string>[]`).

## Error handling

| Condition | Behaviour |
|-----------|-----------|
| Unclosed quoted field | Throws `UnclosedQuoteError` |
| Row shorter than header | Missing fields filled with `""` |
| Row longer than header | Extra fields silently discarded |
| Empty file | Returns `[]` |
| Header-only file | Returns `[]` |

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
npm test
npm run test:coverage
```

## Coverage

100% statement, branch, function, and line coverage.
