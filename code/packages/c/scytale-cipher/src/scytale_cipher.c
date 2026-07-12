/*
 * scytale_cipher.c — implementation of the Scytale cipher (see
 * scytale_cipher.h). A faithful port of the Rust `scytale-cipher` crate,
 * transposing whole UTF-8 character units (not bytes).
 */
#include "scytale_cipher.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy, strlen */

/* A single character: a span of bytes within the input (or the literal pad). */
typedef struct {
    const char *p;
    size_t len;
} Unit;

static const char PAD_SPACE[] = " ";

static char *dup_cstr(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    if (p) {
        memcpy(p, s, n);
    }
    return p;
}

/* Length in bytes of the UTF-8 character led by `c` (a stray continuation or
 * invalid byte counts as a single-byte character). */
static size_t utf8_lead_len(unsigned char c) {
    if (c < 0x80) {
        return 1;
    }
    if ((c & 0xE0) == 0xC0) {
        return 2;
    }
    if ((c & 0xF0) == 0xE0) {
        return 3;
    }
    if ((c & 0xF8) == 0xF0) {
        return 4;
    }
    return 1;
}

/* Number of UTF-8 characters in `text`. */
static size_t count_chars(const char *text) {
    size_t len = strlen(text), i = 0, n = 0;
    while (i < len) {
        size_t l = utf8_lead_len((unsigned char)text[i]);
        if (l > len - i) {
            l = len - i; /* truncated final sequence */
        }
        i += l;
        n++;
    }
    return n;
}

/* Split `text` into its character units. Returns NULL on allocation failure;
 * sets *n_out to the character count. */
static Unit *split_units(const char *text, size_t *n_out) {
    size_t len = strlen(text), i = 0, k = 0;
    size_t n = count_chars(text);
    Unit *units;
    *n_out = n;
    units = calloc(n ? n : 1, sizeof *units); /* checked multiply */
    if (!units) {
        return NULL;
    }
    while (i < len) {
        size_t l = utf8_lead_len((unsigned char)text[i]);
        if (l > len - i) {
            l = len - i;
        }
        units[k].p = text + i;
        units[k].len = l;
        i += l;
        k++;
    }
    return units;
}

/* Concatenate the bytes of `order[0..count]` into a fresh NUL-terminated string.
 * Returns NULL on overflow or allocation failure. */
static char *join_units(const Unit *order, size_t count) {
    size_t total = 0, i, oi = 0;
    char *result;
    for (i = 0; i < count; i++) {
        if (order[i].len > SIZE_MAX - total) {
            return NULL;
        }
        total += order[i].len;
    }
    if (total > SIZE_MAX - 1) {
        return NULL;
    }
    result = malloc(total + 1);
    if (!result) {
        return NULL;
    }
    for (i = 0; i < count; i++) {
        memcpy(result + oi, order[i].p, order[i].len);
        oi += order[i].len;
    }
    result[total] = '\0';
    return result;
}

char *scytale_encrypt(const char *text, size_t key) {
    Unit *units, *padded;
    size_t n, num_rows, padded_len, col, row, oi = 0;
    char *result;

    if (text[0] == '\0') {
        return dup_cstr(""); /* the crate returns "" for empty text, key first */
    }
    units = split_units(text, &n);
    if (!units) {
        return NULL;
    }
    if (key < 2 || key > n) {
        free(units);
        return NULL;
    }

    num_rows = n / key + (n % key ? 1 : 0);
    if (num_rows > SIZE_MAX / key) {
        free(units);
        return NULL;
    }
    padded_len = num_rows * key;
    if (padded_len > SIZE_MAX / sizeof(Unit)) {
        free(units);
        return NULL;
    }
    padded = malloc(padded_len * sizeof *padded);
    if (!padded) {
        free(units);
        return NULL;
    }
    /* text units, then space padding to fill the final row */
    {
        size_t i;
        for (i = 0; i < n; i++) {
            padded[i] = units[i];
        }
        for (i = n; i < padded_len; i++) {
            padded[i].p = PAD_SPACE;
            padded[i].len = 1;
        }
    }

    /* Read the grid column-by-column into a temporary order, then join. */
    {
        Unit *order = malloc(padded_len * sizeof *order);
        if (!order) {
            free(units);
            free(padded);
            return NULL;
        }
        for (col = 0; col < key; col++) {
            for (row = 0; row < num_rows; row++) {
                order[oi++] = padded[row * key + col];
            }
        }
        result = join_units(order, padded_len);
        free(order);
    }
    free(units);
    free(padded);
    return result;
}

char *scytale_decrypt(const char *text, size_t key) {
    Unit *units;
    size_t n, num_rows, full_cols, col, row, offset = 0, oi = 0, emit;
    size_t *col_starts, *col_lens;
    Unit *order;
    char *result;

    if (text[0] == '\0') {
        return dup_cstr("");
    }
    units = split_units(text, &n);
    if (!units) {
        return NULL;
    }
    if (key < 2 || key > n) {
        free(units);
        return NULL;
    }

    num_rows = n / key + (n % key ? 1 : 0);
    full_cols = (n % key == 0) ? key : (n % key);

    col_starts = calloc(key, sizeof *col_starts);
    col_lens = calloc(key, sizeof *col_lens);
    if (!col_starts || !col_lens) {
        free(units);
        free(col_starts);
        free(col_lens);
        return NULL;
    }
    for (col = 0; col < key; col++) {
        size_t len = (n % key == 0 || col < full_cols) ? num_rows : num_rows - 1;
        col_starts[col] = offset;
        col_lens[col] = len;
        offset += len; /* total == n, so no overflow */
    }

    /* Read row-by-row into the original (permuted) character order. */
    order = malloc(n * sizeof *order); /* n units emitted exactly once */
    if (!order) {
        free(units);
        free(col_starts);
        free(col_lens);
        return NULL;
    }
    for (row = 0; row < num_rows; row++) {
        for (col = 0; col < key; col++) {
            if (row < col_lens[col]) {
                order[oi++] = units[col_starts[col] + row];
            }
        }
    }
    /* Strip trailing single-space pad characters. */
    emit = oi; /* == n */
    while (emit > 0 && order[emit - 1].len == 1 && order[emit - 1].p[0] == ' ') {
        emit--;
    }

    result = join_units(order, emit);
    free(units);
    free(col_starts);
    free(col_lens);
    free(order);
    return result;
}

ScytaleBrute *scytale_brute_force(const char *text, size_t *count) {
    size_t n = count_chars(text), max_key, k, idx = 0;
    ScytaleBrute *results;

    *count = 0;
    if (n < 4) {
        return NULL;
    }
    max_key = n / 2;
    /* keys 2..=max_key -> (max_key - 1) entries (max_key >= 2 here) */
    if ((max_key - 1) > SIZE_MAX / sizeof(ScytaleBrute)) {
        return NULL;
    }
    results = malloc((max_key - 1) * sizeof *results);
    if (!results) {
        return NULL;
    }
    for (k = 2; k <= max_key; k++) {
        char *dec = scytale_decrypt(text, k);
        if (dec) {
            results[idx].key = k;
            results[idx].text = dec;
            idx++;
        }
    }
    *count = idx;
    return results;
}

void scytale_brute_free(ScytaleBrute *results, size_t count) {
    size_t i;
    if (!results) {
        return;
    }
    for (i = 0; i < count; i++) {
        free(results[i].text);
    }
    free(results);
}
