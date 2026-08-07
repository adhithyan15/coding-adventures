# csv-parser (C++)

A pure ISO **C++17**, header-only RFC 4180 **CSV parser**, in namespace
`ca::csv`. A faithful port of the Rust `csv-parser` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

A four-state machine that parses delimited text, handling quoted fields
(delimiters and newlines are literal inside `"..."`), escaped quotes (`""` →
`"`), ragged rows (short rows pad with `""`, extra fields dropped), and all of
`\n` / `\r` / `\r\n`. An unclosed quoted field throws `ca::csv::UnclosedQuote`.

## API

```cpp
#include "csv_parser.hpp"
namespace csv = ca::csv;

// header-mapped records: each row is a std::map<string,string>
std::vector<csv::Record> rows = csv::parse("name,age\nAlice,30\nBob,25\n");
rows[0].at("name");   // "Alice"

// raw grid: vector<vector<string>>, header included, file order
csv::Grid g = csv::parse_records("a,b\n1,2\n", ',');
g[1][1];              // "2"
```

- `parse(source, delimiter=',')` → `std::vector<csv::Record>` (each `Record` is a
  `std::map<std::string, std::string>` keyed by header; missing columns read as
  `""`; a repeated header keeps the last column's value).
- `parse_records(source, delimiter=',')` → `csv::Grid`.
- Both throw `ca::csv::UnclosedQuote` on an unterminated quoted field.

The delimiter is a single byte; multibyte UTF-8 content is preserved verbatim.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own cases — quoting, escaping, embedded commas/newlines,
ragged rows, empty and header-only files, CR/LF/CRLF endings, TSV/`;`/`|`
delimiters, the unclosed-quote error, and a multibyte UTF-8 round trip.
