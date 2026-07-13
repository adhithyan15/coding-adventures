/*
 * display.c — implementation of the pure-ISO C VGA text-mode display driver.
 * =========================================================================
 *
 * The framebuffer is a flat byte buffer the caller owns; cell (row, col) lives
 * at offset (row * columns + col) * 2, with the character in byte 0 and the
 * attribute in byte 1. All framebuffer accesses are bounds-checked against the
 * borrowed length so an undersized buffer degrades to a no-op instead of a
 * buffer overflow.
 */
#include "display.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, strlen, strstr */

uint8_t display_make_attribute(uint8_t fg, uint8_t bg) {
    return (uint8_t)(((bg & 0x07u) << 4) | (fg & 0x0Fu));
}

DisplayConfig display_config_default(void) {
    DisplayConfig c;
    c.columns = DISPLAY_DEFAULT_COLUMNS;
    c.rows = DISPLAY_DEFAULT_ROWS;
    c.framebuffer_base = DISPLAY_DEFAULT_FRAMEBUFFER_BASE;
    c.default_attribute = DISPLAY_DEFAULT_ATTRIBUTE;
    return c;
}

DisplayConfig display_config_compact(void) {
    DisplayConfig c = display_config_default();
    c.columns = 40;
    c.rows = 10;
    return c;
}

/* Byte offset of cell (row, col). */
static size_t cell_offset(const DisplayDriver *d, size_t row, size_t col) {
    return (row * d->config.columns + col) * DISPLAY_BYTES_PER_CELL;
}

void display_clear(DisplayDriver *d) {
    size_t total = d->config.columns * d->config.rows * DISPLAY_BYTES_PER_CELL;
    size_t i = 0;
    while (i < total && i + 1 < d->memory_len) {
        d->memory[i] = (uint8_t)' ';
        d->memory[i + 1] = d->config.default_attribute;
        i += DISPLAY_BYTES_PER_CELL;
    }
    d->cursor.row = 0;
    d->cursor.col = 0;
}

void display_init(DisplayDriver *d, DisplayConfig config, uint8_t *memory,
                  size_t memory_len) {
    d->config = config;
    d->memory = memory;
    d->memory_len = memory_len;
    d->cursor.row = 0;
    d->cursor.col = 0;
    display_clear(d);
}

void display_wrap(DisplayDriver *d, DisplayConfig config, uint8_t *memory,
                  size_t memory_len) {
    d->config = config;
    d->memory = memory;
    d->memory_len = memory_len;
    d->cursor.row = 0;
    d->cursor.col = 0;
}

void display_scroll(DisplayDriver *d) {
    size_t bytes_per_row = d->config.columns * DISPLAY_BYTES_PER_CELL;
    size_t total = d->config.rows * bytes_per_row;

    /* Shift rows 1..N-1 up into rows 0..N-2 (rows >= 1 so total >= one row). */
    size_t shift_end = total - bytes_per_row;
    for (size_t i = 0; i < shift_end; i++) {
        if (i + bytes_per_row >= d->memory_len) break; /* defensive */
        d->memory[i] = d->memory[i + bytes_per_row];
    }

    /* Clear the last row. */
    size_t last_row_start = (d->config.rows - 1) * bytes_per_row;
    for (size_t i = last_row_start; i < total && i + 1 < d->memory_len;
         i += DISPLAY_BYTES_PER_CELL) {
        d->memory[i] = (uint8_t)' ';
        d->memory[i + 1] = d->config.default_attribute;
    }

    d->cursor.row = d->config.rows - 1;
    d->cursor.col = 0;
}

void display_put_char(DisplayDriver *d, uint8_t ch) {
    switch (ch) {
        case 0x0A: /* newline */
            d->cursor.col = 0;
            d->cursor.row += 1;
            break;
        case 0x0D: /* carriage return */
            d->cursor.col = 0;
            break;
        case 0x09: /* tab: advance to next multiple of 8 */
            d->cursor.col = (d->cursor.col / 8 + 1) * 8;
            if (d->cursor.col >= d->config.columns) {
                d->cursor.col = 0;
                d->cursor.row += 1;
            }
            break;
        case 0x08: /* backspace: move left, no erase */
            if (d->cursor.col > 0) d->cursor.col -= 1;
            break;
        default: {
            size_t offset = cell_offset(d, d->cursor.row, d->cursor.col);
            if (offset + 1 < d->memory_len) {
                d->memory[offset] = ch;
                d->memory[offset + 1] = d->config.default_attribute;
            }
            d->cursor.col += 1;
            if (d->cursor.col >= d->config.columns) {
                d->cursor.col = 0;
                d->cursor.row += 1;
            }
            break;
        }
    }
    if (d->cursor.row >= d->config.rows) display_scroll(d);
}

void display_put_char_at(DisplayDriver *d, size_t row, size_t col, uint8_t ch,
                         uint8_t attr) {
    if (row >= d->config.rows || col >= d->config.columns) return;
    size_t offset = cell_offset(d, row, col);
    if (offset + 1 >= d->memory_len) return; /* defensive: undersized buffer */
    d->memory[offset] = ch;
    d->memory[offset + 1] = attr;
}

void display_puts(DisplayDriver *d, const char *s) {
    for (const char *p = s; *p != '\0'; p++)
        display_put_char(d, (uint8_t)*p);
}

void display_set_cursor(DisplayDriver *d, size_t row, size_t col) {
    size_t max_row = d->config.rows - 1;
    size_t max_col = d->config.columns - 1;
    d->cursor.row = row < max_row ? row : max_row;
    d->cursor.col = col < max_col ? col : max_col;
}

DisplayCursor display_get_cursor(const DisplayDriver *d) { return d->cursor; }

DisplayCell display_get_cell(const DisplayDriver *d, size_t row, size_t col) {
    DisplayCell cell;
    if (row >= d->config.rows || col >= d->config.columns) {
        cell.character = (uint8_t)' ';
        cell.attribute = d->config.default_attribute;
        return cell;
    }
    size_t offset = cell_offset(d, row, col);
    if (offset + 1 >= d->memory_len) { /* defensive: undersized buffer */
        cell.character = (uint8_t)' ';
        cell.attribute = d->config.default_attribute;
        return cell;
    }
    cell.character = d->memory[offset];
    cell.attribute = d->memory[offset + 1];
    return cell;
}

/* ── Snapshot ─────────────────────────────────────────────────────────────*/

static int is_ascii_ws(uint8_t b) {
    return b == ' ' || b == '\t' || b == '\n' || b == '\r' || b == '\v' ||
           b == '\f';
}

int display_snapshot(const DisplayDriver *d, DisplaySnapshot *snap) {
    snap->lines = NULL;
    snap->rows = d->config.rows;
    snap->columns = d->config.columns;
    snap->cursor = d->cursor;
    if (d->config.rows == 0) return 1;

    snap->lines = (char **)calloc(d->config.rows, sizeof(char *));
    if (snap->lines == NULL) {
        snap->rows = 0;
        return 0;
    }
    for (size_t row = 0; row < d->config.rows; row++) {
        /* Gather the row's characters, then trim trailing ASCII whitespace. */
        size_t len = d->config.columns;
        char *line = (char *)malloc(len + 1);
        if (line == NULL) {
            display_snapshot_free(snap);
            snap->rows = 0;
            return 0;
        }
        for (size_t col = 0; col < len; col++)
            line[col] = (char)display_get_cell(d, row, col).character;
        while (len > 0 && is_ascii_ws((uint8_t)line[len - 1])) len--;
        line[len] = '\0';
        snap->lines[row] = line;
    }
    return 1;
}

void display_snapshot_free(DisplaySnapshot *snap) {
    if (snap == NULL || snap->lines == NULL) return;
    for (size_t i = 0; i < snap->rows; i++) free(snap->lines[i]);
    free(snap->lines);
    snap->lines = NULL;
}

int display_snapshot_contains(const DisplaySnapshot *snap, const char *text) {
    if (snap->lines == NULL) return 0;
    for (size_t i = 0; i < snap->rows; i++)
        if (strstr(snap->lines[i], text) != NULL) return 1;
    return 0;
}

const char *display_snapshot_line_at(const DisplaySnapshot *snap, size_t row) {
    if (snap->lines == NULL || row >= snap->rows) return "";
    return snap->lines[row];
}

char *display_snapshot_to_padded(const DisplaySnapshot *snap) {
    size_t rows = snap->rows;
    size_t cols = snap->columns;
    /* Each row renders to exactly `cols` chars; rows are '\n'-joined. */
    if (cols != 0 && rows > ((size_t)-1) / cols) return NULL;
    size_t body = rows * cols;
    size_t seps = rows > 0 ? rows - 1 : 0;
    if (body > ((size_t)-1) - seps - 1) return NULL;
    char *out = (char *)malloc(body + seps + 1);
    if (out == NULL) return NULL;

    size_t pos = 0;
    for (size_t r = 0; r < rows; r++) {
        if (r > 0) out[pos++] = '\n';
        const char *line = snap->lines ? snap->lines[r] : "";
        size_t llen = strlen(line);
        for (size_t c = 0; c < cols; c++)
            out[pos++] = c < llen ? line[c] : ' ';
    }
    out[pos] = '\0';
    return out;
}
