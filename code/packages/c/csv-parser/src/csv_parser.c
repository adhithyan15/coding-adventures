/*
 * csv_parser.c — implementation of the RFC 4180 CSV parser (see csv_parser.h).
 * A faithful port of the Rust `csv-parser` crate: the same four-state machine
 * (FieldStart / InUnquoted / InQuoted / InQuotedMaybeEnd) over the input bytes.
 */
#include "csv_parser.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, strlen, strcmp */

/* ---- growable field buffer -------------------------------------------- */

typedef struct {
    char *data;
    size_t len, cap;
} StrBuf;

static int sb_reserve(StrBuf *b, size_t extra) {
    size_t need, nc;
    if (extra > SIZE_MAX - b->len) {
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) {
        return 1;
    }
    nc = b->cap ? b->cap : 16;
    while (nc < need) {
        if (nc > SIZE_MAX / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    {
        char *nd = realloc(b->data, nc);
        if (!nd) {
            return 0;
        }
        b->data = nd;
        b->cap = nc;
    }
    return 1;
}
static int sb_push(StrBuf *b, char c) {
    if (!sb_reserve(b, 1)) {
        return 0;
    }
    b->data[b->len++] = c;
    return 1;
}
/* Malloc a NUL-terminated copy of the buffer and reset it to empty; NULL on
 * allocation failure. */
static char *sb_take(StrBuf *b) {
    char *s = malloc(b->len + 1); /* len < SIZE_MAX (it grew one byte at a time) */
    if (!s) {
        return NULL;
    }
    if (b->len) {
        memcpy(s, b->data, b->len);
    }
    s[b->len] = '\0';
    b->len = 0;
    return s;
}
static void sb_free(StrBuf *b) {
    free(b->data);
    b->data = NULL;
    b->len = b->cap = 0;
}

/* ---- growable row (array of owned strings) ----------------------------- */

typedef struct {
    char **items;
    size_t count, cap;
} RowBuild;

static int rb_push(RowBuild *r, char *owned) { /* takes ownership on success */
    if (r->count == r->cap) {
        size_t nc = r->cap ? r->cap * 2 : 4;
        char **ni;
        if (r->cap > (SIZE_MAX / sizeof(char *)) / 2) {
            return 0;
        }
        ni = realloc(r->items, nc * sizeof *ni);
        if (!ni) {
            return 0;
        }
        r->items = ni;
        r->cap = nc;
    }
    r->items[r->count++] = owned;
    return 1;
}
static void row_free_contents(CsvRow *row) {
    size_t i;
    if (row->fields) {
        for (i = 0; i < row->count; i++) {
            free(row->fields[i]);
        }
        free(row->fields);
    }
    row->fields = NULL;
    row->count = 0;
}
static void rb_free(RowBuild *r) {
    size_t i;
    for (i = 0; i < r->count; i++) {
        free(r->items[i]);
    }
    free(r->items);
    r->items = NULL;
    r->count = r->cap = 0;
}
/* Move the builder's fields into a CsvRow, leaving the builder empty. */
static CsvRow rb_take(RowBuild *r) {
    CsvRow row;
    row.fields = r->items;
    row.count = r->count;
    r->items = NULL;
    r->count = 0;
    r->cap = 0;
    return row;
}

/* ---- growable grid (array of rows) ------------------------------------ */

typedef struct {
    CsvRow *rows;
    size_t count, cap;
} GridBuild;

static int gb_push(GridBuild *g, CsvRow row) { /* takes ownership of `row` */
    if (g->count == g->cap) {
        size_t nc = g->cap ? g->cap * 2 : 4;
        CsvRow *nr;
        if (g->cap > (SIZE_MAX / sizeof(CsvRow)) / 2) {
            return 0;
        }
        nr = realloc(g->rows, nc * sizeof *nr);
        if (!nr) {
            return 0;
        }
        g->rows = nr;
        g->cap = nc;
    }
    g->rows[g->count++] = row;
    return 1;
}
static void gb_free(GridBuild *g) {
    size_t i;
    for (i = 0; i < g->count; i++) {
        row_free_contents(&g->rows[i]);
    }
    free(g->rows);
    g->rows = NULL;
    g->count = g->cap = 0;
}

/* ---- the state machine ------------------------------------------------ */

enum {
    S_FIELD_START,
    S_IN_UNQUOTED,
    S_IN_QUOTED,
    S_IN_QUOTED_MAYBE_END
};

/* Push the current field (buf, taken) onto the current row. */
static int push_field(StrBuf *buf, RowBuild *row) {
    char *s = sb_take(buf);
    if (!s || !rb_push(row, s)) {
        free(s); /* not taken on rb_push failure; free(NULL) is safe */
        return 0;
    }
    return 1;
}
/* Finish the current row (row, taken) and push it onto the grid. */
static int push_row(RowBuild *row, GridBuild *grid) {
    CsvRow finished = rb_take(row);
    if (!gb_push(grid, finished)) {
        row_free_contents(&finished);
        return 0;
    }
    return 1;
}

CsvStatus csv_parse_records(const char *source, char delimiter, CsvGrid *out) {
    GridBuild grid = {NULL, 0, 0};
    RowBuild row = {NULL, 0, 0};
    StrBuf buf = {NULL, 0, 0};
    size_t len, i = 0;
    int state = S_FIELD_START;
    CsvStatus status = CSV_OK;
    unsigned char delim = (unsigned char)delimiter;

    out->rows = NULL;
    out->count = 0;
    len = strlen(source);

    while (i < len && status == CSV_OK) {
        unsigned char ch = (unsigned char)source[i];
        switch (state) {
            case S_FIELD_START:
                if (ch == '"') {
                    state = S_IN_QUOTED;
                } else if (ch == delim) {
                    if (!push_field(&buf, &row)) { /* empty field */
                        status = CSV_ERR_ALLOC;
                    }
                } else if (ch == '\n' || ch == '\r') {
                    if (row.count > 0 && !push_field(&buf, &row)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        if (ch == '\r' && i + 1 < len && source[i + 1] == '\n') {
                            i++;
                        }
                        if (!push_row(&row, &grid)) {
                            status = CSV_ERR_ALLOC;
                        }
                    }
                } else {
                    if (!sb_push(&buf, (char)ch)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        state = S_IN_UNQUOTED;
                    }
                }
                break;
            case S_IN_UNQUOTED:
                if (ch == delim) {
                    if (!push_field(&buf, &row)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        state = S_FIELD_START;
                    }
                } else if (ch == '\n' || ch == '\r') {
                    if (!push_field(&buf, &row)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        if (ch == '\r' && i + 1 < len && source[i + 1] == '\n') {
                            i++;
                        }
                        if (!push_row(&row, &grid)) {
                            status = CSV_ERR_ALLOC;
                        } else {
                            state = S_FIELD_START;
                        }
                    }
                } else {
                    if (!sb_push(&buf, (char)ch)) {
                        status = CSV_ERR_ALLOC;
                    }
                }
                break;
            case S_IN_QUOTED:
                if (ch == '"') {
                    state = S_IN_QUOTED_MAYBE_END;
                } else if (!sb_push(&buf, (char)ch)) {
                    status = CSV_ERR_ALLOC;
                }
                break;
            case S_IN_QUOTED_MAYBE_END:
                if (ch == '"') {
                    if (!sb_push(&buf, '"')) { /* escaped "" -> " */
                        status = CSV_ERR_ALLOC;
                    } else {
                        state = S_IN_QUOTED;
                    }
                } else if (ch == delim) {
                    if (!push_field(&buf, &row)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        state = S_FIELD_START;
                    }
                } else if (ch == '\n' || ch == '\r') {
                    if (!push_field(&buf, &row)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        if (ch == '\r' && i + 1 < len && source[i + 1] == '\n') {
                            i++;
                        }
                        if (!push_row(&row, &grid)) {
                            status = CSV_ERR_ALLOC;
                        } else {
                            state = S_FIELD_START;
                        }
                    }
                } else { /* lenient: closing quote then more text */
                    if (!sb_push(&buf, (char)ch)) {
                        status = CSV_ERR_ALLOC;
                    } else {
                        state = S_IN_UNQUOTED;
                    }
                }
                break;
        }
        i++;
    }

    /* Flush the final field / row. */
    if (status == CSV_OK) {
        if (state == S_IN_QUOTED) {
            status = CSV_ERR_UNCLOSED_QUOTE;
        } else if (state == S_IN_UNQUOTED || state == S_IN_QUOTED_MAYBE_END) {
            if (!push_field(&buf, &row)) {
                status = CSV_ERR_ALLOC;
            }
        }
    }
    if (status == CSV_OK && row.count > 0) {
        if (!push_row(&row, &grid)) {
            status = CSV_ERR_ALLOC;
        }
    }

    sb_free(&buf);
    if (status != CSV_OK) {
        rb_free(&row);
        gb_free(&grid);
        return status;
    }
    rb_free(&row); /* empty by now; frees nothing */
    out->rows = grid.rows;
    out->count = grid.count;
    return CSV_OK;
}

void csv_grid_free(CsvGrid *grid) {
    size_t i;
    if (!grid) {
        return;
    }
    if (grid->rows) {
        for (i = 0; i < grid->count; i++) {
            row_free_contents(&grid->rows[i]);
        }
        free(grid->rows);
    }
    grid->rows = NULL;
    grid->count = 0;
}

/* ---- header-mapped table ---------------------------------------------- */

CsvStatus csv_parse_with_delimiter(const char *source, char delimiter,
                                   CsvTable *out) {
    CsvGrid grid;
    CsvStatus st;
    out->header.fields = NULL;
    out->header.count = 0;
    out->rows = NULL;
    out->row_count = 0;

    st = csv_parse_records(source, delimiter, &grid);
    if (st != CSV_OK) {
        return st;
    }
    if (grid.count == 0) {
        return CSV_OK; /* empty file: no header, no data */
    }
    out->header = grid.rows[0]; /* transfer the header row */
    if (grid.count == 1) {
        free(grid.rows); /* header-only: no data rows */
        return CSV_OK;
    }
    {
        size_t n = grid.count - 1;
        CsvRow *data;
        if (n > SIZE_MAX / sizeof(CsvRow)) {
            /* unreachable in practice, but free everything and fail */
            size_t i;
            row_free_contents(&out->header);
            out->header.fields = NULL;
            out->header.count = 0;
            for (i = 1; i < grid.count; i++) {
                row_free_contents(&grid.rows[i]);
            }
            free(grid.rows);
            return CSV_ERR_ALLOC;
        }
        data = malloc(n * sizeof *data);
        if (!data) {
            size_t i;
            row_free_contents(&out->header);
            out->header.fields = NULL;
            out->header.count = 0;
            for (i = 1; i < grid.count; i++) {
                row_free_contents(&grid.rows[i]);
            }
            free(grid.rows);
            return CSV_ERR_ALLOC;
        }
        memcpy(data, &grid.rows[1], n * sizeof *data); /* move the CsvRow structs */
        free(grid.rows);
        out->rows = data;
        out->row_count = n;
    }
    return CSV_OK;
}

CsvStatus csv_parse(const char *source, CsvTable *out) {
    return csv_parse_with_delimiter(source, ',', out);
}

void csv_table_free(CsvTable *table) {
    size_t i;
    if (!table) {
        return;
    }
    row_free_contents(&table->header);
    if (table->rows) {
        for (i = 0; i < table->row_count; i++) {
            row_free_contents(&table->rows[i]);
        }
        free(table->rows);
    }
    table->rows = NULL;
    table->row_count = 0;
}

const char *csv_table_get(const CsvTable *table, size_t row_index,
                          const char *column) {
    size_t col;
    int found = 0;
    size_t found_idx = 0;
    if (!table || row_index >= table->row_count) {
        return NULL;
    }
    /* The last matching header column wins, matching the crate's HashMap. */
    for (col = 0; col < table->header.count; col++) {
        if (strcmp(table->header.fields[col], column) == 0) {
            found = 1;
            found_idx = col;
        }
    }
    if (!found) {
        return NULL;
    }
    if (found_idx < table->rows[row_index].count) {
        return table->rows[row_index].fields[found_idx];
    }
    return ""; /* short row: missing columns read as "" */
}
