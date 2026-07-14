/*
 * sqlite_file.h — a zero-dependency reader for the SQLite on-disk format
 * (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * A faithful C port of the Rust `sqlite-file` crate.  It decodes the subset of
 * the SQLite file format needed to read table rows straight out of a database's
 * bytes — no external SQLite library, no FFI, no I/O.  You hand it a byte
 * buffer (e.g. the `collection.anki2` unpacked from an Anki `.apkg`) and it
 * walks the b-trees.
 *
 *   https://www.sqlite.org/fileformat2.html
 *
 * Layers (leaf-to-root, mirroring the crate):
 *   1. varint  — the 1..9 byte big-endian base-128 integer used everywhere.
 *   2. record  — decode a row's bytes into typed values.
 *   3. header  — parse the 100-byte database header.
 *   4. pager   — borrow page N's bytes out of the buffer (1-based, zero-copy).
 *   5. btree   — walk a table/index b-tree, reassembling overflow chains and
 *                guarding against cycles and amplification DoS.
 *   6. schema  — resolve a table name to its root page and read it.
 *
 * Errors.  Every input is untrusted; every fallible routine returns an
 * sf_error_t (SF_OK == 0).  The Rust error variants carry a message string;
 * here only the discriminant is returned.  A corrupt or hostile file yields a
 * clean error, never an out-of-bounds read or unbounded loop.
 *
 * Memory.  Decoded results (rows, records, schema entries) are malloc-owned and
 * freed with the matching sf_*_free routine.  The pager and decoded byte
 * slices borrow from the caller's buffer only where documented.
 */
#ifndef SQLITE_FILE_H
#define SQLITE_FILE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* Errors                                                             */
/* ------------------------------------------------------------------ */

typedef enum {
    SF_OK = 0,
    SF_ERR_BAD_MAGIC,      /* SqliteError::BadMagic */
    SF_ERR_TRUNCATED,      /* SqliteError::Truncated */
    SF_ERR_BAD_PAGE_SIZE,  /* SqliteError::BadPageSize */
    SF_ERR_BAD_PAGE_NUMBER,/* SqliteError::BadPageNumber */
    SF_ERR_UNSUPPORTED,    /* SqliteError::Unsupported */
    SF_ERR_NO_SUCH_TABLE,  /* SqliteError::NoSuchTable */
    SF_ERR_CORRUPT,        /* SqliteError::Corrupt */
    SF_ERR_ALLOC           /* out of memory (no Rust equivalent) */
} sf_error_t;

/* ------------------------------------------------------------------ */
/* varint                                                             */
/* ------------------------------------------------------------------ */

/* Read a varint from [buf, buf+len).  On success stores the raw
 * two's-complement i64 in *value and the byte count (1..9) in *consumed and
 * returns 1; returns 0 if the buffer ends before the varint does. */
int sf_varint_read(const uint8_t *buf, size_t len, int64_t *value, size_t *consumed);

/* Encode `value` (raw two's-complement i64) into its minimal varint form in
 * out[0..9]; returns the number of bytes written (1..9). */
size_t sf_varint_write(int64_t value, uint8_t out[9]);

/* ------------------------------------------------------------------ */
/* record / values                                                    */
/* ------------------------------------------------------------------ */

typedef enum { SF_VAL_NULL, SF_VAL_INT, SF_VAL_REAL, SF_VAL_TEXT, SF_VAL_BLOB } sf_value_type_t;

/* A single decoded column value.  For TEXT/BLOB, `bytes` is a malloc-owned
 * buffer of `bytes_len` bytes (TEXT is UTF-8 and NOT NUL-terminated). */
typedef struct {
    sf_value_type_t type;
    int64_t int_val;   /* SF_VAL_INT */
    double real_val;   /* SF_VAL_REAL */
    uint8_t *bytes;    /* SF_VAL_TEXT / SF_VAL_BLOB (owned) */
    size_t bytes_len;
} sf_value_t;

/* A decoded record: an array of column values. */
typedef struct {
    sf_value_t *items;
    size_t len;
} sf_row_t;

void sf_row_free(sf_row_t *row);

/* Decode a complete record (header + payload) into its columns.  Returns
 * SF_ERR_CORRUPT on any inconsistency. */
sf_error_t sf_record_decode(const uint8_t *record, size_t len, sf_row_t *out);

/* Encode `count` values into a complete SQLite record.  On success,
 * *out_record is malloc-owned and must be released with sf_record_free(). */
sf_error_t sf_record_encode(const sf_value_t *values, size_t count,
                            uint8_t **out_record, size_t *out_len);

void sf_record_free(uint8_t *record);

/* ------------------------------------------------------------------ */
/* header                                                             */
/* ------------------------------------------------------------------ */

typedef enum { SF_UTF8, SF_UTF16LE, SF_UTF16BE } sf_text_encoding_t;

typedef struct {
    uint32_t page_size;
    uint8_t reserved_space;
    uint32_t page_count;
    uint32_t change_counter;
    uint32_t freelist_trunk;
    uint32_t freelist_count;
    uint32_t schema_cookie;
    uint32_t schema_format;
    sf_text_encoding_t text_encoding;
} sf_header_t;

sf_error_t sf_header_parse(const uint8_t *buf, size_t len, sf_header_t *out);
uint32_t sf_header_usable_size(const sf_header_t *h);

/* ------------------------------------------------------------------ */
/* pager                                                              */
/* ------------------------------------------------------------------ */

/* A read-only, zero-copy view over a database's bytes. */
typedef struct {
    const uint8_t *data;
    size_t len;
    size_t page_size;
} sf_pager_t;

/* Parse the header and build a pager in one step. */
sf_error_t sf_pager_open(const uint8_t *data, size_t len, sf_header_t *out_header,
                         sf_pager_t *out_pager);

/* Borrow page `page_no` (1-based); *out points into the caller's buffer.
 * Returns SF_ERR_BAD_PAGE_NUMBER for page 0 or a page past end-of-file. */
sf_error_t sf_pager_page(const sf_pager_t *p, uint32_t page_no, const uint8_t **out,
                         size_t *out_len);

size_t sf_pager_page_count(const sf_pager_t *p);

/* ------------------------------------------------------------------ */
/* btree                                                              */
/* ------------------------------------------------------------------ */

typedef struct {
    int64_t rowid;
    uint8_t *bytes; /* owned record bytes */
    size_t len;
} sf_table_row_t;

typedef struct {
    sf_table_row_t *rows;
    size_t len;
} sf_table_rows_t;

void sf_table_rows_free(sf_table_rows_t *r);

/* Walk the table b-tree rooted at `root_page` → (rowid, record bytes) in rowid
 * order.  Bounds-, cycle-, and amplification-guarded. */
sf_error_t sf_walk_table(const sf_pager_t *p, const sf_header_t *h, uint32_t root_page,
                         sf_table_rows_t *out);

typedef struct {
    uint8_t *bytes; /* owned record bytes */
    size_t len;
} sf_blob_t;

typedef struct {
    sf_blob_t *records;
    size_t len;
} sf_records_t;

void sf_records_free(sf_records_t *r);

/* Walk the index b-tree rooted at `root_page` → every entry's record bytes
 * (used for indexes and WITHOUT ROWID tables). */
sf_error_t sf_walk_index(const sf_pager_t *p, const sf_header_t *h, uint32_t root_page,
                         sf_records_t *out);

/* ------------------------------------------------------------------ */
/* schema                                                             */
/* ------------------------------------------------------------------ */

typedef struct {
    char *object_type;  /* owned, NUL-terminated */
    char *name;         /* owned, NUL-terminated */
    char *table_name;   /* owned, NUL-terminated */
    int has_root_page;  /* 0 = None */
    uint32_t root_page;
    int has_sql;        /* 0 = None */
    char *sql;          /* owned, NUL-terminated when has_sql */
} sf_schema_entry_t;

typedef struct {
    sf_schema_entry_t *entries;
    size_t len;
} sf_schema_t;

void sf_schema_free(sf_schema_t *s);

/* Decode every row from sqlite_schema. */
sf_error_t sf_read_schema(const uint8_t *data, size_t len, sf_schema_t *out);

/* Return the root page for the table named `name` (NUL-terminated). */
sf_error_t sf_table_root_page(const uint8_t *data, size_t len, const char *name, uint32_t *out);

typedef struct {
    int64_t rowid;
    sf_row_t columns;
} sf_named_row_t;

typedef struct {
    sf_named_row_t *rows;
    size_t len;
} sf_named_rows_t;

void sf_named_rows_free(sf_named_rows_t *r);

/* Read a table by name → (rowid, decoded columns) in rowid order. */
sf_error_t sf_read_table(const uint8_t *data, size_t len, const char *name, sf_named_rows_t *out);

typedef struct {
    sf_row_t *rows;
    size_t len;
} sf_rows_t;

void sf_rows_free(sf_rows_t *r);

/* Read a WITHOUT ROWID table by name → each row's decoded columns. */
sf_error_t sf_read_without_rowid_table(const uint8_t *data, size_t len, const char *name,
                                       sf_rows_t *out);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SQLITE_FILE_H */
