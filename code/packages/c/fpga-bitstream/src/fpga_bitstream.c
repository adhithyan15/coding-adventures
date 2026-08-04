/*
 * fpga_bitstream.c — implementation of the iCE40 record-stream emitter.
 * ===========================================================================
 *
 * The emitter appends variable-length records to a growable byte buffer: a
 * preamble, a CRAM reset and bank-select, one offset + stub-data record per CLB
 * (in (row, col) order), a CRC placeholder, and the end marker. The CLB set is
 * kept as an array keyed by (row, col); emit sorts a copy so the output is
 * deterministic regardless of insertion order.
 */
#include "fpga_bitstream.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* IceStorm command codes (subset). */
#define CMD_CRAM_BANK 0x05
#define CMD_CRAM_OFFSET 0x06
#define CMD_CRAM_RESET 0x07
#define CMD_BRAM_DATA 0x08
#define CMD_CRC 0x80

/* ===========================================================================
 *  Part specs
 * =========================================================================== */

typedef struct {
    Ice40Part part;
    uint32_t rows, cols, cram_bits;
} PartSpec;

static const PartSpec PART_SPECS[] = {
    {ICE40_HX1K, 33, 17, 1024},
    {ICE40_HX8K, 33, 33, 1024},
    {ICE40_UP5K, 33, 33, 1024},
    {ICE40_LP1K, 33, 17, 1024},
};

void fpga_part_specs(Ice40Part part, uint32_t *rows, uint32_t *cols,
                     uint32_t *cram_bits) {
    for (size_t i = 0; i < sizeof PART_SPECS / sizeof PART_SPECS[0]; i++) {
        if (PART_SPECS[i].part == part) {
            *rows = PART_SPECS[i].rows;
            *cols = PART_SPECS[i].cols;
            *cram_bits = PART_SPECS[i].cram_bits;
            return;
        }
    }
    /* All Ice40Part variants are in the table; fall back defensively. */
    *rows = 0;
    *cols = 0;
    *cram_bits = 0;
}

FpgaClbConfig fpga_clb_config_default(void) {
    FpgaClbConfig c;
    memset(c.lut_a_truth_table, 0, sizeof c.lut_a_truth_table);
    memset(c.lut_b_truth_table, 0, sizeof c.lut_b_truth_table);
    c.ff_a_enabled = 0;
    c.ff_b_enabled = 0;
    return c;
}

/* ===========================================================================
 *  FpgaConfig — a (row, col)-keyed CLB set
 * =========================================================================== */

typedef struct {
    uint32_t row, col;
    FpgaClbConfig clb;
} ClbEntry;

struct FpgaConfig {
    Ice40Part part;
    ClbEntry *entries;
    size_t len, cap;
};

FpgaConfig *fpga_config_new(Ice40Part part) {
    FpgaConfig *c = calloc(1, sizeof *c);
    if (!c) return NULL;
    c->part = part;
    return c;
}

void fpga_config_free(FpgaConfig *c) {
    if (!c) return;
    free(c->entries);
    free(c);
}

int fpga_config_insert_clb(FpgaConfig *c, uint32_t row, uint32_t col,
                           const FpgaClbConfig *clb) {
    /* HashMap semantics: an existing key is overwritten, not duplicated. */
    for (size_t i = 0; i < c->len; i++) {
        if (c->entries[i].row == row && c->entries[i].col == col) {
            c->entries[i].clb = *clb;
            return 0;
        }
    }
    if (c->len == c->cap) {
        size_t nc = c->cap ? c->cap * 2 : 4;
        if (c->cap > ((size_t)-1) / 2 / sizeof(ClbEntry)) return -1;
        ClbEntry *ne = realloc(c->entries, nc * sizeof(ClbEntry));
        if (!ne) return -1;
        c->entries = ne;
        c->cap = nc;
    }
    c->entries[c->len].row = row;
    c->entries[c->len].col = col;
    c->entries[c->len].clb = *clb;
    c->len++;
    return 0;
}

size_t fpga_config_clb_count(const FpgaConfig *c) { return c->len; }

/* ===========================================================================
 *  Growable byte buffer
 * =========================================================================== */

typedef struct {
    uint8_t *data;
    size_t len, cap;
    int err;
} ByteBuf;

static void bb_reserve(ByteBuf *b, size_t extra) {
    if (b->err) return;
    if (extra > (size_t)-1 - b->len) {
        b->err = 1;
        return;
    }
    size_t need = b->len + extra;
    if (need <= b->cap) return;
    size_t cap = b->cap ? b->cap : 32;
    while (cap < need) {
        if (cap > ((size_t)-1) / 2) {
            cap = need;
            break;
        }
        cap *= 2;
    }
    uint8_t *nd = realloc(b->data, cap);
    if (!nd) {
        b->err = 1;
        return;
    }
    b->data = nd;
    b->cap = cap;
}

static void bb_push(ByteBuf *b, uint8_t byte) {
    bb_reserve(b, 1);
    if (b->err) return;
    b->data[b->len++] = byte;
}

static void bb_push_bytes(ByteBuf *b, const uint8_t *bytes, size_t n) {
    bb_reserve(b, n);
    if (b->err) return;
    memcpy(b->data + b->len, bytes, n);
    b->len += n;
}

static void bb_push_zeros(ByteBuf *b, size_t n) {
    bb_reserve(b, n);
    if (b->err) return;
    memset(b->data + b->len, 0, n);
    b->len += n;
}

/* Append one command record `[len, command, payload…]` to the buffer. Sets the
 * error flag if the payload exceeds 253 bytes (the Rust panic). */
static void bb_cmd(ByteBuf *b, uint8_t command, const uint8_t *payload,
                   size_t payload_len) {
    if (payload_len > 253) {
        b->err = 1;
        return;
    }
    bb_push(b, (uint8_t)(payload_len + 2));
    bb_push(b, command);
    if (payload_len) bb_push_bytes(b, payload, payload_len);
}

/* A command record whose payload is `zeros` zero bytes (avoids a temporary). */
static void bb_cmd_zeros(ByteBuf *b, uint8_t command, size_t zeros) {
    if (zeros > 253) {
        b->err = 1;
        return;
    }
    bb_push(b, (uint8_t)(zeros + 2));
    bb_push(b, command);
    bb_push_zeros(b, zeros);
}

/* ===========================================================================
 *  cmd (public helper)
 * =========================================================================== */

uint8_t *fpga_cmd(uint8_t command, const uint8_t *payload, size_t payload_len,
                  size_t *out_len) {
    if (payload_len > 253) return NULL; /* the Rust panic */
    size_t total = payload_len + 2;
    uint8_t *rec = malloc(total);
    if (!rec) return NULL;
    rec[0] = (uint8_t)total;
    rec[1] = command;
    if (payload_len) memcpy(rec + 2, payload, payload_len);
    *out_len = total;
    return rec;
}

/* ===========================================================================
 *  emit_bitstream
 * =========================================================================== */

static int by_row_col(const void *a, const void *b) {
    const ClbEntry *x = a, *y = b;
    if (x->row != y->row) return x->row < y->row ? -1 : 1;
    if (x->col != y->col) return x->col < y->col ? -1 : 1;
    return 0;
}

uint8_t *fpga_emit_bitstream(const FpgaConfig *c, size_t *len_out,
                             FpgaBitstreamReport *report) {
    uint32_t rows, cols, cram_bits;
    fpga_part_specs(c->part, &rows, &cols, &cram_bits);
    (void)rows;
    (void)cols;
    size_t cram_bytes = ((size_t)cram_bits + 7) / 8; /* div_ceil(8) */

    /* Sort a copy of the CLBs by (row, col) for a deterministic stream. */
    ClbEntry *sorted = NULL;
    if (c->len) {
        if (c->len > ((size_t)-1) / sizeof(ClbEntry)) return NULL; /* guard first */
        sorted = malloc(c->len * sizeof(ClbEntry));
        if (!sorted) return NULL;
        memcpy(sorted, c->entries, c->len * sizeof(ClbEntry));
        qsort(sorted, c->len, sizeof(ClbEntry), by_row_col);
    }

    ByteBuf b = {NULL, 0, 0, 0};

    /* Preamble: two raw magic bytes. */
    bb_push(&b, 0xFF);
    bb_push(&b, 0x00);
    /* CRAM bank reset, then bank 0 select. */
    bb_cmd(&b, CMD_CRAM_RESET, NULL, 0);
    uint8_t bank0 = 0x00;
    bb_cmd(&b, CMD_CRAM_BANK, &bank0, 1);

    /* Per-CLB tile records. */
    for (size_t i = 0; i < c->len; i++) {
        uint32_t row = sorted[i].row, col = sorted[i].col;
        uint8_t offset_payload[4] = {(uint8_t)(row >> 8), (uint8_t)row,
                                     (uint8_t)(col >> 8), (uint8_t)col};
        bb_cmd(&b, CMD_CRAM_OFFSET, offset_payload, 4);
        bb_cmd_zeros(&b, CMD_BRAM_DATA, cram_bytes);
    }

    /* CRC placeholder, then the raw end marker. */
    uint8_t crc[2] = {0x00, 0x00};
    bb_cmd(&b, CMD_CRC, crc, 2);
    bb_push(&b, 0xFF);
    bb_push(&b, 0xFF);

    free(sorted);

    if (b.err) {
        free(b.data);
        return NULL;
    }
    report->part = c->part;
    report->bytes_written = b.len;
    report->clb_count = c->len;
    report->cram_size = cram_bytes;
    *len_out = b.len;
    return b.data;
}

int fpga_write_bin(const char *path, const FpgaConfig *c,
                   FpgaBitstreamReport *report) {
    size_t len = 0;
    uint8_t *data = fpga_emit_bitstream(c, &len, report);
    if (!data) return -1;
    FILE *f = fopen(path, "wb");
    if (!f) {
        free(data);
        return -1;
    }
    size_t written = fwrite(data, 1, len, f);
    int ok = (written == len);
    if (fclose(f) != 0) ok = 0;
    free(data);
    return ok ? 0 : -1;
}
