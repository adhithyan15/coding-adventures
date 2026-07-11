# csv-parser (C)

A pure ISO **C17** RFC 4180 **CSV parser**. A faithful port of the Rust
`csv-parser` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

A four-state machine that parses delimited text, handling the tricky parts of
CSV: quoted fields (delimiters and newlines are literal inside `"..."`), escaped
quotes (`""` → `"`), ragged rows (short rows pad with `""`, extra fields are
dropped), and all of `\n` / `\r` / `\r\n`. The only hard error is an unclosed
quoted field.

## API

Two views, mirroring the crate:

```c
#include "csv_parser.h"

/* header-mapped table: look up data-row values by column name */
CsvTable t;
if (csv_parse("name,age\nAlice,30\nBob,25\n", &t) == CSV_OK) {
    const char *v = csv_table_get(&t, 0, "name");   /* "Alice" */
    csv_table_free(&t);
}

/* raw grid: every row in file order, header included */
CsvGrid g;
if (csv_parse_records("a,b\n1,2\n", ',', &g) == CSV_OK) {
    /* g.rows[0].fields[0] == "a", g.rows[1].fields[1] == "2" */
    csv_grid_free(&g);
}
```

- `csv_parse` / `csv_parse_with_delimiter` → a `CsvTable` (header + data rows);
  `csv_table_get(t, row, column)` returns the value, `""` if the row is short,
  or `NULL` if the column isn't a header.
- `csv_parse_records` → the raw `CsvGrid`.
- All return `CSV_OK`, `CSV_ERR_UNCLOSED_QUOTE`, or `CSV_ERR_ALLOC`. On error the
  output is zeroed (nothing to free). Every allocation is overflow-guarded.

The delimiter is a single byte; multibyte UTF-8 content is preserved verbatim.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own cases — quoting, escaping, embedded commas/newlines,
ragged rows, empty and header-only files, CR/LF/CRLF endings, TSV/`;`/`|`
delimiters, the unclosed-quote error, and a multibyte UTF-8 round trip.
