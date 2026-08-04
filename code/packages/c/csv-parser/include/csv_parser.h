/*
 * csv_parser.h — an RFC 4180 CSV parser, in pure ISO C17. A faithful port of
 * the Rust `csv-parser` crate.
 * ===========================================================================
 *
 * Parses comma-separated (or any single-byte delimiter) text into rows of
 * string fields, honouring the awkward parts of the format:
 *
 *   - Quoted fields:      "a, b"        -> a delimiter inside quotes is literal
 *   - Embedded newlines:  "line1\nline2" -> a newline inside quotes is literal
 *   - Escaped quotes:     "she said ""hi""" -> "" becomes a single "
 *   - Ragged rows:        short rows pad with "", extra fields are dropped
 *   - Line endings:       \n, \r, and \r\n are all recognised
 *   - Trailing newline:   optional (the last record may omit it)
 *
 * The only hard error is an unclosed quoted field (EOF inside "...").
 *
 * Two views are offered, mirroring the crate:
 *   csv_parse_records — the raw grid (CsvGrid): every row in file order,
 *                       including the header row, as arrays of strings.
 *   csv_parse         — a header-mapped table (CsvTable): the first row is the
 *                       header; look data-row values up by column name with
 *                       csv_table_get (missing columns read as "").
 *
 * UNICODE: the state machine only ever branches on ASCII bytes (the quote,
 * delimiter, CR, LF); all other bytes — including multibyte UTF-8 — are copied
 * verbatim into fields, so UTF-8 content is preserved. The delimiter is a single
 * byte.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef CSV_PARSER_H
#define CSV_PARSER_H

#include <stddef.h> /* size_t */

typedef enum {
    CSV_OK = 0,
    CSV_ERR_UNCLOSED_QUOTE, /* EOF reached inside a quoted field */
    CSV_ERR_ALLOC           /* out of memory */
} CsvStatus;

/* One row: `count` NUL-terminated field strings. */
typedef struct {
    char **fields;
    size_t count;
} CsvRow;

/* A raw grid of rows in file order (the output of csv_parse_records). */
typedef struct {
    CsvRow *rows;
    size_t count;
} CsvGrid;

/* A header-mapped table (the output of csv_parse): the header row plus the data
 * rows that follow it. */
typedef struct {
    CsvRow header;    /* column names (empty if the input had no rows) */
    CsvRow *rows;     /* data rows only (the header is excluded) */
    size_t row_count;
} CsvTable;

/* ---- raw grid --------------------------------------------------------- */

/* csv_parse_records — parse `source` into its raw grid using `delimiter`. On
 * CSV_OK fills *out (free with csv_grid_free); otherwise *out is zeroed. */
CsvStatus csv_parse_records(const char *source, char delimiter, CsvGrid *out);

/* csv_grid_free — free a grid and all its strings. Safe on a zeroed grid. */
void csv_grid_free(CsvGrid *grid);

/* ---- header-mapped table ---------------------------------------------- */

/* csv_parse — parse `source` (comma delimiter) into a header-mapped table. */
CsvStatus csv_parse(const char *source, CsvTable *out);

/* csv_parse_with_delimiter — like csv_parse but with a chosen delimiter. */
CsvStatus csv_parse_with_delimiter(const char *source, char delimiter,
                                   CsvTable *out);

/* csv_table_free — free a table and all its strings. Safe on a zeroed table. */
void csv_table_free(CsvTable *table);

/* csv_table_get — the value of column `column` in data row `row_index`
 * (0-based, the header excluded). Returns the field string, "" if the row is
 * shorter than that column, or NULL if `column` is not a header name. If the
 * header repeats a name, the last occurrence wins (matching the crate's map). */
const char *csv_table_get(const CsvTable *table, size_t row_index,
                          const char *column);

#endif /* CSV_PARSER_H */
