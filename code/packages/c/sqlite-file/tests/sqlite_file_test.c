/*
 * sqlite_file_test.c — unit tests for the C SQLite-file reader.
 *
 * Mirrors the Rust crate's suite across all modules: varint golden vectors +
 * round-trip sweep + truncation, record decode cases, header parsing, the
 * pager, and b-tree table/index walks including overflow reassembly, cycle
 * detection, and the anti-amplification guard.  Database fixtures are built as
 * byte arrays exactly as the crate's tests build them.
 */
#include "sqlite_file.h"
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

static const uint8_t MAGIC16[16] = {'S', 'Q', 'L', 'i', 't', 'e', ' ', 'f',
                                    'o', 'r', 'm', 'a', 't', ' ', '3', '\0'};

static void put_be16(uint8_t *b, size_t off, uint16_t v) {
    b[off] = (uint8_t)(v >> 8);
    b[off + 1] = (uint8_t)(v & 0xff);
}
static void put_be32(uint8_t *b, size_t off, uint32_t v) {
    b[off] = (uint8_t)(v >> 24);
    b[off + 1] = (uint8_t)(v >> 16);
    b[off + 2] = (uint8_t)(v >> 8);
    b[off + 3] = (uint8_t)(v & 0xff);
}

/* Deterministic LCG for the varint sweep. */
static uint64_t g_state = 0x123456789abcdef1ULL;
static uint64_t lcg_next(void) {
    g_state = g_state * 6364136223846793005ULL + 1442695040888963407ULL;
    return g_state;
}

/* ── varint ──────────────────────────────────────────────────────── */

static void test_varint(void) {
    struct { int64_t v; uint8_t bytes[4]; size_t n; } golden[] = {
        {0, {0x00}, 1}, {1, {0x01}, 1}, {127, {0x7f}, 1}, {128, {0x81, 0x00}, 2},
        {129, {0x81, 0x01}, 2}, {255, {0x81, 0x7f}, 2}, {256, {0x82, 0x00}, 2},
        {300, {0x82, 0x2c}, 2}, {16383, {0xff, 0x7f}, 2}, {16384, {0x81, 0x80, 0x00}, 3},
        {2097151, {0xff, 0xff, 0x7f}, 3},
    };
    size_t i;
    for (i = 0; i < sizeof(golden) / sizeof(golden[0]); ++i) {
        uint8_t out[9];
        size_t n = sf_varint_write(golden[i].v, out);
        int64_t dec;
        size_t consumed;
        ISO_CHECK_EQ_UINT(n, golden[i].n);
        ISO_CHECK_MEM_EQ(out, golden[i].bytes, golden[i].n);
        ISO_CHECK(sf_varint_read(golden[i].bytes, golden[i].n, &dec, &consumed) == 1);
        ISO_CHECK(dec == golden[i].v && consumed == golden[i].n);
    }
    /* max u64 (value -1) → nine 0xff bytes. */
    {
        uint8_t out[9];
        int64_t dec;
        size_t consumed;
        uint8_t expected[9];
        size_t n = sf_varint_write(-1, out);
        memset(expected, 0xff, 9);
        ISO_CHECK_EQ_UINT(n, 9u);
        ISO_CHECK_MEM_EQ(out, expected, 9);
        ISO_CHECK(sf_varint_read(out, 9, &dec, &consumed) == 1 && dec == -1 && consumed == 9);
    }
    /* sweep */
    {
        int iter;
        for (iter = 0; iter < 50000; ++iter) {
            uint8_t out[9];
            int64_t value = (int64_t)lcg_next();
            int64_t dec;
            size_t consumed;
            size_t n = sf_varint_write(value, out);
            ISO_CHECK(sf_varint_read(out, n, &dec, &consumed) == 1);
            ISO_CHECK(dec == value && consumed == n && n >= 1 && n <= 9);
        }
    }
    /* truncation */
    {
        int64_t dec;
        size_t consumed;
        uint8_t cont = 0x81;
        uint8_t eight[8];
        ISO_CHECK(sf_varint_read(&cont, 1, &dec, &consumed) == 0);
        ISO_CHECK(sf_varint_read(NULL, 0, &dec, &consumed) == 0);
        memset(eight, 0x80, 8);
        ISO_CHECK(sf_varint_read(eight, 8, &dec, &consumed) == 0);
    }
}

/* ── record ──────────────────────────────────────────────────────── */

static void test_record(void) {
    {
        uint8_t r[] = {0x04, 0x00, 0x01, 0x11, 0x2a, 0x68, 0x69}; /* [NULL,42,"hi"] */
        sf_row_t row;
        ISO_CHECK(sf_record_decode(r, sizeof r, &row) == SF_OK);
        ISO_CHECK_EQ_UINT(row.len, 3u);
        ISO_CHECK(row.items[0].type == SF_VAL_NULL);
        ISO_CHECK(row.items[1].type == SF_VAL_INT && row.items[1].int_val == 42);
        ISO_CHECK(row.items[2].type == SF_VAL_TEXT && row.items[2].bytes_len == 2);
        ISO_CHECK(memcmp(row.items[2].bytes, "hi", 2) == 0);
        sf_row_free(&row);
    }
    {
        uint8_t r[] = {0x04, 0x08, 0x09, 0x07, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0}; /* [0,1,1.5] */
        sf_row_t row;
        ISO_CHECK(sf_record_decode(r, sizeof r, &row) == SF_OK);
        ISO_CHECK(row.items[0].type == SF_VAL_INT && row.items[0].int_val == 0);
        ISO_CHECK(row.items[1].type == SF_VAL_INT && row.items[1].int_val == 1);
        ISO_CHECK(row.items[2].type == SF_VAL_REAL && row.items[2].real_val == 1.5);
        sf_row_free(&row);
    }
    {
        uint8_t r1[] = {0x02, 0x02, 0xff, 0xfe};
        uint8_t r2[] = {0x02, 0x01, 0xff};
        sf_row_t row;
        ISO_CHECK(sf_record_decode(r1, sizeof r1, &row) == SF_OK);
        ISO_CHECK(row.items[0].int_val == -2);
        sf_row_free(&row);
        ISO_CHECK(sf_record_decode(r2, sizeof r2, &row) == SF_OK);
        ISO_CHECK(row.items[0].int_val == -1);
        sf_row_free(&row);
    }
    {
        uint8_t r[] = {0x02, 0x10, 0xde, 0xad}; /* blob */
        sf_row_t row;
        uint8_t want[] = {0xde, 0xad};
        ISO_CHECK(sf_record_decode(r, sizeof r, &row) == SF_OK);
        ISO_CHECK(row.items[0].type == SF_VAL_BLOB && row.items[0].bytes_len == 2);
        ISO_CHECK_MEM_EQ(row.items[0].bytes, want, 2);
        sf_row_free(&row);
    }
    {
        uint8_t r3[] = {0x02, 0x03, 0x01, 0x00, 0x00};
        uint8_t r6[] = {0x02, 0x06, 0, 0, 0, 1, 0, 0, 0, 0};
        sf_row_t row;
        ISO_CHECK(sf_record_decode(r3, sizeof r3, &row) == SF_OK);
        ISO_CHECK(row.items[0].int_val == 65536);
        sf_row_free(&row);
        ISO_CHECK(sf_record_decode(r6, sizeof r6, &row) == SF_OK);
        ISO_CHECK(row.items[0].int_val == (int64_t)1 << 32);
        sf_row_free(&row);
    }
    {
        uint8_t c1[] = {0x04};
        uint8_t c2[] = {0x02, 0x06, 0x00};
        uint8_t c3[] = {0x02, 0x0a};
        sf_row_t row;
        ISO_CHECK(sf_record_decode(c1, sizeof c1, &row) == SF_ERR_CORRUPT);
        ISO_CHECK(sf_record_decode(c2, sizeof c2, &row) == SF_ERR_CORRUPT);
        ISO_CHECK(sf_record_decode(c3, sizeof c3, &row) == SF_ERR_CORRUPT);
    }
}

/* ── header ──────────────────────────────────────────────────────── */

static void make_header(uint8_t *buf /* 100 bytes */, uint16_t page_size_field, uint32_t enc) {
    memset(buf, 0, 100);
    memcpy(buf, MAGIC16, 16);
    put_be16(buf, 16, page_size_field);
    put_be32(buf, 56, enc);
}

static void test_header(void) {
    uint8_t buf[100];
    sf_header_t h;
    make_header(buf, 4096, 1);
    put_be32(buf, 28, 7);
    buf[20] = 0;
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_OK);
    ISO_CHECK_EQ_UINT(h.page_size, 4096u);
    ISO_CHECK_EQ_UINT(h.page_count, 7u);
    ISO_CHECK_EQ_UINT(h.reserved_space, 0u);
    ISO_CHECK(h.text_encoding == SF_UTF8);
    ISO_CHECK_EQ_UINT(sf_header_usable_size(&h), 4096u);

    make_header(buf, 1, 1);
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_OK && h.page_size == 65536u);

    make_header(buf, 4096, 1);
    buf[20] = 32;
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_OK);
    ISO_CHECK_EQ_UINT(sf_header_usable_size(&h), 4096u - 32u);

    make_header(buf, 4096, 1);
    buf[0] = 'X';
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_ERR_BAD_MAGIC);

    make_header(buf, 4097, 1);
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_ERR_BAD_PAGE_SIZE);
    make_header(buf, 256, 1);
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_ERR_BAD_PAGE_SIZE);

    {
        uint8_t tiny[50];
        memset(tiny, 0, 50);
        ISO_CHECK(sf_header_parse(tiny, 50, &h) == SF_ERR_TRUNCATED);
    }
    make_header(buf, 4096, 2);
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_OK && h.text_encoding == SF_UTF16LE);
    make_header(buf, 4096, 3);
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_OK && h.text_encoding == SF_UTF16BE);
    make_header(buf, 4096, 9);
    ISO_CHECK(sf_header_parse(buf, 100, &h) == SF_ERR_UNSUPPORTED);
}

/* ── pager ───────────────────────────────────────────────────────── */

static void test_pager(void) {
    size_t ps = 512;
    uint8_t *db = (uint8_t *)calloc(ps * 3, 1);
    sf_header_t h;
    sf_pager_t p;
    const uint8_t *page;
    size_t page_len;
    memcpy(db, MAGIC16, 16);
    put_be16(db, 16, (uint16_t)ps);
    put_be32(db, 56, 1);
    put_be32(db, 28, 3);
    db[100] = 0xA1;
    db[ps] = 0xB2;
    db[ps * 2] = 0xC3;
    ISO_CHECK(sf_pager_open(db, ps * 3, &h, &p) == SF_OK);
    ISO_CHECK_EQ_UINT(h.page_size, 512u);
    ISO_CHECK_EQ_UINT(h.page_count, 3u);
    ISO_CHECK_EQ_UINT(sf_pager_page_count(&p), 3u);
    ISO_CHECK(sf_pager_page(&p, 1, &page, &page_len) == SF_OK && page_len == 512);
    ISO_CHECK(memcmp(page, MAGIC16, 16) == 0 && page[100] == 0xA1);
    ISO_CHECK(sf_pager_page(&p, 2, &page, &page_len) == SF_OK && page[0] == 0xB2);
    ISO_CHECK(sf_pager_page(&p, 3, &page, &page_len) == SF_OK && page[0] == 0xC3);
    ISO_CHECK(sf_pager_page(&p, 0, &page, &page_len) == SF_ERR_BAD_PAGE_NUMBER);
    ISO_CHECK(sf_pager_page(&p, 4, &page, &page_len) == SF_ERR_BAD_PAGE_NUMBER);
    ISO_CHECK(sf_pager_page(&p, 0xFFFFFFFFu, &page, &page_len) == SF_ERR_BAD_PAGE_NUMBER);
    free(db);
}

/* ── btree helpers ───────────────────────────────────────────────── */

/* Append a varint to a growing cell buffer at *pos. */
static void cell_put_varint(uint8_t *cell, size_t *pos, int64_t v) {
    uint8_t tmp[9];
    size_t n = sf_varint_write(v, tmp);
    memcpy(cell + *pos, tmp, n);
    *pos += n;
}

/* One-page leaf-table db.  rows: array of (rowid, record*, len). */
static uint8_t *one_leaf_page_db(size_t ps, const int64_t *rowids, const uint8_t *const *recs,
                                 const size_t *reclens, size_t nrows, size_t *db_len) {
    uint8_t *page = (uint8_t *)calloc(ps, 1);
    size_t h = 100;
    size_t content_top = ps;
    size_t ptr_array = h + 8;
    size_t i;
    memcpy(page, MAGIC16, 16);
    put_be16(page, 16, (uint16_t)ps);
    put_be32(page, 56, 1);
    put_be32(page, 28, 1);
    page[h] = 0x0D;
    put_be16(page, h + 3, (uint16_t)nrows);
    for (i = 0; i < nrows; ++i) {
        uint8_t cell[64];
        size_t pos = 0;
        cell_put_varint(cell, &pos, (int64_t)reclens[i]);
        cell_put_varint(cell, &pos, rowids[i]);
        memcpy(cell + pos, recs[i], reclens[i]);
        pos += reclens[i];
        content_top -= pos;
        memcpy(page + content_top, cell, pos);
        put_be16(page, ptr_array + i * 2, (uint16_t)content_top);
    }
    put_be16(page, h + 5, (uint16_t)content_top);
    *db_len = ps;
    return page;
}

static size_t table_inline_len(size_t usable, size_t payload_len) {
    size_t max_local = usable - 35;
    size_t min_local = (usable - 12) * 32 / 255 - 23;
    size_t span = usable - 4;
    size_t k = min_local + (payload_len - min_local) % span;
    return (k <= max_local) ? k : min_local;
}

/* Overflow-row db: page 1 header only, page 2 one-cell leaf, overflow pages. */
static uint8_t *one_overflow_row_db(size_t ps, int64_t rowid, const uint8_t *payload,
                                    size_t payload_len, size_t *db_len) {
    size_t usable = ps;
    size_t inline_len = table_inline_len(usable, payload_len);
    size_t tail_len = payload_len - inline_len;
    size_t content = usable - 4;
    size_t n_overflow = (tail_len + content - 1) / content;
    size_t total_pages = 2 + n_overflow;
    uint32_t first_overflow = 3;
    uint8_t *data = (uint8_t *)calloc(ps * total_pages, 1);
    size_t base = ps;
    uint8_t cell[600]; /* holds varints + inline head (< max_local 477) + 4-byte ptr */
    size_t pos = 0;
    size_t cell_rel;
    size_t i;

    memcpy(data, MAGIC16, 16);
    put_be16(data, 16, (uint16_t)ps);
    put_be32(data, 56, 1);
    put_be32(data, 28, (uint32_t)total_pages);

    data[base] = 0x0D;
    put_be16(data, base + 3, 1);
    cell_put_varint(cell, &pos, (int64_t)payload_len);
    cell_put_varint(cell, &pos, rowid);
    memcpy(cell + pos, payload, inline_len);
    pos += inline_len;
    put_be32(cell, pos, first_overflow);
    pos += 4;
    cell_rel = ps - pos;
    memcpy(data + base + cell_rel, cell, pos);
    put_be16(data, base + 8, (uint16_t)cell_rel);
    put_be16(data, base + 5, (uint16_t)cell_rel);

    for (i = 0; i < n_overflow; ++i) {
        size_t page_no = first_overflow + i;
        size_t ob = (page_no - 1) * ps;
        uint32_t next = (i + 1 < n_overflow) ? (uint32_t)(page_no + 1) : 0;
        size_t off = i * content;
        size_t chunk = (tail_len - off < content) ? tail_len - off : content;
        put_be32(data, ob, next);
        memcpy(data + ob + 4, payload + inline_len + off, chunk);
    }
    *db_len = ps * total_pages;
    return data;
}

static void test_btree_leaf_and_interior(void) {
    /* leaf, out of order → sorted */
    {
        int64_t rowids[3] = {2, 1, 3};
        uint8_t r0[] = {0xAA, 0xBB};
        uint8_t r1[] = {0x01};
        uint8_t r2[] = {0xCC, 0xDD, 0xEE};
        const uint8_t *recs[3] = {r0, r1, r2};
        size_t lens[3] = {2, 1, 3};
        size_t dblen;
        uint8_t *db = one_leaf_page_db(512, rowids, recs, lens, 3, &dblen);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 1, &rows) == SF_OK);
        ISO_CHECK_EQ_UINT(rows.len, 3u);
        ISO_CHECK(rows.rows[0].rowid == 1 && rows.rows[0].len == 1 && rows.rows[0].bytes[0] == 0x01);
        ISO_CHECK(rows.rows[1].rowid == 2 && rows.rows[1].len == 2);
        ISO_CHECK(rows.rows[2].rowid == 3 && rows.rows[2].len == 3);
        sf_table_rows_free(&rows);
        free(db);
    }
    /* empty */
    {
        size_t dblen;
        uint8_t *db = one_leaf_page_db(512, NULL, NULL, NULL, 0, &dblen);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 1, &rows) == SF_OK && rows.len == 0);
        sf_table_rows_free(&rows);
        free(db);
    }
    /* interior over two leaves */
    {
        size_t ps = 512;
        uint8_t *data = (uint8_t *)calloc(ps * 3, 1);
        size_t h = 100;
        uint8_t cell[16];
        size_t pos = 0;
        size_t cell_off;
        sf_header_t hdr;
        sf_pager_t p;
        sf_table_rows_t rows;
        size_t pn;
        memcpy(data, MAGIC16, 16);
        put_be16(data, 16, (uint16_t)ps);
        put_be32(data, 56, 1);
        put_be32(data, 28, 3);
        data[h] = 0x05;
        put_be16(data, h + 3, 1);
        put_be32(data, h + 8, 3);
        put_be32(cell, 0, 2); /* left child */
        pos = 4;
        cell_put_varint(cell, &pos, 2);
        cell_off = ps - pos;
        memcpy(data + cell_off, cell, pos);
        put_be16(data, h + 12, (uint16_t)cell_off);
        /* page 2: rowid 1; page 3: rowids 2,3 */
        for (pn = 2; pn <= 3; ++pn) {
            size_t base = (pn - 1) * ps;
            size_t top = base + ps;
            size_t nrows = (pn == 2) ? 1 : 2;
            size_t k;
            int64_t rids[2];
            uint8_t recs[2] = {0, 0};
            data[base] = 0x0D;
            put_be16(data, base + 3, (uint16_t)nrows);
            if (pn == 2) { rids[0] = 1; recs[0] = 0x11; }
            else { rids[0] = 2; recs[0] = 0x22; rids[1] = 3; recs[1] = 0x33; }
            for (k = 0; k < nrows; ++k) {
                uint8_t c[8];
                size_t cp = 0;
                cell_put_varint(c, &cp, 1); /* record len 1 */
                cell_put_varint(c, &cp, rids[k]);
                c[cp++] = recs[k];
                top -= cp;
                memcpy(data + top, c, cp);
                put_be16(data, base + 8 + k * 2, (uint16_t)(top - base));
            }
        }
        ISO_CHECK(sf_pager_open(data, ps * 3, &hdr, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &hdr, 1, &rows) == SF_OK);
        ISO_CHECK_EQ_UINT(rows.len, 3u);
        ISO_CHECK(rows.rows[0].bytes[0] == 0x11 && rows.rows[1].bytes[0] == 0x22 &&
                  rows.rows[2].bytes[0] == 0x33);
        sf_table_rows_free(&rows);
        free(data);
    }
    /* unknown page type */
    {
        int64_t rid = 1;
        uint8_t rec = 0x00;
        const uint8_t *recs[1] = {&rec};
        size_t lens[1] = {1};
        size_t dblen;
        uint8_t *db = one_leaf_page_db(512, &rid, recs, lens, 1, &dblen);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        db[100] = 0x0A;
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 1, &rows) == SF_ERR_CORRUPT);
        free(db);
    }
    /* interior self-cycle */
    {
        size_t ps = 512;
        uint8_t *data = (uint8_t *)calloc(ps, 1);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        memcpy(data, MAGIC16, 16);
        put_be16(data, 16, (uint16_t)ps);
        put_be32(data, 56, 1);
        put_be32(data, 28, 1);
        data[100] = 0x05;
        put_be16(data, 103, 0);
        put_be32(data, 108, 1);
        ISO_CHECK(sf_pager_open(data, ps, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 1, &rows) == SF_ERR_CORRUPT);
        free(data);
    }
}

static void test_btree_overflow(void) {
    size_t i;
    uint8_t *payload = (uint8_t *)malloc(1500);
    for (i = 0; i < 1500; ++i) payload[i] = (uint8_t)(i % 251);
    {
        size_t dblen;
        uint8_t *db = one_overflow_row_db(512, 7, payload, 1500, &dblen);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 2, &rows) == SF_OK);
        ISO_CHECK(rows.len == 1 && rows.rows[0].rowid == 7 && rows.rows[0].len == 1500);
        ISO_CHECK_MEM_EQ(rows.rows[0].bytes, payload, 1500);
        sf_table_rows_free(&rows);
        free(db);
    }
    {
        uint8_t *big = (uint8_t *)malloc(5000);
        size_t dblen;
        uint8_t *db;
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        for (i = 0; i < 5000; ++i) big[i] = (uint8_t)(i * 7 % 256);
        db = one_overflow_row_db(512, 1, big, 5000, &dblen);
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 2, &rows) == SF_OK);
        ISO_CHECK_MEM_EQ(rows.rows[0].bytes, big, 5000);
        sf_table_rows_free(&rows);
        free(db);
        free(big);
    }
    /* cycle */
    {
        size_t dblen;
        uint8_t *db = one_overflow_row_db(512, 1, payload, 1500, &dblen);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        put_be32(db, 1024, 3); /* page 3 -> itself */
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 2, &rows) == SF_ERR_CORRUPT);
        free(db);
    }
    /* ends too soon */
    {
        size_t dblen;
        uint8_t *db = one_overflow_row_db(512, 1, payload, 1500, &dblen);
        sf_header_t h;
        sf_pager_t p;
        sf_table_rows_t rows;
        put_be32(db, 1024, 0);
        ISO_CHECK(sf_pager_open(db, dblen, &h, &p) == SF_OK);
        ISO_CHECK(sf_walk_table(&p, &h, 2, &rows) == SF_ERR_CORRUPT);
        free(db);
    }
    free(payload);
}

static void test_amplification(void) {
    size_t ps = 512;
    uint8_t *page = (uint8_t *)calloc(ps, 1);
    size_t h = 100;
    uint8_t cell[512];
    size_t pos = 0;
    size_t cell_off;
    size_t i;
    sf_header_t hdr;
    sf_pager_t p;
    sf_table_rows_t rows;
    memcpy(page, MAGIC16, 16);
    put_be16(page, 16, (uint16_t)ps);
    put_be32(page, 56, 1);
    put_be32(page, 28, 1);
    page[h] = 0x0D;
    put_be16(page, h + 3, 20); /* 20 cells */
    cell_put_varint(cell, &pos, 400);
    cell_put_varint(cell, &pos, 1);
    memset(cell + pos, 0x5A, 400);
    pos += 400;
    cell_off = ps - pos;
    memcpy(page + cell_off, cell, pos);
    for (i = 0; i < 20; ++i) put_be16(page, h + 8 + i * 2, (uint16_t)cell_off);
    ISO_CHECK(sf_pager_open(page, ps, &hdr, &p) == SF_OK);
    ISO_CHECK(sf_walk_table(&p, &hdr, 1, &rows) == SF_ERR_CORRUPT);
    free(page);
}

static void test_btree_index(void) {
    size_t ps = 512;
    /* single leaf index page */
    {
        uint8_t *page = (uint8_t *)calloc(ps, 1);
        size_t h = 100;
        size_t top = ps;
        size_t ptr = h + 8;
        sf_header_t hdr;
        sf_pager_t p;
        sf_records_t recs;
        /* three records: {01 02}, {AA}, {BB CC DD} */
        uint8_t recdata[3][3] = {{0x01, 0x02, 0}, {0xAA, 0, 0}, {0xBB, 0xCC, 0xDD}};
        size_t reclen[3] = {2, 1, 3};
        size_t i;
        int found_aa = 0, found_0102 = 0, found_bbccdd = 0;
        memcpy(page, MAGIC16, 16);
        put_be16(page, 16, (uint16_t)ps);
        put_be32(page, 56, 1);
        put_be32(page, 28, 1);
        page[h] = 0x0A;
        put_be16(page, h + 3, 3);
        for (i = 0; i < 3; ++i) {
            uint8_t c[8];
            size_t cp = 0;
            cell_put_varint(c, &cp, (int64_t)reclen[i]);
            memcpy(c + cp, recdata[i], reclen[i]);
            cp += reclen[i];
            top -= cp;
            memcpy(page + top, c, cp);
            put_be16(page, ptr + i * 2, (uint16_t)top);
        }
        put_be16(page, h + 5, (uint16_t)top);
        ISO_CHECK(sf_pager_open(page, ps, &hdr, &p) == SF_OK);
        ISO_CHECK(sf_walk_index(&p, &hdr, 1, &recs) == SF_OK);
        ISO_CHECK_EQ_UINT(recs.len, 3u);
        for (i = 0; i < recs.len; ++i) {
            if (recs.records[i].len == 1 && recs.records[i].bytes[0] == 0xAA) found_aa = 1;
            if (recs.records[i].len == 2) found_0102 = 1;
            if (recs.records[i].len == 3) found_bbccdd = 1;
        }
        ISO_CHECK(found_aa && found_0102 && found_bbccdd);
        sf_records_free(&recs);
        free(page);
    }
    /* interior index emits divider + children */
    {
        uint8_t *data = (uint8_t *)calloc(ps * 3, 1);
        size_t h = 100;
        uint8_t cell[16];
        size_t pos = 0;
        size_t cell_off;
        sf_header_t hdr;
        sf_pager_t p;
        sf_records_t recs;
        size_t i;
        int f20 = 0, f50 = 0, f80 = 0;
        uint32_t leaves[2] = {2, 3};
        uint8_t leafrec[2] = {0x20, 0x80};
        memcpy(data, MAGIC16, 16);
        put_be16(data, 16, (uint16_t)ps);
        put_be32(data, 56, 1);
        put_be32(data, 28, 3);
        data[h] = 0x02;
        put_be16(data, h + 3, 1);
        put_be32(data, h + 8, 3);
        put_be32(cell, 0, 2);
        pos = 4;
        cell_put_varint(cell, &pos, 1);
        cell[pos++] = 0x50;
        cell_off = ps - pos;
        memcpy(data + cell_off, cell, pos);
        put_be16(data, h + 12, (uint16_t)cell_off);
        for (i = 0; i < 2; ++i) {
            size_t base = (leaves[i] - 1) * ps;
            uint8_t c[4];
            size_t cp = 0;
            size_t top;
            data[base] = 0x0A;
            put_be16(data, base + 3, 1);
            cell_put_varint(c, &cp, 1);
            c[cp++] = leafrec[i];
            top = ps - cp;
            memcpy(data + base + top, c, cp);
            put_be16(data, base + 8, (uint16_t)top);
        }
        ISO_CHECK(sf_pager_open(data, ps * 3, &hdr, &p) == SF_OK);
        ISO_CHECK(sf_walk_index(&p, &hdr, 1, &recs) == SF_OK);
        ISO_CHECK_EQ_UINT(recs.len, 3u);
        for (i = 0; i < recs.len; ++i) {
            if (recs.records[i].bytes[0] == 0x20) f20 = 1;
            if (recs.records[i].bytes[0] == 0x50) f50 = 1;
            if (recs.records[i].bytes[0] == 0x80) f80 = 1;
        }
        ISO_CHECK(f20 && f50 && f80);
        sf_records_free(&recs);
        free(data);
    }
}

int main(void) {
    test_varint();
    test_record();
    test_header();
    test_pager();
    test_btree_leaf_and_interior();
    test_btree_overflow();
    test_amplification();
    test_btree_index();
    ISO_TEST_RESULT();
}
