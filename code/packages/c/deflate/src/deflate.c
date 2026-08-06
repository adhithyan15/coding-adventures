/*
 * deflate.c — implementation of DEFLATE (see deflate.h). A faithful port of
 * the Rust `deflate` crate: LZSS tokenization (via `c/lzss`) feeding a
 * fixed-or-dynamic Huffman coder, exactly per RFC 1951.
 *
 * File map:
 *   1. Growable byte buffer (ByteBuf) — output accumulator, shared by encode
 *      and decode.
 *   2. Bit I/O — BitWriter (LSB-first packing; MSB-first Huffman emission via
 *      bit reversal) and BitReader (the inverse).
 *   3. Length/distance code tables (RFC 1951 §3.2.5) and their lookups.
 *   4. Fixed Huffman tables (RFC 1951 §3.2.6) — encode and decode sides.
 *   5. Canonical Huffman: code assignment (encode) and the "puff.c-style"
 *      base/offset decode table (decode) — both driven purely by a
 *      code-length array, so encoder and decoder agree by construction.
 *   6. Length-limited Huffman via package-merge (RFC 1951 §3.2.7 mandates
 *      codes ≤ 15 bits; a plain Huffman tree can exceed that on skewed data).
 *   7. Dynamic-block planning: frequency counts → length-limited trees → CL
 *      run-length encoding → exact bit cost.
 *   8. Block emitters (fixed / dynamic) and `deflate_compress`.
 *   9. Block decoders (stored / fixed / dynamic) and `deflate_decompress`.
 */
#include "deflate.h"
#include "lzss.h" /* c/lzss (CMP02): lzss_encode, LzssToken */

#include <stdint.h> /* uint8_t..uint64_t */
#include <stdlib.h> /* malloc, realloc, calloc, free */
#include <string.h> /* memcpy, memset */

/* RFC 1951 caps every LL/distance Huffman code at 15 bits, and every
 * code-length (CL) code at 7 bits (7 fits comfortably under 15, so one
 * constant covers both table sizes below). */
#define DEFLATE_MAX_BITS 15

/* ===========================================================================
 * 1. Growable byte buffer
 * ===========================================================================
 * Identical growth policy to `c/lzss`'s ByteBuf: doubling capacity, overflow-
 * checked, `ok` latches to 0 forever on the first allocation failure so every
 * subsequent push becomes a no-op instead of touching freed/undersized memory.
 */
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
    if (!b->ok) {
        return 0;
    }
    if (extra > SIZE_MAX - b->len) {
        b->ok = 0;
        return 0;
    }
    {
        size_t need = b->len + extra;
        if (need <= b->cap) {
            return 1;
        }
        {
            size_t nc = b->cap ? b->cap : 64;
            unsigned char *nd;
            while (nc < need) {
                if (nc > SIZE_MAX / 2) {
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
        }
    }
    return 1;
}

static void bb_push(ByteBuf *b, unsigned char c) {
    if (bb_reserve(b, 1)) {
        b->data[b->len++] = c;
    }
}

/* ===========================================================================
 * 2. Bit I/O
 * ===========================================================================
 * DEFLATE packs bits into bytes LSB-first: the first bit written occupies bit
 * 0 of the first byte. Huffman codes are assigned canonically (MSB-first
 * semantics), so writing one means emitting its bits from the top down, each
 * as the next LSB-first stream bit — and reading one means accumulating bits
 * from the bottom up (`code = (code << 1) | next_bit`) until a canonical
 * (code, length) pair matches. See `iso_test.h`-style comments throughout for
 * why: this exact asymmetry is RFC 1951 §3.1.1's "packing into bytes" rule.
 */
typedef struct {
    uint64_t buf;
    unsigned bit_pos; /* number of valid bits currently staged in buf, 0..63 */
    ByteBuf out;
} BitWriter;

static void bw_init(BitWriter *w) {
    w->buf = 0;
    w->bit_pos = 0;
    bb_init(&w->out);
}

/* Write the low `n` bits of `val` (0 <= n <= 32), LSB of val emitted first. */
static void bw_write_bits_lsb(BitWriter *w, uint32_t val, unsigned n) {
    unsigned i;
    for (i = 0; i < n; i++) {
        if ((val >> i) & 1u) {
            w->buf |= ((uint64_t)1) << w->bit_pos;
        }
        w->bit_pos++;
        if (w->bit_pos == 64) {
            unsigned k;
            for (k = 0; k < 8; k++) {
                bb_push(&w->out, (unsigned char)(w->buf & 0xFFu));
                w->buf >>= 8;
            }
            w->bit_pos = 0;
        }
    }
}

/* Write a canonical Huffman code: `nbits` bits of `code`, MSB first — i.e. the
 * bit at position (nbits-1) of `code` is the first bit pushed into the
 * LSB-first stream, then (nbits-2), ... down to bit 0. This is the exact
 * inverse of the decode loop in `huff_decode_symbol` below. */
static void bw_write_huffman(BitWriter *w, uint32_t code, unsigned nbits) {
    unsigned i;
    for (i = 0; i < nbits; i++) {
        unsigned bit = (code >> (nbits - 1u - i)) & 1u;
        if (bit) {
            w->buf |= ((uint64_t)1) << w->bit_pos;
        }
        w->bit_pos++;
        if (w->bit_pos == 64) {
            unsigned k;
            for (k = 0; k < 8; k++) {
                bb_push(&w->out, (unsigned char)(w->buf & 0xFFu));
                w->buf >>= 8;
            }
            w->bit_pos = 0;
        }
    }
}

/* Flush any partial byte (zero-padded to the next byte boundary). */
static void bw_flush(BitWriter *w) {
    while (w->bit_pos > 0) {
        bb_push(&w->out, (unsigned char)(w->buf & 0xFFu));
        w->buf >>= 8;
        w->bit_pos = (w->bit_pos >= 8) ? w->bit_pos - 8 : 0;
    }
}

/* Maintains a lookahead buffer of up to 64 bits so callers can read up to 32
 * bits at a time without a per-bit refill. `read_bits(n)` returns the next n
 * bits LSB-first (bit 0 of the result = earliest bit in the stream) — used for
 * block headers, extra bits, and HLIT/HDIST/HCLEN fields. Huffman symbols are
 * decoded one bit at a time (see `huff_decode_symbol`), accumulated MSB-first. */
typedef struct {
    const unsigned char *data;
    size_t len;
    size_t byte_pos;
    uint64_t buf;
    unsigned bits_in_buf;
} BitReader;

static void br_init(BitReader *r, const unsigned char *data, size_t len) {
    r->data = data;
    r->len = len;
    r->byte_pos = 0;
    r->buf = 0;
    r->bits_in_buf = 0;
}

/* Returns 0 (and leaves the reader unusable for further reads of `n`) on
 * truncated input rather than reading past `len` — every caller propagates
 * this as DEFLATE_ERR_MALFORMED. */
static int br_refill(BitReader *r, unsigned n) {
    while (r->bits_in_buf < n) {
        if (r->byte_pos >= r->len) {
            return 0;
        }
        r->buf |= ((uint64_t)r->data[r->byte_pos]) << r->bits_in_buf;
        r->byte_pos++;
        r->bits_in_buf += 8;
    }
    return 1;
}

static int br_read_bits(BitReader *r, unsigned n, uint32_t *out) {
    if (n == 0) {
        *out = 0;
        return 1;
    }
    if (!br_refill(r, n)) {
        return 0;
    }
    *out = (uint32_t)(r->buf & (((uint64_t)1 << n) - 1));
    r->buf >>= n;
    r->bits_in_buf -= n;
    return 1;
}

/* Discard any partial bits left in the current byte, so the next read starts
 * at a byte boundary. Used before a stored block's LEN/NLEN/data. */
static void br_align_to_byte(BitReader *r) {
    unsigned leftover = r->bits_in_buf % 8;
    if (leftover != 0) {
        r->buf >>= leftover;
        r->bits_in_buf -= leftover;
    }
}

static int br_read_byte(BitReader *r, unsigned char *out) {
    uint32_t v;
    if (!br_read_bits(r, 8, &v)) {
        return 0;
    }
    *out = (unsigned char)v;
    return 1;
}

/* ===========================================================================
 * 3. Length / distance code tables (RFC 1951 §3.2.5)
 * ===========================================================================
 * LENGTH_TABLE covers LL symbols 257-285 (our own encoder, capped at
 * max_match=255, never emits 285 — but the DECODER must recognise it, since
 * real producers with match length 258 do emit it). DIST_TABLE covers the
 * full 32768-byte-window distance alphabet, codes 0-29 — "encode
 * conservatively [into whichever codes the data needs], decode liberally [the
 * complete RFC 1951 alphabet, since we must also read zlib/gzip/Office
 * streams]," as CMP05 puts it.
 */
typedef struct {
    unsigned short symbol;
    unsigned int base;
    unsigned char extra;
} LenEntry;

static const LenEntry LENGTH_TABLE[] = {
    {257, 3, 0},  {258, 4, 0},  {259, 5, 0},  {260, 6, 0},
    {261, 7, 0},  {262, 8, 0},  {263, 9, 0},  {264, 10, 0},
    {265, 11, 1}, {266, 13, 1}, {267, 15, 1}, {268, 17, 1},
    {269, 19, 2}, {270, 23, 2}, {271, 27, 2}, {272, 31, 2},
    {273, 35, 3}, {274, 43, 3}, {275, 51, 3}, {276, 59, 3},
    {277, 67, 4}, {278, 83, 4}, {279, 99, 4}, {280, 115, 4},
    {281, 131, 5}, {282, 163, 5}, {283, 195, 5}, {284, 227, 5},
    /* Symbol 285: the "maximum match" code — length 258 exactly, no extra
     * bits. Our own encoder (max_match=255) never emits it; the decoder must
     * recognise it to read zlib/gzip/Office streams, which use it routinely. */
    {285, 258, 0},
};
#define LENGTH_TABLE_SIZE (sizeof(LENGTH_TABLE) / sizeof(LENGTH_TABLE[0]))

typedef struct {
    unsigned short code;
    unsigned int base;
    unsigned char extra;
} DistEntry;

static const DistEntry DIST_TABLE[] = {
    {0, 1, 0},      {1, 2, 0},      {2, 3, 0},      {3, 4, 0},
    {4, 5, 1},      {5, 7, 1},      {6, 9, 2},      {7, 13, 2},
    {8, 17, 3},     {9, 25, 3},     {10, 33, 4},    {11, 49, 4},
    {12, 65, 5},    {13, 97, 5},    {14, 129, 6},   {15, 193, 6},
    {16, 257, 7},   {17, 385, 7},   {18, 513, 8},   {19, 769, 8},
    {20, 1025, 9},  {21, 1537, 9},  {22, 2049, 10}, {23, 3073, 10},
    {24, 4097, 11}, {25, 6145, 11}, {26, 8193, 12}, {27, 12289, 12},
    {28, 16385, 13}, {29, 24577, 13},
};
#define DIST_TABLE_SIZE (sizeof(DIST_TABLE) / sizeof(DIST_TABLE[0]))

/* Linear scans below: both tables are tiny (29 and 30 entries), so this is
 * simplicity over micro-optimisation — each call runs a handful of times per
 * token, not per byte. */

static unsigned short length_symbol(unsigned length) {
    size_t i;
    for (i = 0; i < LENGTH_TABLE_SIZE; i++) {
        unsigned max_len = LENGTH_TABLE[i].base + (1u << LENGTH_TABLE[i].extra) - 1u;
        if (length <= max_len) {
            return LENGTH_TABLE[i].symbol;
        }
    }
    return 284; /* unreachable given max_match=255, kept as a safe fallback */
}

static unsigned short dist_code_for(unsigned distance) {
    size_t i;
    for (i = 0; i < DIST_TABLE_SIZE; i++) {
        unsigned max_d = DIST_TABLE[i].base + (1u << DIST_TABLE[i].extra) - 1u;
        if (distance <= max_d) {
            return DIST_TABLE[i].code;
        }
    }
    return 23; /* unreachable given window_size=32768, kept as a safe fallback */
}

static unsigned length_base_of(unsigned short sym) {
    size_t i;
    for (i = 0; i < LENGTH_TABLE_SIZE; i++) {
        if (LENGTH_TABLE[i].symbol == sym) {
            return LENGTH_TABLE[i].base;
        }
    }
    return 0;
}

static unsigned length_extra_of(unsigned short sym) {
    size_t i;
    for (i = 0; i < LENGTH_TABLE_SIZE; i++) {
        if (LENGTH_TABLE[i].symbol == sym) {
            return LENGTH_TABLE[i].extra;
        }
    }
    return 0;
}

static unsigned dist_base_of(unsigned short code) {
    size_t i;
    for (i = 0; i < DIST_TABLE_SIZE; i++) {
        if (DIST_TABLE[i].code == code) {
            return DIST_TABLE[i].base;
        }
    }
    return 0;
}

static unsigned dist_extra_of(unsigned short code) {
    size_t i;
    for (i = 0; i < DIST_TABLE_SIZE; i++) {
        if (DIST_TABLE[i].code == code) {
            return DIST_TABLE[i].extra;
        }
    }
    return 0;
}

/* ===========================================================================
 * 4. Fixed Huffman tables (RFC 1951 §3.2.6)
 * ===========================================================================
 * The fixed literal/length codes are pre-agreed, so a fixed block transmits no
 * table at all:
 *
 *   Symbols   0-143 -> 8-bit codes, starting at 0b00110000  (=  48)
 *   Symbols 144-255 -> 9-bit codes, starting at 0b110010000 (= 400)
 *   Symbols 256-279 -> 7-bit codes, starting at 0b0000000   (=   0)
 *   Symbols 280-287 -> 8-bit codes, starting at 0b11000000  (= 192)
 *
 * Distance symbols are all 5-bit codes numerically equal to the symbol.
 */
static void fixed_ll_code(unsigned sym, uint32_t *code, unsigned *nbits) {
    if (sym <= 143) {
        *code = 0x30u + sym;
        *nbits = 8;
    } else if (sym <= 255) {
        *code = 0x190u + (sym - 144);
        *nbits = 9;
    } else if (sym <= 279) {
        *code = sym - 256;
        *nbits = 7;
    } else {
        *code = 0xC0u + (sym - 280);
        *nbits = 8;
    }
}

static void fixed_ll_lengths(unsigned char *lens /* size 288 */) {
    size_t i;
    for (i = 0; i <= 143; i++) lens[i] = 8;
    for (i = 144; i <= 255; i++) lens[i] = 9;
    for (i = 256; i <= 279; i++) lens[i] = 7;
    for (i = 280; i <= 287; i++) lens[i] = 8;
}

static void fixed_dist_lengths(unsigned char *lens /* size 32 */) {
    size_t i;
    for (i = 0; i < 32; i++) lens[i] = 5;
}

/* ===========================================================================
 * 5. Canonical Huffman: encode-side codes, decode-side base/offset table
 * ===========================================================================
 */

/* Assign canonical codes (RFC 1951 §3.2.2) for `n` symbols given their code
 * lengths (0 = absent). Writes `codes[sym]` for every present symbol; absent
 * symbols get 0 (never read, since their length is 0). */
static void huff_build_canonical_codes(const unsigned char *lengths, size_t n,
                                       uint32_t *codes) {
    unsigned bl_count[DEFLATE_MAX_BITS + 1];
    uint32_t next_code[DEFLATE_MAX_BITS + 2];
    unsigned max_len = 0;
    size_t i;
    unsigned bits;
    uint32_t code;

    memset(codes, 0, n * sizeof *codes);
    for (i = 0; i < n; i++) {
        if (lengths[i] > max_len) {
            max_len = lengths[i];
        }
    }
    if (max_len == 0) {
        return;
    }

    memset(bl_count, 0, sizeof bl_count);
    for (i = 0; i < n; i++) {
        if (lengths[i] > 0) {
            bl_count[lengths[i]]++;
        }
    }

    memset(next_code, 0, sizeof next_code);
    code = 0;
    for (bits = 1; bits <= max_len; bits++) {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    for (i = 0; i < n; i++) {
        if (lengths[i] > 0) {
            codes[i] = next_code[lengths[i]];
            next_code[lengths[i]]++;
        }
    }
}

/* Decode table in the classic "base + offset per length" form (the technique
 * behind Mark Adler's `puff.c`, a minimal reference inflate): rather than a
 * hash map keyed by (code, length), we exploit that canonical codes of the
 * same length are *consecutive integers* assigned in ascending symbol order.
 * So for each length we only need (a) how many codes have that length and
 * (b) the symbols with that length, listed in ascending order — from which a
 * candidate code's rank within its length directly indexes the symbol. */
typedef struct {
    unsigned short counts[DEFLATE_MAX_BITS + 1]; /* counts[len], len = 1..15 */
    unsigned short *symbols;                     /* malloc'd; ascending (len, symbol) */
} HuffDecodeTable;

/* Returns 0 only on allocation failure. `lengths[i] > DEFLATE_MAX_BITS` is
 * treated as "absent" defensively; every length actually reaching this
 * function (fixed tables, or CL-decoded dynamic-block lengths) is already
 * bounded to <= 15 by construction, so that branch is unreachable in
 * practice but costs nothing to guard. */
static int huff_build_decode_table(const unsigned char *lengths, size_t n,
                                   HuffDecodeTable *t) {
    size_t sym, present = 0;
    unsigned len;
    size_t offs[DEFLATE_MAX_BITS + 2];

    memset(t->counts, 0, sizeof t->counts);
    for (sym = 0; sym < n; sym++) {
        unsigned char l = lengths[sym];
        if (l > 0 && l <= DEFLATE_MAX_BITS) {
            t->counts[l]++;
            present++;
        }
    }

    t->symbols = (unsigned short *)malloc((present > 0 ? present : 1) * sizeof(unsigned short));
    if (!t->symbols) {
        return 0;
    }

    offs[0] = 0;
    offs[1] = 0;
    for (len = 1; len < DEFLATE_MAX_BITS; len++) {
        offs[len + 1] = offs[len] + t->counts[len];
    }
    for (sym = 0; sym < n; sym++) {
        unsigned char l = lengths[sym];
        if (l > 0 && l <= DEFLATE_MAX_BITS) {
            t->symbols[offs[l]] = (unsigned short)sym;
            offs[l]++;
        }
    }
    return 1;
}

static void huff_free_decode_table(HuffDecodeTable *t) {
    free(t->symbols);
    t->symbols = NULL;
}

/* Decode one symbol: accumulate bits MSB-first (`code = (code << 1) | bit`)
 * one at a time, and after each bit check whether `code` falls in the
 * contiguous range of codes assigned to the current length. Returns 0 on a
 * truncated stream or a code that never matches any assigned length (both are
 * DEFLATE_ERR_MALFORMED to the caller). */
static int huff_decode_symbol(BitReader *r, const HuffDecodeTable *t,
                              unsigned short *out_sym) {
    int code = 0, first = 0, index = 0;
    unsigned len;
    for (len = 1; len <= DEFLATE_MAX_BITS; len++) {
        uint32_t bit;
        int count;
        if (!br_read_bits(r, 1, &bit)) {
            return 0;
        }
        code |= (int)bit;
        count = t->counts[len];
        if (code - first < count) {
            *out_sym = t->symbols[index + (code - first)];
            return 1;
        }
        index += count;
        first += count;
        first <<= 1;
        code <<= 1;
    }
    return 0;
}

/* ===========================================================================
 * 6. Length-limited Huffman: package-merge (Larmore-Hirschberg 1990)
 * ===========================================================================
 * RFC 1951 caps every code at `max_len` bits (15 for LL/distance, 7 for CL).
 * A plain (unlimited) Huffman tree can exceed that on skewed frequencies, and
 * emitting a >15-bit code would make the stream invalid — so `compress`
 * builds *length-limited* trees via package-merge, which provably yields the
 * OPTIMAL code among every code with max length <= max_len (proven by
 * Larmore & Hirschberg, JACM 1990), and always succeeds whenever the alphabet
 * size <= 2^max_len (true here: LL 286 <= 2^15, dist 30 <= 2^15, CL 19 <= 2^7).
 *
 * The "coin collector" framing: think of each symbol's code as being built up
 * bit by bit — a "coin" at depth d (1 <= d <= max_len) costs weight = freq
 * and buys one more bit of that symbol's code length. Kraft's inequality says
 * a valid code needs exactly (2m - 2) such coins total (m = symbol count, at
 * depths 1..max_len across all symbols) for the minimum-cost assignment.
 * Package-merge finds the minimum-weight (2m - 2) coins directly:
 *
 *   1. Level max_len's list = the m original symbols (coins), sorted by
 *      weight ascending.
 *   2. Repeat (max_len - 1) times, going from a deeper level to the next
 *      shallower one:
 *        a. PACKAGE: pair up adjacent items of the current list (already
 *           sorted), summing weights; each pair becomes one "package" that
 *           covers the union of symbols its two halves covered. Drop a
 *           trailing unpaired item.
 *        b. MERGE: merge those packages with the ORIGINAL symbol list (both
 *           sorted ascending) into the next level's list.
 *   3. After the final (level-1) list, take its (2m - 2) lowest-weight items.
 *      Each selected item's `covers` set gets +1 code-length for every symbol
 *      it covers — summed across all selected items, that is each symbol's
 *      final code length.
 *
 * We track `covers` as a bitmask (one bit per present symbol) rather than an
 * explicit list — the alphabets here are tiny (<=286), so O(m) union/count
 * per item is negligible, and a bitmask needs no dynamic growth.
 */
typedef struct {
    unsigned long weight;
    unsigned char *cov; /* bitmask, mask_bytes long; owned by this item */
} PMItem;

static int pm_item_alloc(PMItem *it, unsigned long weight, size_t mask_bytes) {
    it->weight = weight;
    it->cov = (unsigned char *)calloc(mask_bytes ? mask_bytes : 1, 1);
    return it->cov != NULL;
}

static int pm_item_clone(PMItem *dst, const PMItem *src, size_t mask_bytes) {
    dst->weight = src->weight;
    dst->cov = (unsigned char *)malloc(mask_bytes ? mask_bytes : 1);
    if (!dst->cov) {
        return 0;
    }
    memcpy(dst->cov, src->cov, mask_bytes ? mask_bytes : 1);
    return 1;
}

static int pm_item_union(PMItem *dst, const PMItem *a, const PMItem *b, size_t mask_bytes) {
    size_t i;
    dst->weight = a->weight + b->weight;
    dst->cov = (unsigned char *)malloc(mask_bytes ? mask_bytes : 1);
    if (!dst->cov) {
        return 0;
    }
    for (i = 0; i < mask_bytes; i++) {
        dst->cov[i] = (unsigned char)(a->cov[i] | b->cov[i]);
    }
    return 1;
}

static void pm_item_free(PMItem *it) {
    free(it->cov);
    it->cov = NULL;
}

static void pm_set_bit(unsigned char *mask, size_t idx) {
    mask[idx / 8] |= (unsigned char)(1u << (idx % 8));
}

static int pm_get_bit(const unsigned char *mask, size_t idx) {
    return (mask[idx / 8] >> (idx % 8)) & 1;
}

typedef struct {
    PMItem *items;
    size_t count;
} PMList;

static void pmlist_free(PMList *l) {
    size_t i;
    for (i = 0; i < l->count; i++) {
        pm_item_free(&l->items[i]);
    }
    free(l->items);
    l->items = NULL;
    l->count = 0;
}

/* Verify code lengths satisfy Kraft's inequality (Sum 2^(max_len-len) <=
 * 2^max_len) and respect the cap. Integer arithmetic throughout — no float
 * rounding error possible. A `compress`-side invariant: if this ever fails
 * (it shouldn't, given our bounded alphabets), the caller discards the plan
 * and falls back to a fixed block rather than ever emitting an invalid
 * stream. */
static int kraft_sum_ok(const unsigned char *lengths, size_t n, unsigned max_len) {
    uint64_t total = 0;
    uint64_t limit = (uint64_t)1 << max_len;
    size_t i;
    for (i = 0; i < n; i++) {
        unsigned l = lengths[i];
        if (l == 0) {
            continue;
        }
        if (l > max_len) {
            return 0;
        }
        total += (uint64_t)1 << (max_len - l);
        if (total > limit) {
            return 0;
        }
    }
    return 1;
}

/* Compute length-limited Huffman code lengths for `freqs[0..n)`, capped at
 * `max_len` bits. Returns a malloc'd `lengths[n]` array (0 = absent symbol)
 * on success, or NULL on allocation failure. All-zero frequencies yield an
 * all-zero result (the caller assigns any required dummy code itself, since
 * "at least one code" is a wire-format rule, not a Huffman-construction one). */
static unsigned char *length_limited_huffman(const unsigned long *freqs, size_t n,
                                             unsigned max_len) {
    size_t *present;
    size_t m = 0, i;
    unsigned char *lengths;
    size_t mask_bytes;
    PMList originals, list;
    int ok = 1;

    lengths = (unsigned char *)calloc(n ? n : 1, 1);
    if (!lengths) {
        return NULL;
    }
    present = (size_t *)malloc((n ? n : 1) * sizeof *present);
    if (!present) {
        free(lengths);
        return NULL;
    }
    for (i = 0; i < n; i++) {
        if (freqs[i] > 0) {
            present[m++] = i;
        }
    }

    if (m == 0) {
        free(present);
        return lengths;
    }
    if (m == 1) {
        lengths[present[0]] = 1; /* a single symbol still needs a valid 1-bit code */
        free(present);
        return lengths;
    }

    mask_bytes = (m + 7) / 8;

    /* Original coins: one per present symbol, in ascending symbol order
     * (matching `present[]`), then stable-sorted by weight ascending. Simple
     * insertion sort: alphabets are tiny (<=286) and stability (ties keep
     * ascending-symbol order) makes the result deterministic. */
    originals.items = (PMItem *)malloc(m * sizeof(PMItem));
    originals.count = 0;
    if (!originals.items) {
        ok = 0;
    }
    for (i = 0; i < m && ok; i++) {
        ok = pm_item_alloc(&originals.items[i], freqs[present[i]], mask_bytes);
        if (ok) {
            pm_set_bit(originals.items[i].cov, i);
            originals.count = i + 1;
        }
    }
    if (ok) {
        size_t j;
        for (i = 1; i < m; i++) {
            PMItem key = originals.items[i];
            j = i;
            while (j > 0 && originals.items[j - 1].weight > key.weight) {
                originals.items[j] = originals.items[j - 1];
                j--;
            }
            originals.items[j] = key;
        }
    }
    if (!ok) {
        pmlist_free(&originals);
        free(present);
        free(lengths);
        return NULL;
    }

    /* `list` starts as an independent clone of `originals` — `originals`
     * itself must stay untouched, since every level's merge step reads it. */
    list.items = (PMItem *)malloc(m * sizeof(PMItem));
    list.count = 0;
    if (!list.items) {
        ok = 0;
    }
    for (i = 0; i < m && ok; i++) {
        ok = pm_item_clone(&list.items[i], &originals.items[i], mask_bytes);
        if (ok) {
            list.count = i + 1;
        }
    }

    {
        unsigned level;
        for (level = 1; ok && level < max_len; level++) {
            PMList packaged, merged;
            size_t k, oi, pi, mi;

            packaged.count = list.count / 2;
            packaged.items = packaged.count
                                 ? (PMItem *)malloc(packaged.count * sizeof(PMItem))
                                 : NULL;
            if (packaged.count && !packaged.items) {
                ok = 0;
                break;
            }
            for (k = 0; k < packaged.count; k++) {
                if (!pm_item_union(&packaged.items[k], &list.items[2 * k],
                                   &list.items[2 * k + 1], mask_bytes)) {
                    ok = 0;
                    packaged.count = k;
                    break;
                }
            }
            if (!ok) {
                pmlist_free(&packaged);
                break;
            }

            merged.items = (PMItem *)malloc(
                (m + packaged.count > 0 ? m + packaged.count : 1) * sizeof(PMItem));
            merged.count = 0;
            if (!merged.items) {
                ok = 0;
                pmlist_free(&packaged);
                break;
            }

            oi = 0;
            pi = 0;
            mi = 0;
            while (ok && oi < m && pi < packaged.count) {
                if (originals.items[oi].weight <= packaged.items[pi].weight) {
                    ok = pm_item_clone(&merged.items[mi], &originals.items[oi], mask_bytes);
                    oi++;
                } else {
                    ok = pm_item_clone(&merged.items[mi], &packaged.items[pi], mask_bytes);
                    pi++;
                }
                if (ok) {
                    mi++;
                }
            }
            while (ok && oi < m) {
                ok = pm_item_clone(&merged.items[mi], &originals.items[oi], mask_bytes);
                oi++;
                if (ok) {
                    mi++;
                }
            }
            while (ok && pi < packaged.count) {
                ok = pm_item_clone(&merged.items[mi], &packaged.items[pi], mask_bytes);
                pi++;
                if (ok) {
                    mi++;
                }
            }
            merged.count = mi;

            pmlist_free(&packaged); /* packaged items were only ever cloned FROM */
            pmlist_free(&list);
            list = merged;
        }
    }

    if (ok) {
        size_t take = 2 * m - 2;
        unsigned *depth = (unsigned *)calloc(m, sizeof(unsigned));
        if (!depth) {
            ok = 0;
        } else {
            size_t idx;
            if (take > list.count) {
                take = list.count; /* defensive; provably unreachable for m <= 2^max_len */
            }
            for (idx = 0; idx < take; idx++) {
                size_t b;
                for (b = 0; b < m; b++) {
                    if (pm_get_bit(list.items[idx].cov, b)) {
                        depth[b]++;
                    }
                }
            }
            for (i = 0; i < m; i++) {
                unsigned d = depth[i];
                if (d < 1) {
                    d = 1; /* defensive: package-merge guarantees d >= 1 for m >= 2 */
                }
                if (d > max_len) {
                    d = max_len; /* defensive: provably unreachable for our alphabets */
                }
                lengths[present[i]] = (unsigned char)d;
            }
            free(depth);
        }
    }

    pmlist_free(&list);
    pmlist_free(&originals);
    free(present);

    if (!ok) {
        free(lengths);
        return NULL;
    }
    return lengths;
}

/* ===========================================================================
 * 7. Dynamic-block planning (RFC 1951 §3.2.7)
 * ===========================================================================
 */
static const size_t CL_PERMUTATION[19] = {16, 17, 18, 0, 8,  7, 9, 6, 10, 5,
                                          11, 4,  12, 3, 13, 2, 14, 1, 15};

/* One element of the run-length-encoded (LL ++ dist) code-length stream:
 *   sym  0-15 : a literal code length (no extra bits)
 *   sym  16   : repeat the previous length, extra = count-3, count 3..6  (2 bits)
 *   sym  17   : a run of zeros,           extra = count-3, count 3..10  (3 bits)
 *   sym  18   : a run of zeros,           extra = count-11, count 11..138 (7 bits)
 */
typedef struct {
    unsigned short sym;
    unsigned char extra_bits;
    unsigned int extra_val;
} ClItem;

typedef struct {
    ClItem *items;
    size_t count, cap;
} ClBuf;

static void clbuf_init(ClBuf *b) {
    b->items = NULL;
    b->count = 0;
    b->cap = 0;
}

static int clbuf_push(ClBuf *b, ClItem it) {
    if (b->count == b->cap) {
        size_t nc = b->cap ? b->cap * 2 : 32;
        ClItem *ni = (ClItem *)realloc(b->items, nc * sizeof *ni);
        if (!ni) {
            return 0;
        }
        b->items = ni;
        b->cap = nc;
    }
    b->items[b->count++] = it;
    return 1;
}

/* RLE-encode `lengths[0..n)` into CL symbols, per RFC 1951 §3.2.7: a literal
 * length is emitted as itself; a run of >= 4 identical NONZERO lengths uses
 * symbol 16 (repeat-previous) for everything past the first; a run of zeros
 * uses symbol 18 (11..138) then 17 (3..10) then literal zeros for the
 * remainder. Returns 0 on allocation failure. */
static int rle_code_lengths(const unsigned char *lengths, size_t n, ClBuf *out) {
    size_t i = 0;
    clbuf_init(out);
    while (i < n) {
        unsigned char cur = lengths[i];
        size_t run = 1;
        while (i + run < n && lengths[i + run] == cur) {
            run++;
        }

        if (cur == 0) {
            size_t remaining = run;
            while (remaining >= 11) {
                size_t count = remaining < 138 ? remaining : 138;
                ClItem it;
                it.sym = 18;
                it.extra_bits = 7;
                it.extra_val = (unsigned int)(count - 11);
                if (!clbuf_push(out, it)) return 0;
                remaining -= count;
            }
            while (remaining >= 3) {
                size_t count = remaining < 10 ? remaining : 10;
                ClItem it;
                it.sym = 17;
                it.extra_bits = 3;
                it.extra_val = (unsigned int)(count - 3);
                if (!clbuf_push(out, it)) return 0;
                remaining -= count;
            }
            for (; remaining > 0; remaining--) {
                ClItem it;
                it.sym = 0;
                it.extra_bits = 0;
                it.extra_val = 0;
                if (!clbuf_push(out, it)) return 0;
            }
        } else {
            size_t remaining = run - 1;
            ClItem first;
            first.sym = cur;
            first.extra_bits = 0;
            first.extra_val = 0;
            if (!clbuf_push(out, first)) return 0;
            while (remaining >= 3) {
                size_t count = remaining < 6 ? remaining : 6;
                ClItem it;
                it.sym = 16;
                it.extra_bits = 2;
                it.extra_val = (unsigned int)(count - 3);
                if (!clbuf_push(out, it)) return 0;
                remaining -= count;
            }
            for (; remaining > 0; remaining--) {
                ClItem it;
                it.sym = cur;
                it.extra_bits = 0;
                it.extra_val = 0;
                if (!clbuf_push(out, it)) return 0;
            }
        }
        i += run;
    }
    return 1;
}

/* Everything needed to emit a dynamic block: the code tables, the RLE'd
 * header, and the exact bit cost (so `deflate_compress` can pick fixed vs.
 * dynamic by comparing real numbers, never a heuristic). */
typedef struct {
    unsigned char ll_lengths_full[288];   /* full LL alphabet; trailing 0 = absent */
    unsigned char dist_lengths_full[32];  /* full dist alphabet */
    uint32_t ll_codes[288];
    uint32_t dist_codes[32];
    unsigned char cl_lengths[19];
    uint32_t cl_codes[19];
    size_t hlit;           /* transmitted LL length count, 257..286 */
    size_t hdist;          /* transmitted dist length count, 1..30 */
    size_t cl_order_count; /* transmitted CL length count, 4..19 */
    ClBuf rle;
    unsigned long total_bits;
} DynamicPlan;

static void plan_free(DynamicPlan *p) {
    free(p->rle.items);
    p->rle.items = NULL;
    p->rle.count = 0;
    p->rle.cap = 0;
}

/* Build a DynamicPlan for `tokens`. Returns 0 (and leaves `*plan` fully freed)
 * on allocation failure OR if a length-limiting step ever violates its
 * invariants — both cases are the caller's cue to skip the dynamic block and
 * emit fixed instead, never a malformed stream. */
static int plan_dynamic(const LzssToken *tokens, size_t count, DynamicPlan *plan) {
    unsigned long ll_freq[286];
    unsigned long dist_freq[30];
    unsigned char *ll_len, *dist_len, *cl_len;
    unsigned long cl_freq[19];
    size_t i, hlit, hdist;
    int any_dist = 0;

    memset(plan, 0, sizeof *plan);
    clbuf_init(&plan->rle);

    memset(ll_freq, 0, sizeof ll_freq);
    memset(dist_freq, 0, sizeof dist_freq);
    ll_freq[256] = 1; /* end-of-block always appears exactly once */
    for (i = 0; i < count; i++) {
        if (!tokens[i].is_match) {
            ll_freq[tokens[i].literal]++;
        } else {
            unsigned short lsym = length_symbol(tokens[i].length);
            unsigned short dc = dist_code_for(tokens[i].offset);
            ll_freq[lsym]++;
            dist_freq[dc]++;
        }
    }

    ll_len = length_limited_huffman(ll_freq, 286, DEFLATE_MAX_BITS);
    if (!ll_len) {
        return 0;
    }
    dist_len = length_limited_huffman(dist_freq, 30, DEFLATE_MAX_BITS);
    if (!dist_len) {
        free(ll_len);
        return 0;
    }

    /* RFC 1951 requires HDIST >= 1 even with no matches in the block: give it
     * one dummy 1-bit code for symbol 0 (never referenced by any token). */
    for (i = 0; i < 30; i++) {
        if (dist_len[i] > 0) {
            any_dist = 1;
            break;
        }
    }
    if (!any_dist) {
        dist_len[0] = 1;
    }

    if (!kraft_sum_ok(ll_len, 286, DEFLATE_MAX_BITS) ||
        !kraft_sum_ok(dist_len, 30, DEFLATE_MAX_BITS)) {
        free(ll_len);
        free(dist_len);
        return 0;
    }

    memcpy(plan->ll_lengths_full, ll_len, 286);
    memcpy(plan->dist_lengths_full, dist_len, 30);
    free(ll_len);
    free(dist_len);

    hlit = 286;
    while (hlit > 257 && plan->ll_lengths_full[hlit - 1] == 0) {
        hlit--;
    }
    hdist = 30;
    while (hdist > 1 && plan->dist_lengths_full[hdist - 1] == 0) {
        hdist--;
    }
    plan->hlit = hlit;
    plan->hdist = hdist;

    huff_build_canonical_codes(plan->ll_lengths_full, 288, plan->ll_codes);
    huff_build_canonical_codes(plan->dist_lengths_full, 32, plan->dist_codes);

    {
        unsigned char combined[286 + 30];
        memcpy(combined, plan->ll_lengths_full, hlit);
        memcpy(combined + hlit, plan->dist_lengths_full, hdist);
        if (!rle_code_lengths(combined, hlit + hdist, &plan->rle)) {
            plan_free(plan);
            return 0;
        }
    }

    memset(cl_freq, 0, sizeof cl_freq);
    for (i = 0; i < plan->rle.count; i++) {
        cl_freq[plan->rle.items[i].sym]++;
    }
    cl_len = length_limited_huffman(cl_freq, 19, 7);
    if (!cl_len) {
        plan_free(plan);
        return 0;
    }
    if (!kraft_sum_ok(cl_len, 19, 7)) {
        free(cl_len);
        plan_free(plan);
        return 0;
    }
    memcpy(plan->cl_lengths, cl_len, 19);
    free(cl_len);
    huff_build_canonical_codes(plan->cl_lengths, 19, plan->cl_codes);

    plan->cl_order_count = 19;
    while (plan->cl_order_count > 4 &&
          plan->cl_lengths[CL_PERMUTATION[plan->cl_order_count - 1]] == 0) {
        plan->cl_order_count--;
    }

    {
        unsigned long bits = 3; /* BFINAL + BTYPE */
        bits += 5 + 5 + 4;      /* HLIT + HDIST + HCLEN */
        bits += plan->cl_order_count * 3;
        for (i = 0; i < plan->rle.count; i++) {
            bits += plan->cl_lengths[plan->rle.items[i].sym];
            bits += plan->rle.items[i].extra_bits;
        }
        for (i = 0; i < count; i++) {
            if (!tokens[i].is_match) {
                bits += plan->ll_lengths_full[tokens[i].literal];
            } else {
                unsigned short lsym = length_symbol(tokens[i].length);
                unsigned short dc = dist_code_for(tokens[i].offset);
                bits += plan->ll_lengths_full[lsym];
                bits += length_extra_of(lsym);
                bits += plan->dist_lengths_full[dc];
                bits += dist_extra_of(dc);
            }
        }
        bits += plan->ll_lengths_full[256]; /* EOB */
        plan->total_bits = bits;
    }

    return 1;
}

/* ===========================================================================
 * 8. Block emitters and deflate_compress
 * ===========================================================================
 */
static unsigned long fixed_block_bits(const LzssToken *tokens, size_t count) {
    unsigned long bits = 3; /* BFINAL + BTYPE */
    size_t i;
    uint32_t code;
    unsigned nbits;
    for (i = 0; i < count; i++) {
        if (!tokens[i].is_match) {
            fixed_ll_code(tokens[i].literal, &code, &nbits);
            bits += nbits;
        } else {
            unsigned short lsym = length_symbol(tokens[i].length);
            unsigned short dc = dist_code_for(tokens[i].offset);
            fixed_ll_code(lsym, &code, &nbits);
            bits += nbits + length_extra_of(lsym);
            bits += 5 + dist_extra_of(dc); /* fixed distance codes are 5 bits */
        }
    }
    fixed_ll_code(256, &code, &nbits);
    bits += nbits;
    return bits;
}

static void emit_fixed_block(BitWriter *w, const LzssToken *tokens, size_t count) {
    size_t i;
    uint32_t code;
    unsigned nbits;

    bw_write_bits_lsb(w, 1, 1); /* BFINAL = 1 */
    bw_write_bits_lsb(w, 1, 2); /* BTYPE  = 01 (fixed) */

    for (i = 0; i < count; i++) {
        if (!tokens[i].is_match) {
            fixed_ll_code(tokens[i].literal, &code, &nbits);
            bw_write_huffman(w, code, nbits);
        } else {
            unsigned short lsym = length_symbol(tokens[i].length);
            unsigned short dc = dist_code_for(tokens[i].offset);
            fixed_ll_code(lsym, &code, &nbits);
            bw_write_huffman(w, code, nbits);
            bw_write_bits_lsb(w, (uint32_t)(tokens[i].length - length_base_of(lsym)),
                              length_extra_of(lsym));
            bw_write_huffman(w, dc, 5);
            bw_write_bits_lsb(w, (uint32_t)(tokens[i].offset - dist_base_of(dc)),
                              dist_extra_of(dc));
        }
    }

    fixed_ll_code(256, &code, &nbits);
    bw_write_huffman(w, code, nbits);
}

static void emit_dynamic_block(BitWriter *w, const LzssToken *tokens, size_t count,
                               const DynamicPlan *plan) {
    size_t i;

    bw_write_bits_lsb(w, 1, 1); /* BFINAL = 1 */
    bw_write_bits_lsb(w, 2, 2); /* BTYPE  = 10 (dynamic) */

    bw_write_bits_lsb(w, (uint32_t)(plan->hlit - 257), 5);
    bw_write_bits_lsb(w, (uint32_t)(plan->hdist - 1), 5);
    bw_write_bits_lsb(w, (uint32_t)(plan->cl_order_count - 4), 4);

    for (i = 0; i < plan->cl_order_count; i++) {
        bw_write_bits_lsb(w, plan->cl_lengths[CL_PERMUTATION[i]], 3);
    }

    for (i = 0; i < plan->rle.count; i++) {
        const ClItem *it = &plan->rle.items[i];
        bw_write_huffman(w, plan->cl_codes[it->sym], plan->cl_lengths[it->sym]);
        if (it->extra_bits > 0) {
            bw_write_bits_lsb(w, it->extra_val, it->extra_bits);
        }
    }

    for (i = 0; i < count; i++) {
        if (!tokens[i].is_match) {
            unsigned char lit = tokens[i].literal;
            bw_write_huffman(w, plan->ll_codes[lit], plan->ll_lengths_full[lit]);
        } else {
            unsigned short lsym = length_symbol(tokens[i].length);
            unsigned short dc = dist_code_for(tokens[i].offset);
            bw_write_huffman(w, plan->ll_codes[lsym], plan->ll_lengths_full[lsym]);
            bw_write_bits_lsb(w, (uint32_t)(tokens[i].length - length_base_of(lsym)),
                              length_extra_of(lsym));
            bw_write_huffman(w, plan->dist_codes[dc], plan->dist_lengths_full[dc]);
            bw_write_bits_lsb(w, (uint32_t)(tokens[i].offset - dist_base_of(dc)),
                              dist_extra_of(dc));
        }
    }

    bw_write_huffman(w, plan->ll_codes[256], plan->ll_lengths_full[256]);
}

DeflateStatus deflate_compress(const unsigned char *data, size_t len,
                               unsigned char **out, size_t *out_len) {
    LzssToken *tokens = NULL;
    size_t count = 0;
    unsigned long fixed_bits;
    DynamicPlan plan;
    int have_plan;
    BitWriter w;

    *out = NULL;
    *out_len = 0;

    /* LZSS pass (CMP02, `c/lzss`): the full RFC 1951 window (32768), our
     * fixed max_match (255, so the encoder never needs LL symbol 285) and
     * min_match (3). */
    if (lzss_encode(data, len, 32768, 255, 3, &tokens, &count) != LZSS_OK) {
        return DEFLATE_ERR_ALLOC;
    }

    fixed_bits = fixed_block_bits(tokens, count);
    have_plan = plan_dynamic(tokens, count, &plan);

    bw_init(&w);
    if (have_plan && plan.total_bits < fixed_bits) {
        emit_dynamic_block(&w, tokens, count, &plan);
    } else {
        emit_fixed_block(&w, tokens, count);
    }
    if (have_plan) {
        plan_free(&plan);
    }
    free(tokens);

    bw_flush(&w);
    if (!w.out.ok) {
        free(w.out.data);
        return DEFLATE_ERR_ALLOC;
    }

    *out = w.out.data;
    *out_len = w.out.len;
    return DEFLATE_OK;
}

void deflate_free(unsigned char *buf) {
    free(buf);
}

/* ===========================================================================
 * 9. Block decoders and deflate_decompress
 * ===========================================================================
 */

/* Copy `length` bytes from `dist` bytes behind the current end of `out`,
 * byte-by-byte (not a bulk copy) so overlapping back-references — the
 * `dist < length` case DEFLATE relies on to encode long runs cheaply — read
 * freshly-written bytes correctly.
 *
 * Bounds enforced (untrusted input): `dist` must not reach further back than
 * bytes already produced (else the reference is dangling), and growing by
 * `length` must not cross DEFLATE_MAX_OUTPUT (decompression-bomb guard). */
static int copy_back_ref(ByteBuf *out, size_t dist, size_t length) {
    size_t start, i;
    if (dist == 0 || dist > out->len) {
        return 0;
    }
    if (out->len + length > DEFLATE_MAX_OUTPUT) {
        return 0;
    }
    start = out->len - dist;
    for (i = 0; i < length; i++) {
        bb_push(out, out->data[start + i]);
        if (!out->ok) {
            return 0;
        }
    }
    return 1;
}

/* Shared decode loop for fixed and dynamic blocks: decode an LL symbol; <256
 * is a literal, 256 ends the block, 257-285 is a length code followed by a
 * distance code and a back-reference copy. Returns 0 on any malformed input
 * or allocation failure (`out->ok` distinguishes which, for the caller's
 * DeflateStatus). */
static int decode_block(const HuffDecodeTable *ll_table, const HuffDecodeTable *dist_table,
                        BitReader *r, ByteBuf *out) {
    for (;;) {
        unsigned short sym;
        if (!huff_decode_symbol(r, ll_table, &sym)) {
            return 0;
        }

        if (sym < 256) {
            if (out->len >= DEFLATE_MAX_OUTPUT) {
                return 0;
            }
            bb_push(out, (unsigned char)sym);
            if (!out->ok) {
                return 0;
            }
        } else if (sym == 256) {
            return 1;
        } else {
            size_t length_idx = (size_t)(sym - 257);
            uint32_t extra_len, extra_dist;
            unsigned short dist_sym;
            size_t length, dist;

            if (length_idx >= LENGTH_TABLE_SIZE) {
                return 0; /* symbol 286/287: reserved, never a valid LL code */
            }
            if (!br_read_bits(r, LENGTH_TABLE[length_idx].extra, &extra_len)) {
                return 0;
            }
            length = (size_t)LENGTH_TABLE[length_idx].base + extra_len;

            if (!huff_decode_symbol(r, dist_table, &dist_sym)) {
                return 0;
            }
            if ((size_t)dist_sym >= DIST_TABLE_SIZE) {
                return 0; /* dist codes 30/31: reserved, never valid */
            }
            if (!br_read_bits(r, DIST_TABLE[dist_sym].extra, &extra_dist)) {
                return 0;
            }
            dist = (size_t)DIST_TABLE[dist_sym].base + extra_dist;

            if (!copy_back_ref(out, dist, length)) {
                return 0;
            }
        }
    }
}

/* Decode `total` code lengths from the CL (meta) tree, per RFC 1951 §3.2.7:
 *   0-15 -> a literal code length
 *   16   -> repeat the previous length, 3 + read_bits(2) times
 *   17   -> emit zeros, 3 + read_bits(3) times
 *   18   -> emit zeros, 11 + read_bits(7) times
 * Returns 0 on truncated input, an invalid CL symbol, or a repeat/run that
 * would overflow `total` (all DEFLATE_ERR_MALFORMED). */
static int decode_code_lengths(const HuffDecodeTable *cl_table, BitReader *r,
                               size_t total, unsigned char *lengths) {
    size_t n = 0;
    unsigned char prev = 0;
    while (n < total) {
        unsigned short sym;
        if (!huff_decode_symbol(r, cl_table, &sym)) {
            return 0;
        }
        if (sym <= 15) {
            prev = (unsigned char)sym;
            lengths[n++] = prev;
        } else if (sym == 16) {
            uint32_t rep;
            size_t k;
            if (!br_read_bits(r, 2, &rep)) return 0;
            for (k = 0; k < rep + 3; k++) {
                if (n >= total) return 0;
                lengths[n++] = prev;
            }
        } else if (sym == 17) {
            uint32_t rep;
            size_t k;
            if (!br_read_bits(r, 3, &rep)) return 0;
            for (k = 0; k < rep + 3; k++) {
                if (n >= total) return 0;
                lengths[n++] = 0;
            }
            prev = 0;
        } else if (sym == 18) {
            uint32_t rep;
            size_t k;
            if (!br_read_bits(r, 7, &rep)) return 0;
            for (k = 0; k < rep + 11; k++) {
                if (n >= total) return 0;
                lengths[n++] = 0;
            }
            prev = 0;
        } else {
            return 0; /* CL alphabet is only 0-18 */
        }
    }
    return 1;
}

DeflateStatus deflate_decompress(const unsigned char *data, size_t len,
                                 unsigned char **out, size_t *out_len) {
    BitReader r;
    ByteBuf output;
    int bfinal;

    *out = NULL;
    *out_len = 0;
    br_init(&r, data, len);
    bb_init(&output);

    do {
        uint32_t bfinal_bits, btype_bits;

        if (!br_read_bits(&r, 1, &bfinal_bits) || !br_read_bits(&r, 2, &btype_bits)) {
            free(output.data);
            return DEFLATE_ERR_MALFORMED;
        }
        bfinal = (int)bfinal_bits;

        if (btype_bits == 0) {
            /* ── Stored block: byte-align, then LEN/NLEN (16-bit LE) + LEN
             * verbatim bytes. LEN is inherently bounded to 65535, so there is
             * no bomb risk from this branch alone; the running-total check
             * still guards against many stored blocks in sequence. */
            unsigned char lo, hi;
            uint16_t block_len, nlen;
            size_t i;

            br_align_to_byte(&r);
            if (!br_read_byte(&r, &lo) || !br_read_byte(&r, &hi)) {
                free(output.data);
                return DEFLATE_ERR_MALFORMED;
            }
            block_len = (uint16_t)(lo | (hi << 8));
            if (!br_read_byte(&r, &lo) || !br_read_byte(&r, &hi)) {
                free(output.data);
                return DEFLATE_ERR_MALFORMED;
            }
            nlen = (uint16_t)(lo | (hi << 8));
            if ((uint16_t)(~block_len & 0xFFFFu) != nlen) {
                free(output.data);
                return DEFLATE_ERR_MALFORMED;
            }
            if (output.len + block_len > DEFLATE_MAX_OUTPUT) {
                free(output.data);
                return DEFLATE_ERR_MALFORMED;
            }
            for (i = 0; i < block_len; i++) {
                unsigned char b;
                if (!br_read_byte(&r, &b)) {
                    free(output.data);
                    return DEFLATE_ERR_MALFORMED;
                }
                bb_push(&output, b);
            }
            if (!output.ok) {
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }
        } else if (btype_bits == 1) {
            /* ── Fixed Huffman: pre-agreed §3.2.6 tables, nothing transmitted. */
            unsigned char ll_lens[288], dist_lens[32];
            HuffDecodeTable llt, dt;
            int ok;

            fixed_ll_lengths(ll_lens);
            fixed_dist_lengths(dist_lens);
            if (!huff_build_decode_table(ll_lens, 288, &llt)) {
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }
            if (!huff_build_decode_table(dist_lens, 32, &dt)) {
                huff_free_decode_table(&llt);
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }
            ok = decode_block(&llt, &dt, &r, &output);
            huff_free_decode_table(&llt);
            huff_free_decode_table(&dt);
            if (!ok) {
                DeflateStatus st = output.ok ? DEFLATE_ERR_MALFORMED : DEFLATE_ERR_ALLOC;
                free(output.data);
                return st;
            }
        } else if (btype_bits == 2) {
            /* ── Dynamic Huffman: read HLIT/HDIST/HCLEN, then the CL tree,
             * then decode the LL and dist trees' code lengths through it. */
            uint32_t hlit_b, hdist_b, hclen_b;
            size_t hlit, hdist, hclen, i;
            unsigned char cl_lengths[19];
            HuffDecodeTable clt, llt, dt;
            unsigned char *all_lengths;
            int ok;

            if (!br_read_bits(&r, 5, &hlit_b) || !br_read_bits(&r, 5, &hdist_b) ||
                !br_read_bits(&r, 4, &hclen_b)) {
                free(output.data);
                return DEFLATE_ERR_MALFORMED;
            }
            hlit = (size_t)hlit_b + 257;
            hdist = (size_t)hdist_b + 1;
            hclen = (size_t)hclen_b + 4;

            memset(cl_lengths, 0, sizeof cl_lengths);
            for (i = 0; i < hclen; i++) {
                uint32_t v;
                if (!br_read_bits(&r, 3, &v)) {
                    free(output.data);
                    return DEFLATE_ERR_MALFORMED;
                }
                cl_lengths[CL_PERMUTATION[i]] = (unsigned char)v;
            }
            if (!huff_build_decode_table(cl_lengths, 19, &clt)) {
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }

            /* hlit+hdist <= 288+32 = 320: a small, input-independent bound
             * (HLIT/HDIST are 5-bit fields), not attacker-scalable. */
            all_lengths = (unsigned char *)malloc(hlit + hdist);
            if (!all_lengths) {
                huff_free_decode_table(&clt);
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }
            ok = decode_code_lengths(&clt, &r, hlit + hdist, all_lengths);
            huff_free_decode_table(&clt);
            if (!ok) {
                free(all_lengths);
                free(output.data);
                return DEFLATE_ERR_MALFORMED;
            }

            if (!huff_build_decode_table(all_lengths, hlit, &llt)) {
                free(all_lengths);
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }
            if (!huff_build_decode_table(all_lengths + hlit, hdist, &dt)) {
                huff_free_decode_table(&llt);
                free(all_lengths);
                free(output.data);
                return DEFLATE_ERR_ALLOC;
            }
            free(all_lengths);

            ok = decode_block(&llt, &dt, &r, &output);
            huff_free_decode_table(&llt);
            huff_free_decode_table(&dt);
            if (!ok) {
                DeflateStatus st = output.ok ? DEFLATE_ERR_MALFORMED : DEFLATE_ERR_ALLOC;
                free(output.data);
                return st;
            }
        } else {
            /* BTYPE == 3 is reserved by RFC 1951 and never valid. */
            free(output.data);
            return DEFLATE_ERR_MALFORMED;
        }
    } while (!bfinal);

    *out = output.data;
    *out_len = output.len;
    return DEFLATE_OK;
}
