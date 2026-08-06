/*
 * zstd.c — implementation of ZStd (RFC 8878 educational subset). See zstd.h
 * for the frame layout, the supported-feature subset, and — importantly —
 * the warning about the FSE-codec bug class this file was written to avoid.
 * This is a line-by-line transcription of the algorithm validated in
 * `code/packages/rust/zstd/src/lib.rs` against the real `zstd` CLI.
 */
#include "zstd.h"

#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, memset */

#include "lzss.h" /* LzssToken, lzss_encode — the LZ77 match-finder (CMP02) */

/* ─── Constants ──────────────────────────────────────────────────────────── */

/* ZStd magic number 0xFD2FB528, written little-endian as bytes 28 B5 2F FD.
 * Every valid ZStd frame starts with these 4 bytes. */
static const uint8_t ZSTD_MAGIC[4] = {0x28, 0xB5, 0x2F, 0xFD};

/* Maximum block size: 128 KB = 1<<17. RFC 8878 allows blocks up to
 * min(Window_Size, 128 KB); this port fixes Window_Size at 8 MB (well above
 * 128 KB) so 128 KB is always the binding limit. Larger inputs are split
 * across multiple blocks by `zstd_compress`; on decode, any block claiming a
 * larger size is rejected outright (a "malformed frame" defense — see
 * zstd.h's security notes). */
#define ZSTD_MAX_BLOCK_SIZE ((size_t)128 * 1024)

/* Decompression-bomb guard: maximum total decompressed output size.
 *
 * A Compressed block's WIRE size is capped at ZSTD_MAX_BLOCK_SIZE, but that
 * says nothing about how large it can EXPAND to: a single FSE-coded sequence
 * can have a match length up to ~131 KB (ML code 52), and one 128 KB block
 * can carry tens of thousands of sequences. This is therefore checked
 * INCREMENTALLY — once per literal run and once per match copy inside
 * decompress_block(), not merely once per top-level Raw/RLE block. */
#define ZSTD_MAX_OUTPUT ((size_t)256 * 1024 * 1024)

/* zstd_check_budget — true if growing the output by `additional` bytes (from
 * `current_size` bytes) would stay within ZSTD_MAX_OUTPUT.
 *
 * Written as a subtraction-then-compare (rather than an addition-then-
 * compare) specifically to avoid size_t overflow: `current_size` is always
 * <= ZSTD_MAX_OUTPUT by the loop invariant that every growth is gated by
 * this same check, so `ZSTD_MAX_OUTPUT - current_size` can never underflow. */
static int zstd_check_budget(size_t current_size, size_t additional) {
    if (current_size > ZSTD_MAX_OUTPUT) {
        return 0;
    }
    return additional <= ZSTD_MAX_OUTPUT - current_size;
}

/* zstd_floor_log2_u32 — floor(log2(x)) for x >= 1, computed by a portable
 * shift-and-count loop (no compiler builtins like __builtin_clz — those are
 * extensions this pure-ISO-C package deliberately avoids). Used both for the
 * FSE decode-table's per-slot bit-count derivation and for the offset code
 * number (RFC 8878 §3.1.1.3.2.1: code = floor(log2(offset + 3))). */
static unsigned zstd_floor_log2_u32(uint32_t x) {
    unsigned n = 0;
    while (x > 1) {
        x >>= 1;
        n++;
    }
    return n;
}

/* ─── Growable byte buffer ───────────────────────────────────────────────── */
/* Same pattern as `c/lzss`'s ByteBuf: a doubling-capacity buffer with an
 * `ok` flag that latches to false on the first allocation failure, so every
 * subsequent push becomes a silent no-op instead of a crash — callers check
 * `ok` once at the end rather than after every single push. */

typedef struct {
    unsigned char *data;
    size_t len, cap;
    int ok;
} ByteBuf;

static void bb_init(ByteBuf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->ok = 1;
}

static int bb_reserve(ByteBuf *b, size_t extra) {
    size_t need, nc;
    unsigned char *nd;
    if (!b->ok) {
        return 0;
    }
    if (extra > (size_t)-1 - b->len) {
        b->ok = 0;
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) {
        return 1;
    }
    nc = b->cap ? b->cap : 32;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    nd = (unsigned char *)realloc(b->data, nc);
    if (!nd) {
        b->ok = 0;
        return 0;
    }
    b->data = nd;
    b->cap = nc;
    return 1;
}

static void bb_push(ByteBuf *b, unsigned char c) {
    if (bb_reserve(b, 1)) {
        b->data[b->len++] = c;
    }
}

/* bb_push_n — append `n` bytes in one memcpy (used for literal runs, raw
 * blocks, and RLE runs — anywhere a byte-at-a-time bb_push loop would be
 * both slower and noisier to read). `src` may be NULL when n == 0. */
static void bb_push_n(ByteBuf *b, const unsigned char *src, size_t n) {
    if (n == 0) {
        return;
    }
    if (bb_reserve(b, n)) {
        memcpy(b->data + b->len, src, n);
        b->len += n;
    }
}

static void bb_push_rle(ByteBuf *b, unsigned char byte, size_t n) {
    if (n == 0) {
        return;
    }
    if (bb_reserve(b, n)) {
        memset(b->data + b->len, byte, n);
        b->len += n;
    }
}

/* ─── LL / ML code tables (RFC 8878 §3.1.1.3) ───────────────────────────────
 *
 * These map a *code number* to a (baseline, extra_bits) pair: the decoded
 * value is `baseline + read(extra_bits)`. For example LL code 17 means
 * literal_length = 18 + read(1 extra bit) — i.e. it covers lengths 18..19.
 * Offset codes need no such table: their baseline is always `1 << code`
 * (computed directly, see the sequences-section code below) rather than a
 * lookup, since offsets are unbounded (no fixed maximum code count).
 */

typedef struct {
    uint32_t baseline;
    uint8_t extra_bits;
} ZstdCode;

/* Literal Length code table: codes 0..35. Codes 0..15 are literal lengths
 * 0..15 verbatim (0 extra bits); codes 16+ cover exponentially wider ranges. */
static const ZstdCode LL_CODES[36] = {
    {0, 0},   {1, 0},   {2, 0},   {3, 0},   {4, 0},   {5, 0},
    {6, 0},   {7, 0},   {8, 0},   {9, 0},   {10, 0},  {11, 0},
    {12, 0},  {13, 0},  {14, 0},  {15, 0},
    {16, 1},  {18, 1},  {20, 1},  {22, 1},
    {24, 2},  {28, 2},
    {32, 3},  {40, 3},
    {48, 4},  {64, 6},
    {128, 7}, {256, 8}, {512, 9}, {1024, 10}, {2048, 11}, {4096, 12},
    {8192, 13}, {16384, 14}, {32768, 15}, {65536, 16},
};

/* Match Length code table: codes 0..52. ZStd's minimum match length is 3
 * (not 0) — code 0 means match length 3. */
static const ZstdCode ML_CODES[53] = {
    {3, 0},   {4, 0},   {5, 0},   {6, 0},   {7, 0},   {8, 0},
    {9, 0},   {10, 0},  {11, 0},  {12, 0},  {13, 0},  {14, 0},
    {15, 0},  {16, 0},  {17, 0},  {18, 0},  {19, 0},  {20, 0},
    {21, 0},  {22, 0},  {23, 0},  {24, 0},  {25, 0},  {26, 0},
    {27, 0},  {28, 0},  {29, 0},  {30, 0},  {31, 0},  {32, 0},
    {33, 0},  {34, 0},
    {35, 1},  {37, 1},  {39, 1},  {41, 1},
    {43, 2},  {47, 2},
    {51, 3},  {59, 3},
    {67, 4},  {83, 4},
    {99, 5},  {131, 7},
    {259, 8}, {515, 9}, {1027, 10}, {2051, 11},
    {4099, 12}, {8195, 13}, {16387, 14}, {32771, 15}, {65539, 16},
};

/* ll_to_code / ml_to_code — map a literal/match length to its code number
 * via a linear scan (tables are tiny; a binary search would not be worth
 * the extra code). Codes are listed in increasing baseline order, so the
 * LAST code whose baseline <= value is the right one. */
static size_t ll_to_code(uint32_t ll) {
    size_t code = 0, i;
    for (i = 0; i < 36; i++) {
        if (LL_CODES[i].baseline <= ll) {
            code = i;
        } else {
            break;
        }
    }
    return code;
}

static size_t ml_to_code(uint32_t ml) {
    size_t code = 0, i;
    for (i = 0; i < 53; i++) {
        if (ML_CODES[i].baseline <= ml) {
            code = i;
        } else {
            break;
        }
    }
    return code;
}

/* ─── FSE predefined distributions (RFC 8878 Appendix B) ────────────────────
 *
 * "Predefined_Mode" means no per-frame table description is transmitted —
 * both encoder and decoder build the SAME table from these fixed constants.
 * A value of -1 means "probability 1/table_size" (the rarest possible
 * non-zero probability): that symbol gets exactly one slot in the decode
 * table, and its encoder state transition always costs the full
 * accuracy-log number of bits (see build_encode_sym's `cnt == 1` branch,
 * which -1 symbols share with genuine count-1 symbols).
 */

static const int16_t LL_NORM[36] = {
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
    2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
};
#define LL_ACC_LOG 6u /* table_size = 64 */

static const int16_t ML_NORM[53] = {
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
    -1, -1, -1, -1, -1,
};
#define ML_ACC_LOG 6u

static const int16_t OF_NORM[29] = {
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
};
#define OF_ACC_LOG 5u /* table_size = 32 */

/* ─── FSE decode / encode table entries ──────────────────────────────────── */

/* FseDe — one cell of an FSE decode table. To decode a symbol from state S:
 *   1. `sym` is the output symbol (a bare lookup — costs no bits).
 *   2. Read `nb` bits from the bitstream.
 *   3. New state = `base + those bits`. */
typedef struct {
    uint8_t sym;
    uint8_t nb;
    uint16_t base;
} FseDe;

/* FseEe — the encode-side transform for one symbol. Given encoder state S:
 *   nb_out = (S + delta_nb) >> 16        (bits to flush; note the addition
 *                                          is uint32_t arithmetic, so it
 *                                          WRAPS — this is intentional, see
 *                                          build_encode_sym)
 *   emit the low nb_out bits of S
 *   new_S = state_table[(S >> nb_out) + delta_fs] */
typedef struct {
    uint32_t delta_nb;
    int32_t delta_fs; /* signed: cumul[sym] - count can be negative */
} FseEe;

/* build_decode_table — build an FSE decode table from a normalised
 * probability distribution `norm` (`norm_len` symbols) at `acc_log` bits of
 * accuracy (table size = 1 << acc_log). Caller supplies `tbl`, sized to
 * exactly `1 << acc_log` entries.
 *
 * Algorithm (RFC 8878 §4.1, cross-checked against the real zstd C reference
 * `FSE_buildDTable_internal` in `lib/decompress/fse_decompress.c`):
 *   1. Symbols with probability -1 (the rarest) go at the TOP of the table
 *      (highest indices), one slot each.
 *   2. Remaining symbols are spread into the lower portion by a SINGLE
 *      ascending pass over `0..norm_len`, placing each symbol's full count
 *      immediately when encountered, walking the table with a fixed step
 *      `(sz>>1)+(sz>>3)+3` (co-prime to any power-of-two sz, so the walk
 *      visits every free slot exactly once).
 *   3. Each table cell is assigned `nb` (bits to read) and `base` (the
 *      addend) so that `base + read(nb)` lands in `[sz, 2*sz)` — the valid
 *      encoder state range.
 *
 * THE BUG THIS AVOIDS: an earlier, unvalidated version of this same port (in
 * several other languages — see zstd.h's warning and lessons.md Lesson 96)
 * used a FABRICATED two-pass spread ("all symbols with count > 1 first, in
 * ascending order, then all symbols with count == 1") instead of step 2's
 * real single pass. That produces a different but internally self-consistent
 * table — invisible to any test that only round-trips through this same
 * codec's own encoder/decoder, since both sides agreed on the same wrong
 * layout. It was caught only by decompressing real `zstd` CLI output.
 */
static void build_decode_table(const int16_t *norm, size_t norm_len,
                                unsigned acc_log, FseDe *tbl) {
    size_t sz = (size_t)1 << acc_log;
    size_t step = (sz >> 1) + (sz >> 3) + 3;
    uint16_t sym_next[64]; /* indexed by symbol; 64 covers the largest
                              norm_len used here (ML_CODES, 53) */
    size_t high, s, pos, i;

    for (i = 0; i < sz; i++) {
        tbl[i].sym = 0;
        tbl[i].nb = 0;
        tbl[i].base = 0;
    }
    for (s = 0; s < norm_len; s++) {
        sym_next[s] = 0;
    }

    /* Phase 1: probability -1 symbols at the high end, one slot each. */
    high = sz - 1;
    for (s = 0; s < norm_len; s++) {
        if (norm[s] == -1) {
            tbl[high].sym = (uint8_t)s;
            sym_next[s] = 1;
            if (high > 0) {
                high--;
            }
        }
    }

    /* Phase 2: single ascending pass, each symbol's full count placed
     * immediately — see the doc comment above for why this must NOT be
     * split into a two-pass "count>1 then count==1" walk. */
    pos = 0;
    for (s = 0; s < norm_len; s++) {
        int16_t c = norm[s];
        size_t cnt, k;
        if (c <= 0) {
            continue;
        }
        cnt = (size_t)c;
        sym_next[s] = (uint16_t)cnt;
        for (k = 0; k < cnt; k++) {
            tbl[pos].sym = (uint8_t)s;
            pos = (pos + step) & (sz - 1);
            while (pos > high) {
                pos = (pos + step) & (sz - 1);
            }
        }
    }

    /* Phase 3: assign (nb, base) per cell. For a symbol occupying its j-th
     * cell (in ascending table-index order), the "next state" counter `ns`
     * starts at `count` and increments once per occurrence:
     *   nb   = acc_log - floor(log2(ns))
     *   base = (ns << nb) - sz
     * which guarantees `base + read(nb)` lands in [sz, 2*sz). */
    {
        uint16_t sn[64];
        for (s = 0; s < norm_len; s++) {
            sn[s] = sym_next[s];
        }
        for (i = 0; i < sz; i++) {
            size_t sym = tbl[i].sym;
            uint32_t ns = sn[sym];
            unsigned nb;
            uint32_t base;
            sn[sym] = (uint16_t)(sn[sym] + 1);
            nb = acc_log - zstd_floor_log2_u32(ns);
            base = (ns << nb) - (uint32_t)sz;
            tbl[i].nb = (uint8_t)nb;
            tbl[i].base = (uint16_t)base;
        }
    }
}

/* build_encode_sym — build the FSE encode-side tables from the same
 * normalised distribution: `ee` (indexed by symbol, size `norm_len`) and
 * `st` (the state table, size `1 << acc_log`).
 *
 * The decoder assigns (sym, nb, base) to each table cell in ASCENDING INDEX
 * ORDER (build_decode_table's Phase 3). The encoder must mirror that exactly:
 * for symbol s, its j-th occurrence (ascending index order) maps encode slot
 * `cumul[s] + j` to encoder output state `(that table index) + sz` — so that
 * a decoder landing on that table index reconstructs the same symbol via the
 * same bits the encoder just wrote.
 */
static void build_encode_sym(const int16_t *norm, size_t norm_len,
                              unsigned acc_log, FseEe *ee, uint16_t *st) {
    uint32_t sz = (uint32_t)1 << acc_log;
    uint32_t cumul[64];
    uint32_t total = 0;
    size_t step = ((size_t)sz >> 1) + ((size_t)sz >> 3) + 3;
    uint8_t spread[64]; /* spread[table_index] = symbol occupying that cell */
    size_t idx_high, idx_limit, pos, s, i;
    uint32_t sym_occ[64];

    for (s = 0; s < norm_len; s++) {
        int16_t c = norm[s];
        uint32_t cnt = (c == -1) ? 1u : (uint32_t)(c > 0 ? c : 0);
        cumul[s] = total;
        total += cnt;
    }

    /* Phase 1: probability -1 symbols at the high end (mirrors
     * build_decode_table's Phase 1 exactly — same step function, same
     * high-to-low fill order — so the two tables stay in lockstep). */
    idx_high = sz - 1;
    for (s = 0; s < norm_len; s++) {
        if (norm[s] == -1) {
            spread[idx_high] = (uint8_t)s;
            if (idx_high > 0) {
                idx_high--;
            }
        }
    }
    idx_limit = idx_high;

    /* Phase 2: single ascending pass — MUST mirror build_decode_table's
     * Phase 2 exactly (see that function's doc comment on the two-pass bug
     * this avoids). */
    pos = 0;
    for (s = 0; s < norm_len; s++) {
        int16_t c = norm[s];
        size_t cnt, k;
        if (c <= 0) {
            continue;
        }
        cnt = (size_t)c;
        for (k = 0; k < cnt; k++) {
            spread[pos] = (uint8_t)s;
            pos = (pos + step) & ((size_t)sz - 1);
            while (pos > idx_limit) {
                pos = (pos + step) & ((size_t)sz - 1);
            }
        }
    }

    /* Phase 3: walk `spread` in ascending index order, tracking how many
     * times each symbol has been seen so far (`sym_occ`) to compute its
     * encode slot `cumul[sym] + occurrence`. */
    for (s = 0; s < norm_len; s++) {
        sym_occ[s] = 0;
    }
    for (i = 0; i < (size_t)sz; i++) {
        size_t sym = spread[i];
        uint32_t j = sym_occ[sym]++;
        size_t slot = (size_t)cumul[sym] + j;
        st[slot] = (uint16_t)(i + sz);
    }

    /* Phase 4: derive (delta_nb, delta_fs) per symbol.
     *   max_bits_out = acc_log - floor(log2(count))   (count==1 -> acc_log)
     *   delta_nb = (max_bits_out << 16) - (count << max_bits_out)
     *   delta_fs = cumul[sym] - count
     * These let the hot encode loop (fse_encode_sym) work from pure
     * arithmetic + one table lookup — no branching on the symbol's
     * probability class at encode time. */
    for (s = 0; s < norm_len; s++) {
        ee[s].delta_nb = 0;
        ee[s].delta_fs = 0;
    }
    for (s = 0; s < norm_len; s++) {
        int16_t c = norm[s];
        uint32_t cnt = (c == -1) ? 1u : (uint32_t)(c > 0 ? c : 0);
        uint32_t mbo;
        if (cnt == 0) {
            continue;
        }
        mbo = (cnt == 1) ? acc_log : (acc_log - zstd_floor_log2_u32(cnt));
        ee[s].delta_nb = (mbo << 16) - (cnt << mbo);
        ee[s].delta_fs = (int32_t)cumul[s] - (int32_t)cnt;
    }
}

/* ─── Reverse bit-writer / reader ────────────────────────────────────────── *
 *
 * ZStd's sequence bitstream is written BACKWARDS relative to decode order:
 * the encoder emits, first, the bits a forward-reading decoder will need
 * LAST. The buffer's final byte carries a sentinel bit (the highest set bit)
 * marking where meaningful data ends; the reader locates it and reads
 * backward from there toward byte 0, so a decoder walking the stream
 * top-to-bottom sees bits in the SAME order the encoder logically produced
 * them despite writing the buffer back-to-front.
 *
 * Bit-writer: an accumulation register fills from the LSB side; whenever 8
 * bits are buffered they are flushed as one byte (byte[0] = earliest
 * written). Bit-reader: mirrors this with a LEFT-aligned register — the top
 * `bits` bits are valid — so `read_bits(n)` always returns the MOST recently
 * written n bits first, shifting them out of the register's top.
 */

typedef struct {
    ByteBuf buf;
    uint64_t reg;
    unsigned bits;
} RevBitWriter;

static void rbw_init(RevBitWriter *w) {
    bb_init(&w->buf);
    w->reg = 0;
    w->bits = 0;
}

static void rbw_add_bits(RevBitWriter *w, uint64_t val, unsigned nb) {
    uint64_t mask;
    if (nb == 0) {
        return;
    }
    mask = (nb >= 64) ? (uint64_t)-1 : (((uint64_t)1 << nb) - 1);
    w->reg |= (val & mask) << w->bits;
    w->bits += nb;
    while (w->bits >= 8) {
        bb_push(&w->buf, (unsigned char)(w->reg & 0xFFu));
        w->reg >>= 8;
        w->bits -= 8;
    }
}

/* rbw_flush — write the final partial byte with its sentinel bit, then reset
 * (the writer is single-use per bitstream: one flush per encode_sequences_
 * section call). */
static void rbw_flush(RevBitWriter *w) {
    unsigned char sentinel = (unsigned char)(1u << w->bits);
    unsigned char last_byte = (unsigned char)((w->reg & 0xFFu) | sentinel);
    bb_push(&w->buf, last_byte);
    w->reg = 0;
    w->bits = 0;
}

typedef struct {
    const uint8_t *data;
    size_t len;
    uint64_t reg;   /* valid bits packed at the TOP (MSB side) */
    unsigned bits;  /* how many valid bits are currently loaded */
    size_t pos;     /* index of the next byte to load, walking toward 0 */
} RevBitReader;

static void rbr_reload(RevBitReader *r) {
    while (r->bits <= 56 && r->pos > 0) {
        unsigned shift;
        r->pos -= 1;
        shift = 64 - r->bits - 8;
        r->reg |= ((uint64_t)r->data[r->pos]) << shift;
        r->bits += 8;
    }
}

/* rbr_init — locate the sentinel bit in the last byte and prime the reader.
 * Returns 0 (and leaves *r unusable) if `data` is empty or its last byte is
 * zero (no sentinel — a malformed/truncated bitstream). */
static int rbr_init(RevBitReader *r, const uint8_t *data, size_t len) {
    uint8_t last;
    unsigned sentinel_pos, valid_bits;
    uint64_t mask;

    if (len == 0) {
        return 0;
    }
    last = data[len - 1];
    if (last == 0) {
        return 0;
    }

    /* sentinel_pos = index of the highest set bit in `last` (0 = LSB). */
    sentinel_pos = 0;
    {
        uint8_t t = last;
        while (t > 1) {
            t = (uint8_t)(t >> 1);
            sentinel_pos++;
        }
    }
    valid_bits = sentinel_pos;

    mask = (valid_bits == 0) ? 0 : (((uint64_t)1 << valid_bits) - 1);
    r->data = data;
    r->len = len;
    r->reg = (valid_bits == 0) ? 0
                                : (((uint64_t)(last & mask)) << (64 - valid_bits));
    r->bits = valid_bits;
    r->pos = len - 1; /* the sentinel byte itself is already consumed */

    rbr_reload(r);
    return 1;
}

static uint64_t rbr_read_bits(RevBitReader *r, unsigned nb) {
    uint64_t val;
    if (nb == 0) {
        return 0;
    }
    val = r->reg >> (64 - nb);
    r->reg = (nb >= 64) ? 0 : (r->reg << nb);
    r->bits = (r->bits > nb) ? (r->bits - nb) : 0;
    if (r->bits < 24) {
        rbr_reload(r);
    }
    return val;
}

/* ─── FSE encode/decode step helpers ─────────────────────────────────────── */

/* fse_encode_sym — emit symbol `sym`'s state-transition bits and advance
 * `*state`. The `+` on uint32_t below is C's normal unsigned arithmetic,
 * which wraps modulo 2^32 by definition — exactly the "wrapping_add"
 * semantics the reference algorithm requires (delta_nb is constructed so
 * that its wraparound behavior, when it occurs, is meaningful, not a bug). */
static void fse_encode_sym(uint32_t *state, uint8_t sym, const FseEe *ee,
                            const uint16_t *st, RevBitWriter *bw) {
    const FseEe *e = &ee[sym];
    uint32_t nb32 = (*state + e->delta_nb) >> 16;
    uint8_t nb = (uint8_t)nb32;
    int64_t slot_i;
    size_t slot;

    rbw_add_bits(bw, (uint64_t)*state, nb);
    slot_i = (int64_t)(*state >> nb) + (int64_t)e->delta_fs;
    slot = (slot_i > 0) ? (size_t)slot_i : 0;
    *state = (uint32_t)st[slot];
}

/* fse_init_state — compute an FSE encoder's STARTING state directly from a
 * symbol, writing NO bits at all. Mirrors the real zstd reference's
 * `FSE_initCState2`.
 *
 * WHY THIS EXISTS: RFC 8878's decoder never performs a state-update read
 * after the LAST sequence in a block (there's no "next" sequence whose peek
 * needs a freshly-prepared state) — see decompress_block's `i != n_seqs - 1`
 * guard below. Symmetrically, the ENCODER's reverse loop processes that same
 * last sequence FIRST (see encode_sequences_section) and so has no incoming
 * transition to flush for it — the corresponding decode-side bit-consuming
 * read simply does not exist. An earlier, unvalidated version of this port
 * (see zstd.h's warning) always flushed a transition for every sequence
 * uniformly, writing bits a real decoder never reads and silently shifting
 * the bit-alignment of everything that follows — invisible to a codec
 * testing only against itself. See lessons.md Lesson 96.
 *
 * The widen-to-uint64_t before adding `1 << 15` mirrors the validated Rust
 * reference precisely (avoiding any question of 32-bit-vs-64-bit
 * intermediate wraparound in the addition, even though for THIS package's
 * fixed predefined tables delta_nb never comes close to needing it). */
static uint32_t fse_init_state(uint8_t sym, const FseEe *ee,
                                const uint16_t *st) {
    const FseEe *e = &ee[sym];
    uint64_t delta_nb = (uint64_t)e->delta_nb;
    uint64_t nb_bits_out = (delta_nb + ((uint64_t)1 << 15)) >> 16;
    uint64_t value = (nb_bits_out << 16) - delta_nb;
    int64_t slot_i = (int64_t)(value >> nb_bits_out) + (int64_t)e->delta_fs;
    size_t slot = (slot_i > 0) ? (size_t)slot_i : 0;
    return (uint32_t)st[slot];
}

/* fse_peek — read the symbol at the CURRENT decode state without consuming
 * any bits. The FSE state itself IS the decode-table index, so this is a
 * bare lookup; only fse_update_state (below) reads bits. RFC 8878
 * §3.1.1.3.2.1.2 requires all three fields (LL, ML, OF) to be peeked from
 * their current states BEFORE any extra bits or state updates happen — see
 * decompress_block. */
static FseDe fse_peek(uint16_t state, const FseDe *de) { return de[state]; }

/* fse_update_state — consume `entry.nb` bits and compute the next decode
 * state. Per the reference decoder, this is SKIPPED for the last sequence
 * in a block (see decompress_block's guard) — calling it unconditionally
 * consumes bits that were never written by the encoder, corrupting every
 * read that follows. See lessons.md Lesson 96. */
static uint16_t fse_update_state(FseDe entry, RevBitReader *br) {
    return (uint16_t)(entry.base + (uint16_t)rbr_read_bits(br, entry.nb));
}

/* ─── Sequences ──────────────────────────────────────────────────────────── */

/* One ZStd sequence: emit `ll` literal bytes from the literals section, then
 * copy `ml` bytes from `off` positions back in the output. Trailing literals
 * after the last sequence have no corresponding sequence entry. */
typedef struct {
    uint32_t ll, ml, off;
} ZstdSeq;

typedef struct {
    ZstdSeq *data;
    size_t count, cap;
    int ok;
} SeqBuf;

static void sb_init(SeqBuf *s) {
    s->data = NULL;
    s->count = 0;
    s->cap = 0;
    s->ok = 1;
}

static void sb_push(SeqBuf *s, ZstdSeq seq) {
    if (!s->ok) {
        return;
    }
    if (s->count == s->cap) {
        size_t nc = s->cap ? s->cap * 2 : 16;
        ZstdSeq *nd;
        if (s->cap > ((size_t)-1 / sizeof(ZstdSeq)) / 2) {
            s->ok = 0;
            return;
        }
        nd = (ZstdSeq *)realloc(s->data, nc * sizeof *nd);
        if (!nd) {
            s->ok = 0;
            return;
        }
        s->data = nd;
        s->cap = nc;
    }
    s->data[s->count++] = seq;
}

/* tokens_to_seqs — convert `c/lzss`'s LZ77 token stream into a flat literals
 * buffer plus a sequence list. LZSS emits Literal(byte) / Match{offset,
 * length}; ZStd groups the run of literals immediately BEFORE each match
 * into that match's sequence. Any literals after the LAST match have no
 * sequence and stay in the literals buffer only. */
static void tokens_to_seqs(const LzssToken *tokens, size_t count,
                            ByteBuf *lits, SeqBuf *seqs) {
    uint32_t lit_run = 0;
    size_t t;
    for (t = 0; t < count; t++) {
        if (!tokens[t].is_match) {
            bb_push(lits, tokens[t].literal);
            lit_run++;
        } else {
            ZstdSeq seq;
            seq.ll = lit_run;
            seq.ml = tokens[t].length;
            seq.off = tokens[t].offset;
            sb_push(seqs, seq);
            lit_run = 0;
        }
        if (!lits->ok || !seqs->ok) {
            return;
        }
    }
}

/* ─── Literals section ───────────────────────────────────────────────────── *
 *
 * This port only ever emits Raw_Literals (type 0, no Huffman table) — the
 * simplest of the four literal encodings RFC 8878 defines. Header size
 * depends on the literal count:
 *   n <=   31 : 1-byte header = (n << 3) | 0b000
 *   n <= 4095 : 2-byte header = (n << 4) | 0b0100
 *   else      : 3-byte header = (n << 4) | 0b1100
 * (bits [1:0] = Literals_Block_Type = 00/Raw; bits [3:2] = Size_Format.)
 */

static void encode_literals_section(const uint8_t *lits, size_t n,
                                     ByteBuf *out) {
    if (n <= 31) {
        bb_push(out, (unsigned char)(n << 3));
    } else if (n <= 4095) {
        uint32_t hdr = ((uint32_t)n << 4) | 0x4u;
        bb_push(out, (unsigned char)(hdr & 0xFFu));
        bb_push(out, (unsigned char)((hdr >> 8) & 0xFFu));
    } else {
        uint32_t hdr = ((uint32_t)n << 4) | 0xCu;
        bb_push(out, (unsigned char)(hdr & 0xFFu));
        bb_push(out, (unsigned char)((hdr >> 8) & 0xFFu));
        bb_push(out, (unsigned char)((hdr >> 16) & 0xFFu));
    }
    bb_push_n(out, lits, n);
}

/* decode_literals_section — the mirror of encode_literals_section. Rejects
 * anything other than Raw_Literals (type != 0): this port's encoder never
 * emits Huffman-coded literals, so seeing one here means the frame came from
 * a different, fuller encoder — out of scope for this educational subset. */
static ZstdStatus decode_literals_section(const uint8_t *data, size_t len,
                                           ByteBuf *lits_out,
                                           size_t *consumed_out) {
    uint8_t b0, ltype, size_format;
    size_t n, header_bytes, start, end;

    if (len == 0) {
        return ZSTD_ERR_FORMAT;
    }
    b0 = data[0];
    ltype = (uint8_t)(b0 & 0x03u);
    if (ltype != 0) {
        return ZSTD_ERR_FORMAT;
    }

    size_format = (uint8_t)((b0 >> 2) & 0x03u);
    if (size_format == 0 || size_format == 2) {
        n = (size_t)(b0 >> 3);
        header_bytes = 1;
    } else if (size_format == 1) {
        if (len < 2) {
            return ZSTD_ERR_FORMAT;
        }
        n = ((size_t)(b0 >> 4)) | ((size_t)data[1] << 4);
        header_bytes = 2;
    } else {
        if (len < 3) {
            return ZSTD_ERR_FORMAT;
        }
        n = ((size_t)(b0 >> 4)) | ((size_t)data[1] << 4) |
            ((size_t)data[2] << 12);
        header_bytes = 3;
    }

    start = header_bytes;
    end = start + n;
    if (end < start || end > len) {
        return ZSTD_ERR_FORMAT;
    }

    bb_init(lits_out);
    bb_push_n(lits_out, data + start, n);
    if (!lits_out->ok) {
        return ZSTD_ERR_ALLOC;
    }

    *consumed_out = end;
    return ZSTD_OK;
}

/* ─── Sequences section ──────────────────────────────────────────────────── *
 *
 * Layout: [sequence_count: 1-3 bytes] [symbol_compression_modes: 1 byte,
 * always 0x00 = all Predefined] [FSE bitstream, backward-written].
 *
 * Field order (RFC 8878 §3.1.1.3.2.1.2, cross-checked against the real
 * `zstd` CLI via TC-9 and the reference C source `ZSTD_decodeSequence`):
 * a FORWARD-reading decoder processes each sequence as:
 *   1. PEEK all three symbols (LL, ML, OF) from their CURRENT states — a
 *      bare table lookup, no bits consumed.
 *   2. Read extra bits, in order OF, ML, LL.
 *   3. Update states (consumes bits), in order LL, ML, OF — preparing the
 *      states the NEXT sequence's peek will use. SKIPPED for the LAST
 *      sequence: there is no "next" sequence to prepare a state for.
 * The very FIRST states a forward decoder sees are read up front, in order
 * LL, OF, ML — a DIFFERENT order from the per-sequence update order above;
 * RFC 8878 is genuinely asymmetric here, not a copy-paste typo.
 *
 * Our encoder writes the mirror image, backwards, processing sequences in
 * REVERSE order (last real sequence first):
 *   - First-processed sequence (semantically the LAST real one): no
 *     incoming transition to flush — state computed directly via
 *     fse_init_state, writing NO bits.
 *   - Every other sequence: flush a transition, write order OF, ML, LL (a
 *     forward decoder reads this as update order LL, ML, OF, right after
 *     decoding the PREVIOUS — i.e. next-processed — sequence).
 *   - Then extra bits, write order LL, ML, OF (read as OF, ML, LL).
 *   - After all sequences: flush the initial states in write order ML, OF,
 *     LL, so a forward reader sees them in order LL, OF, ML.
 *
 * See zstd.h's top-of-file warning and lessons.md Lesson 96 for the bug
 * class this exact ordering was written to avoid.
 */

/* encode_seq_count — RFC 8878 §3.1.1.3.1's Number_of_Sequences field.
 *   0..127        : 1 byte  = count
 *   128..32511    : 2 bytes = (marker|high_byte), low_byte  -- MARKER FIRST
 *   32512+        : 3 bytes = 0xFF, then (count - 0x7F00) as little-endian
 *
 * The marker/high byte MUST come first on the wire regardless of host
 * endianness. A validated-against-the-real-CLI regression this port avoids:
 * writing the plain little-endian pair of (count | 0x8000) — i.e. low byte
 * first, marker+high byte SECOND — round-trips against ITSELF perfectly
 * (both sides of a self-consistent bug agree) but is not the real wire
 * format, and silently misparses under any independent RFC 8878 decoder
 * once a single compressed block carries 128+ sequences. */
static void encode_seq_count(size_t count, ByteBuf *out) {
    if (count < 128) {
        bb_push(out, (unsigned char)count);
    } else if (count < 0x7F00) {
        unsigned char hi = (unsigned char)(((count >> 8) & 0xFFu) | 0x80u);
        unsigned char lo = (unsigned char)(count & 0xFFu);
        bb_push(out, hi);
        bb_push(out, lo);
    } else {
        size_t r = count - 0x7F00;
        bb_push(out, 0xFFu);
        bb_push(out, (unsigned char)(r & 0xFFu));
        bb_push(out, (unsigned char)((r >> 8) & 0xFFu));
    }
}

static ZstdStatus decode_seq_count(const uint8_t *data, size_t len,
                                    size_t *count_out, size_t *consumed_out) {
    uint8_t b0;
    if (len < 1) {
        return ZSTD_ERR_FORMAT;
    }
    b0 = data[0];
    if (b0 < 128) {
        *count_out = b0;
        *consumed_out = 1;
        return ZSTD_OK;
    } else if (b0 < 0xFF) {
        if (len < 2) {
            return ZSTD_ERR_FORMAT;
        }
        *count_out = (((size_t)(b0 & 0x7Fu)) << 8) | (size_t)data[1];
        *consumed_out = 2;
        return ZSTD_OK;
    } else {
        if (len < 3) {
            return ZSTD_ERR_FORMAT;
        }
        *count_out = 0x7F00 + (size_t)data[1] + ((size_t)data[2] << 8);
        *consumed_out = 3;
        return ZSTD_OK;
    }
}

/* encode_sequences_section — encode `seqs` (must be non-empty; the caller,
 * compress_block, never calls this with an empty list) using the predefined
 * FSE tables. Returns 1 on success, 0 on allocation failure. */
static int encode_sequences_section(const ZstdSeq *seqs, size_t count,
                                     ByteBuf *out) {
    FseEe ee_ll[36], ee_ml[53], ee_of[29];
    uint16_t st_ll[64], st_ml[64], st_of[32];
    uint32_t sz_ll = (uint32_t)1 << LL_ACC_LOG;
    uint32_t sz_ml = (uint32_t)1 << ML_ACC_LOG;
    uint32_t sz_of = (uint32_t)1 << OF_ACC_LOG;
    uint32_t state_ll = sz_ll, state_ml = sz_ml, state_of = sz_of;
    RevBitWriter bw;
    int first = 1;
    size_t i;

    build_encode_sym(LL_NORM, 36, LL_ACC_LOG, ee_ll, st_ll);
    build_encode_sym(ML_NORM, 53, ML_ACC_LOG, ee_ml, st_ml);
    build_encode_sym(OF_NORM, 29, OF_ACC_LOG, ee_of, st_of);

    rbw_init(&bw);

    for (i = count; i-- > 0;) {
        const ZstdSeq *seq = &seqs[i];
        size_t ll_code = ll_to_code(seq->ll);
        size_t ml_code = ml_to_code(seq->ml);
        uint32_t raw_off = seq->off + 3;
        uint8_t of_code =
            (raw_off <= 1) ? 0 : (uint8_t)zstd_floor_log2_u32(raw_off);
        uint32_t of_extra = raw_off - ((uint32_t)1 << of_code);
        uint32_t ml_extra = seq->ml - ML_CODES[ml_code].baseline;
        uint32_t ll_extra = seq->ll - LL_CODES[ll_code].baseline;

        if (!first) {
            fse_encode_sym(&state_of, of_code, ee_of, st_of, &bw);
            fse_encode_sym(&state_ml, (uint8_t)ml_code, ee_ml, st_ml, &bw);
            fse_encode_sym(&state_ll, (uint8_t)ll_code, ee_ll, st_ll, &bw);
        } else {
            state_of = fse_init_state(of_code, ee_of, st_of);
            state_ml = fse_init_state((uint8_t)ml_code, ee_ml, st_ml);
            state_ll = fse_init_state((uint8_t)ll_code, ee_ll, st_ll);
            first = 0;
        }

        rbw_add_bits(&bw, ll_extra, LL_CODES[ll_code].extra_bits);
        rbw_add_bits(&bw, ml_extra, ML_CODES[ml_code].extra_bits);
        rbw_add_bits(&bw, of_extra, of_code);
    }

    rbw_add_bits(&bw, state_ml - sz_ml, ML_ACC_LOG);
    rbw_add_bits(&bw, state_of - sz_of, OF_ACC_LOG);
    rbw_add_bits(&bw, state_ll - sz_ll, LL_ACC_LOG);
    rbw_flush(&bw);

    if (!bw.buf.ok) {
        free(bw.buf.data);
        return 0;
    }
    bb_push_n(out, bw.buf.data, bw.buf.len);
    free(bw.buf.data);
    return out->ok;
}

/* ─── Block-level compress / decompress ──────────────────────────────────── */

/* compress_block — try to LZ77+FSE-compress one block.
 * Returns:
 *   1  — compressed output written to *out (caller owns *out->data);
 *   0  — not beneficial (no sequences found, or compressed >= original) —
 *        *out is untouched; caller should fall back to a Raw block;
 *  -1  — an allocation failed; caller must propagate ZSTD_ERR_ALLOC. */
static int compress_block(const uint8_t *block, size_t block_len,
                           ByteBuf *out) {
    LzssToken *tokens = NULL;
    size_t tok_count = 0;
    ByteBuf lits;
    SeqBuf seqs;
    int ok;

    /* Window 32 KB (bigger than LZSS's own 4 KB default, for a better
     * ratio), max match 255, min match 3 — mirrors the Rust reference. */
    if (lzss_encode(block, block_len, 32768, 255, 3, &tokens, &tok_count) !=
        LZSS_OK) {
        return -1;
    }

    bb_init(&lits);
    sb_init(&seqs);
    tokens_to_seqs(tokens, tok_count, &lits, &seqs);
    free(tokens);

    if (!lits.ok || !seqs.ok) {
        free(lits.data);
        free(seqs.data);
        return -1;
    }

    /* No LZ77 matches at all -> a compressed block would still carry
     * sequences-section overhead for zero benefit. Let the caller fall back
     * to Raw (or RLE, already tried before this is called). */
    if (seqs.count == 0) {
        free(lits.data);
        free(seqs.data);
        return 0;
    }

    bb_init(out);
    encode_literals_section(lits.data, lits.len, out);
    encode_seq_count(seqs.count, out);
    bb_push(out, 0x00);
    ok = encode_sequences_section(seqs.data, seqs.count, out);

    free(lits.data);
    free(seqs.data);

    if (!ok || !out->ok) {
        free(out->data);
        return -1;
    }
    if (out->len >= block_len) {
        free(out->data);
        out->data = NULL;
        out->len = 0;
        out->cap = 0;
        return 0;
    }
    return 1;
}

/* decompress_block — decode one Compressed block's payload into `out`
 * (append-only; `out` may already hold bytes from earlier blocks). See the
 * module doc comment above encode_seq_count for the exact field order this
 * mirrors, and zstd.h for the decompression-bomb / offset-bounds guards.
 *
 * `rep1`/`rep2`/`rep3` are the three Repeated_Offset registers (RFC 8878
 * §3.1.1.3.2.1.1) — IN/OUT, because they are FRAME-scoped, not block-scoped:
 * "For the first block, the starting offset history is populated with
 * Repeated_Offset1=1, Repeated_Offset2=4, Repeated_Offset3=8" (RFC 8878),
 * and every later block in the same frame continues from wherever the
 * previous Compressed block's sequences left them. The caller (
 * zstd_decompress) owns the three registers and threads them through every
 * Compressed block in a frame.
 *
 * WHY THIS PORT'S DECODER NEEDS THIS EVEN THOUGH ITS OWN ENCODER NEVER
 * EMITS REPEAT-OFFSET SEQUENCES: `encode_sequences_section` always writes
 * an explicit offset code (>= 2, since our minimum LZ77 match offset is 1,
 * giving raw_off = offset+3 >= 4), so this port's own compress()/
 * decompress() round trip never touches the repeat-offset path — the
 * "no repeat-offset shortcuts" simplification in zstd.h is entirely an
 * ENCODER-side choice. But the real `zstd` CLI's encoder uses repeat
 * offsets constantly (they are one of its main entropy wins, especially
 * for periodic/repetitive data), so a decoder that only understands
 * explicit offset codes will systematically fail TC-9's "compress with the
 * real CLI, decompress with ours" direction — caught here by fuzzing this
 * port against the real CLI across varied inputs (constant-byte data was
 * the first repro: 4713 bytes of the same byte compresses to a single
 * Compressed block whose one sequence has Offset_Value=1, i.e. "reuse
 * Repeated_Offset1", which starts at its default value of 1 — an
 * unmistakable RLE-via-repeat-offset pattern). Algorithm cross-checked
 * against both the RFC 8878 prose and the literal reference C source
 * (`ZSTD_decodeSequence` in `zstd_decompress_block.c`) per the Lesson-96
 * playbook of not trusting either alone. */
static ZstdStatus decompress_block(const uint8_t *data, size_t len,
                                    ByteBuf *out, uint32_t *rep1,
                                    uint32_t *rep2, uint32_t *rep3) {
    ByteBuf lits;
    size_t lit_consumed = 0, pos, n_seqs = 0, sc_bytes = 0;
    uint8_t modes_byte, ll_mode, of_mode, ml_mode;
    RevBitReader br;
    FseDe dt_ll[64], dt_ml[64], dt_of[32];
    uint16_t state_ll, state_ml, state_of;
    size_t lit_pos, i;
    ZstdStatus st;

    bb_init(&lits);
    st = decode_literals_section(data, len, &lits, &lit_consumed);
    if (st != ZSTD_OK) {
        free(lits.data);
        return st;
    }
    pos = lit_consumed;

    if (pos >= len) {
        /* Block has only literals, no sequences. */
        if (!zstd_check_budget(out->len, lits.len)) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }
        bb_push_n(out, lits.data, lits.len);
        free(lits.data);
        return out->ok ? ZSTD_OK : ZSTD_ERR_ALLOC;
    }

    st = decode_seq_count(data + pos, len - pos, &n_seqs, &sc_bytes);
    if (st != ZSTD_OK) {
        free(lits.data);
        return st;
    }
    pos += sc_bytes;

    if (n_seqs == 0) {
        if (!zstd_check_budget(out->len, lits.len)) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }
        bb_push_n(out, lits.data, lits.len);
        free(lits.data);
        return out->ok ? ZSTD_OK : ZSTD_ERR_ALLOC;
    }

    if (pos >= len) {
        free(lits.data);
        return ZSTD_ERR_FORMAT;
    }
    modes_byte = data[pos];
    pos += 1;
    ll_mode = (uint8_t)((modes_byte >> 6) & 3u);
    of_mode = (uint8_t)((modes_byte >> 4) & 3u);
    ml_mode = (uint8_t)((modes_byte >> 2) & 3u);
    if (ll_mode != 0 || of_mode != 0 || ml_mode != 0) {
        /* Only Predefined-mode tables are supported: this port never
         * builds an FSE table from untrusted wire bytes (RLE/FSE_Compressed
         * /Repeat modes are all out of scope for the educational subset). */
        free(lits.data);
        return ZSTD_ERR_FORMAT;
    }

    if (!rbr_init(&br, data + pos, len - pos)) {
        free(lits.data);
        return ZSTD_ERR_FORMAT;
    }

    build_decode_table(LL_NORM, 36, LL_ACC_LOG, dt_ll);
    build_decode_table(ML_NORM, 53, ML_ACC_LOG, dt_ml);
    build_decode_table(OF_NORM, 29, OF_ACC_LOG, dt_of);

    /* Initial states, read in order LL, OF, ML (see the module comment on
     * this asymmetry). */
    state_ll = (uint16_t)rbr_read_bits(&br, LL_ACC_LOG);
    state_of = (uint16_t)rbr_read_bits(&br, OF_ACC_LOG);
    state_ml = (uint16_t)rbr_read_bits(&br, ML_ACC_LOG);

    lit_pos = 0;

    for (i = 0; i < n_seqs; i++) {
        FseDe ll_entry, ml_entry, of_entry;
        uint8_t ll_code, ml_code, of_code;
        uint32_t of_raw, ml, ll, offset;
        int ll_is_zero;
        size_t lit_end, copy_start, j;

        /* Step 1: PEEK — bare lookups, no bits consumed. */
        ll_entry = fse_peek(state_ll, dt_ll);
        ml_entry = fse_peek(state_ml, dt_ml);
        of_entry = fse_peek(state_of, dt_of);
        ll_code = ll_entry.sym;
        ml_code = ml_entry.sym;
        of_code = of_entry.sym;

        if (ll_code >= 36 || ml_code >= 53) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }

        /* ll_is_zero is needed for the repeat-offset interpretation below
         * (RFC 8878's "when Literals_Length is 0, repeated offsets are
         * shifted by 1" rule) and is knowable right now, from the PEEKED
         * ll_code alone — LL code 0 is the only code with baseline 0 and 0
         * extra bits, so ll_code == 0 iff the eventual decoded `ll` value
         * is 0. No extra bits need to be read yet to know this. */
        ll_is_zero = (ll_code == 0);

        /* Step 2: extra bits, order OF, ML, LL. The NUMBER of bits read for
         * the offset field is always exactly `of_code` regardless of the
         * repeat-offset interpretation below (RFC 8878 / the reference
         * decoder never varies bit-consumption on ll_is_zero — only how
         * the resulting value maps to an actual offset changes). */
        of_raw = ((uint32_t)1 << of_code) |
                 (uint32_t)rbr_read_bits(&br, of_code);
        ml = ML_CODES[ml_code].baseline +
             (uint32_t)rbr_read_bits(&br, ML_CODES[ml_code].extra_bits);
        ll = LL_CODES[ll_code].baseline +
             (uint32_t)rbr_read_bits(&br, LL_CODES[ll_code].extra_bits);

        /* Offset_Value -> actual offset (RFC 8878 §3.1.1.3.2.1.1), including
         * the Repeated_Offset (R1/R2/R3) mechanism this port's DECODER must
         * understand even though its own ENCODER never emits it — see the
         * doc comment on decompress_block.
         *
         * of_code >= 2 guarantees of_raw = (1<<of_code)+extra >= 4, i.e.
         * Offset_Value > 3: an ordinary explicit offset. of_code <= 1
         * guarantees of_raw in {1, 2, 3}: a repeat-offset reference.
         *
         * The repeat case collapses to one selector in [0, 3]:
         *     selector = ll_is_zero + of_raw - 1
         * (derived from, and verified against, the reference decoder's
         * `ofBase + ll0 + extra_bit` — see decompress_block's doc comment):
         *   0 -> reuse rep1 unchanged (no rotation)
         *   1 -> use rep2 (rep1,rep2 swap; rep3 untouched)
         *   2 -> use rep3 (full rotate: rep1,rep2,rep3 <- new,old_rep1,old_rep2)
         *   3 -> use rep1-1 (full rotate, same shape as selector 2) */
        if (of_code >= 2) {
            offset = of_raw - 3;
            *rep3 = *rep2;
            *rep2 = *rep1;
            *rep1 = offset;
        } else {
            unsigned selector = (unsigned)ll_is_zero + of_raw - 1;
            switch (selector) {
                case 0:
                    offset = *rep1;
                    break;
                case 1:
                    offset = *rep2;
                    *rep2 = *rep1;
                    *rep1 = offset;
                    break;
                case 2:
                    offset = *rep3;
                    *rep3 = *rep2;
                    *rep2 = *rep1;
                    *rep1 = offset;
                    break;
                default: /* 3 */
                    offset = (*rep1 > 0) ? (*rep1 - 1) : 0;
                    *rep3 = *rep2;
                    *rep2 = *rep1;
                    *rep1 = offset;
                    break;
            }
        }

        /* Step 3: state updates, order LL, ML, OF — SKIPPED for the last
         * sequence (see fse_update_state's doc comment / Lesson 96). */
        if (i != n_seqs - 1) {
            state_ll = fse_update_state(ll_entry, &br);
            state_ml = fse_update_state(ml_entry, &br);
            state_of = fse_update_state(of_entry, &br);
        }

        /* Emit `ll` literal bytes. */
        lit_end = lit_pos + (size_t)ll;
        if (lit_end < lit_pos || lit_end > lits.len) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }
        if (!zstd_check_budget(out->len, (size_t)ll)) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }
        bb_push_n(out, lits.data + lit_pos, (size_t)ll);
        lit_pos = lit_end;

        /* Copy `ml` bytes from `offset` positions back. Offset 0 or beyond
         * the bytes produced so far is malformed — reject, don't read OOB. */
        if (offset == 0 || (size_t)offset > out->len) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }
        if (!zstd_check_budget(out->len, (size_t)ml)) {
            free(lits.data);
            return ZSTD_ERR_FORMAT;
        }
        copy_start = out->len - (size_t)offset;
        for (j = 0; j < (size_t)ml; j++) {
            /* out->data[copy_start + j] is read BEFORE bb_push (which may
             * reallocate) — safe because function-call argument evaluation
             * completes before the call executes; same idiom as c/lzss's
             * overlap-safe decode. */
            bb_push(out, out->data[copy_start + j]);
        }

        if (!out->ok) {
            free(lits.data);
            return ZSTD_ERR_ALLOC;
        }
    }

    /* Any literals remaining after the last sequence. */
    if (lit_pos > lits.len) {
        free(lits.data);
        return ZSTD_ERR_FORMAT;
    }
    if (!zstd_check_budget(out->len, lits.len - lit_pos)) {
        free(lits.data);
        return ZSTD_ERR_FORMAT;
    }
    bb_push_n(out, lits.data + lit_pos, lits.len - lit_pos);

    free(lits.data);
    return out->ok ? ZSTD_OK : ZSTD_ERR_ALLOC;
}

/* ─── Public API ─────────────────────────────────────────────────────────── */

ZstdStatus zstd_compress(const uint8_t *input, size_t input_len,
                          uint8_t **output, size_t *output_len) {
    ByteBuf out;
    size_t offset;
    uint64_t fcs;
    int k;

    *output = NULL;
    *output_len = 0;
    bb_init(&out);

    /* Magic (4 bytes LE). */
    bb_push_n(&out, ZSTD_MAGIC, 4);

    /* Frame Header Descriptor: FCS_Field_Size=11 (8-byte FCS),
     * Single_Segment=1 (no Window_Descriptor), Unused/Reserved/Checksum/
     * Dict_ID all 0 -> 0b1110_0000 = 0xE0. See zstd.h / lessons.md Lesson 95
     * for why Content_Checksum_Flag is bit 2 (not bit 4) — this constant is
     * fixed regardless, but zstd_decompress's parsing of THIS bit matters
     * for real-world frames, which is where that lesson bit. */
    bb_push(&out, 0xE0);

    /* Frame_Content_Size: 8 bytes LE. */
    fcs = (uint64_t)input_len;
    for (k = 0; k < 8; k++) {
        bb_push(&out, (unsigned char)(fcs & 0xFFu));
        fcs >>= 8;
    }

    if (!out.ok) {
        free(out.data);
        return ZSTD_ERR_ALLOC;
    }

    if (input_len == 0) {
        /* One empty Raw last block: header = last=1,type=Raw,size=0 = 0x01. */
        bb_push(&out, 0x01);
        bb_push(&out, 0x00);
        bb_push(&out, 0x00);
        if (!out.ok) {
            free(out.data);
            return ZSTD_ERR_ALLOC;
        }
        *output = out.data;
        *output_len = out.len;
        return ZSTD_OK;
    }

    offset = 0;
    while (offset < input_len) {
        size_t end = offset + ZSTD_MAX_BLOCK_SIZE;
        const uint8_t *block;
        size_t block_len, i;
        int last, all_same;

        if (end > input_len) {
            end = input_len;
        }
        block = input + offset;
        block_len = end - offset;
        last = (end == input_len) ? 1 : 0;

        all_same = 1;
        for (i = 1; i < block_len; i++) {
            if (block[i] != block[0]) {
                all_same = 0;
                break;
            }
        }

        if (all_same) {
            /* RLE block: header (last, type=01, size) + 1 repeated byte. */
            uint32_t hdr =
                ((uint32_t)block_len << 3) | (1u << 1) | (uint32_t)last;
            bb_push(&out, (unsigned char)(hdr & 0xFFu));
            bb_push(&out, (unsigned char)((hdr >> 8) & 0xFFu));
            bb_push(&out, (unsigned char)((hdr >> 16) & 0xFFu));
            bb_push(&out, block[0]);
        } else {
            ByteBuf compressed;
            int rc = compress_block(block, block_len, &compressed);
            if (rc < 0) {
                free(out.data);
                return ZSTD_ERR_ALLOC;
            }
            if (rc == 1) {
                uint32_t hdr = ((uint32_t)compressed.len << 3) | (2u << 1) |
                                (uint32_t)last;
                bb_push(&out, (unsigned char)(hdr & 0xFFu));
                bb_push(&out, (unsigned char)((hdr >> 8) & 0xFFu));
                bb_push(&out, (unsigned char)((hdr >> 16) & 0xFFu));
                bb_push_n(&out, compressed.data, compressed.len);
                free(compressed.data);
            } else {
                /* Raw fallback: header (last, type=00, size) + verbatim. */
                uint32_t hdr = ((uint32_t)block_len << 3) | (uint32_t)last;
                bb_push(&out, (unsigned char)(hdr & 0xFFu));
                bb_push(&out, (unsigned char)((hdr >> 8) & 0xFFu));
                bb_push(&out, (unsigned char)((hdr >> 16) & 0xFFu));
                bb_push_n(&out, block, block_len);
            }
        }

        if (!out.ok) {
            free(out.data);
            return ZSTD_ERR_ALLOC;
        }
        offset = end;
    }

    *output = out.data;
    *output_len = out.len;
    return ZSTD_OK;
}

ZstdStatus zstd_decompress(const uint8_t *input, size_t input_len,
                            uint8_t **output, size_t *output_len) {
    size_t pos, dict_id_bytes, fcs_bytes;
    uint8_t fhd, fcs_flag, single_seg, checksum_flag, dict_flag;
    static const size_t dict_bytes_tbl[4] = {0, 1, 2, 4};
    ByteBuf out;
    ZstdStatus st;
    /* Repeated_Offset registers (RFC 8878 §3.1.1.3.2.1.1): frame-scoped —
     * default 1/4/8 "for the first block", then threaded unmodified through
     * every Compressed block's sequences for the rest of the frame (Raw/RLE
     * blocks don't touch them). See decompress_block's doc comment. */
    uint32_t rep1 = 1, rep2 = 4, rep3 = 8;

    *output = NULL;
    *output_len = 0;

    if (input_len < 5) {
        return ZSTD_ERR_FORMAT;
    }
    if (input[0] != ZSTD_MAGIC[0] || input[1] != ZSTD_MAGIC[1] ||
        input[2] != ZSTD_MAGIC[2] || input[3] != ZSTD_MAGIC[3]) {
        return ZSTD_ERR_FORMAT;
    }

    pos = 4;
    fhd = input[pos];
    pos += 1;

    fcs_flag = (uint8_t)((fhd >> 6) & 3u);
    single_seg = (uint8_t)((fhd >> 5) & 1u);
    /* Content_Checksum_Flag is FHD BIT 2, not bit 4 — verified empirically
     * against the real `zstd` CLI (`zstd -c` vs `zstd -c --no-check` differ
     * exactly at bit 2) and against RFC 8878 §3.1.1.1 (bit 4 is
     * Unused_bit). See lessons.md Lesson 95: an earlier, unvalidated version
     * of this port class read bit 4 here, which "worked" only because
     * nothing downstream actually enforced a no-trailing-bytes check — the
     * two bugs masked each other until a real checksummed frame from the
     * genuine `zstd` CLI (TC-9) exposed it. */
    checksum_flag = (uint8_t)((fhd >> 2) & 1u);
    dict_flag = (uint8_t)(fhd & 3u);

    if (single_seg == 0) {
        if (pos >= input_len) {
            return ZSTD_ERR_FORMAT;
        }
        pos += 1; /* Window_Descriptor byte: present, but this port doesn't
                     enforce window-size limits, so it's simply skipped. */
    }

    dict_id_bytes = dict_bytes_tbl[dict_flag];
    if (pos + dict_id_bytes > input_len) {
        return ZSTD_ERR_FORMAT;
    }
    pos += dict_id_bytes; /* dictionaries are out of scope; skip the ID */

    switch (fcs_flag) {
        case 0:
            fcs_bytes = (single_seg == 1) ? 1 : 0;
            break;
        case 1:
            fcs_bytes = 2;
            break;
        case 2:
            fcs_bytes = 4;
            break;
        default: /* 3 */
            fcs_bytes = 8;
            break;
    }
    if (pos + fcs_bytes > input_len) {
        return ZSTD_ERR_FORMAT;
    }
    /* Frame_Content_Size is read but deliberately never used to
     * pre-allocate the output buffer (see zstd.h's decompression-bomb
     * note) — it's an untrusted declared size, not a budget. */
    pos += fcs_bytes;

    bb_init(&out);

    for (;;) {
        uint32_t hdr;
        int last;
        unsigned btype;
        size_t bsize;

        if (pos + 3 > input_len) {
            free(out.data);
            return ZSTD_ERR_FORMAT;
        }
        hdr = (uint32_t)input[pos] | ((uint32_t)input[pos + 1] << 8) |
              ((uint32_t)input[pos + 2] << 16);
        pos += 3;

        last = (int)(hdr & 1u);
        btype = (unsigned)((hdr >> 1) & 3u);
        bsize = (size_t)(hdr >> 3);

        /* Security: reject a block claiming more than the 128 KB max as
         * malformed, before trusting `bsize` for any bounds arithmetic. */
        if (bsize > ZSTD_MAX_BLOCK_SIZE) {
            free(out.data);
            return ZSTD_ERR_FORMAT;
        }

        if (btype == 0) {
            /* Raw */
            if (pos + bsize > input_len) {
                free(out.data);
                return ZSTD_ERR_FORMAT;
            }
            if (!zstd_check_budget(out.len, bsize)) {
                free(out.data);
                return ZSTD_ERR_FORMAT;
            }
            bb_push_n(&out, input + pos, bsize);
            pos += bsize;
        } else if (btype == 1) {
            /* RLE */
            if (pos >= input_len) {
                free(out.data);
                return ZSTD_ERR_FORMAT;
            }
            if (!zstd_check_budget(out.len, bsize)) {
                free(out.data);
                return ZSTD_ERR_FORMAT;
            }
            bb_push_rle(&out, input[pos], bsize);
            pos += 1;
        } else if (btype == 2) {
            /* Compressed */
            if (pos + bsize > input_len) {
                free(out.data);
                return ZSTD_ERR_FORMAT;
            }
            st = decompress_block(input + pos, bsize, &out, &rep1, &rep2,
                                   &rep3);
            if (st != ZSTD_OK) {
                free(out.data);
                return st;
            }
            pos += bsize;
        } else {
            free(out.data);
            return ZSTD_ERR_FORMAT; /* reserved block type */
        }

        if (!out.ok) {
            free(out.data);
            return ZSTD_ERR_ALLOC;
        }
        if (last) {
            break;
        }
    }

    /* If Content_Checksum_Flag was set, a 4-byte xxHash64 checksum follows
     * the last block. This port has no xxHash64 implementation, so the
     * value is not verified — but its presence must be accounted for so a
     * caller inspecting bytes past this point (or a future strict
     * trailing-bytes check) doesn't misread it as corruption. Real `zstd`
     * writes this by default, so any real-world interop input is likely to
     * have it (see TC-9). */
    if (checksum_flag == 1 && pos + 4 > input_len) {
        free(out.data);
        return ZSTD_ERR_FORMAT;
    }

    *output = out.data;
    *output_len = out.len;
    return ZSTD_OK;
}
