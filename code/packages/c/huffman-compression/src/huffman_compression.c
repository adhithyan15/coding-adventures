/*
 * huffman_compression.c — Huffman compression with canonical codes and the
 * CMP04 wire format. Ported from the Rust `huffman-compression` crate.
 *
 * The Huffman tree is built in a FIXED array (at most 511 nodes for a 256-symbol
 * alphabet), so there is no dynamic tree and no pointer ownership to get wrong —
 * only the output buffers are heap-allocated.
 */
#include "huffman_compression.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, realloc, free, qsort */
#include <string.h> /* memcpy */

/* ── growable byte buffer ─────────────────────────────────────────────────── */
typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
    int failed;
} bytebuf;

static void bb_push(bytebuf *b, uint8_t byte) {
    if (b->failed) {
        return;
    }
    if (b->len == b->cap) {
        size_t new_cap = b->cap == 0 ? 64 : b->cap * 2;
        uint8_t *grown;
        if (b->cap > SIZE_MAX / 2) {
            b->failed = 1;
            return;
        }
        grown = (uint8_t *)realloc(b->data, new_cap);
        if (grown == NULL) {
            b->failed = 1;
            return;
        }
        b->data = grown;
        b->cap = new_cap;
    }
    b->data[b->len++] = byte;
}

/* ── array-based Huffman tree, for computing per-symbol code lengths ──────── */
typedef struct {
    uint32_t weight;
    int left;   /* child node index, or -1 */
    int right;  /* child node index, or -1 */
    int symbol; /* 0..255 for a leaf, -1 for internal */
    int order;  /* insertion order, for deterministic tie-breaking */
} hnode;

/* A tiny binary min-heap of node indices, ordered by (weight, order). */
typedef struct {
    int idx[512];
    int size;
    const hnode *nodes;
} nheap;

static int node_less(const hnode *nodes, int a, int b) {
    if (nodes[a].weight != nodes[b].weight) {
        return nodes[a].weight < nodes[b].weight;
    }
    return nodes[a].order < nodes[b].order;
}

static void nheap_push(nheap *h, int node) {
    int i = h->size++;
    h->idx[i] = node;
    while (i > 0) {
        int parent = (i - 1) / 2;
        if (node_less(h->nodes, h->idx[i], h->idx[parent])) {
            int t = h->idx[i];
            h->idx[i] = h->idx[parent];
            h->idx[parent] = t;
            i = parent;
        } else {
            break;
        }
    }
}

static int nheap_pop(nheap *h) {
    int top = h->idx[0];
    int i = 0;
    h->idx[0] = h->idx[--h->size];
    for (;;) {
        int l = 2 * i + 1, r = 2 * i + 2, best = i;
        if (l < h->size && node_less(h->nodes, h->idx[l], h->idx[best])) {
            best = l;
        }
        if (r < h->size && node_less(h->nodes, h->idx[r], h->idx[best])) {
            best = r;
        }
        if (best == i) {
            break;
        }
        {
            int t = h->idx[i];
            h->idx[i] = h->idx[best];
            h->idx[best] = t;
        }
        i = best;
    }
    return top;
}

/* Fill code_len[0..255] with each present symbol's code length (0 if absent). */
static void compute_code_lengths(const uint32_t freq[256], uint8_t code_len[256]) {
    hnode nodes[512];
    int n = 0;
    nheap heap;
    int order = 0;
    int distinct = 0;
    int sym;

    for (sym = 0; sym < 256; sym++) {
        code_len[sym] = 0;
    }
    heap.size = 0;
    heap.nodes = nodes;
    for (sym = 0; sym < 256; sym++) {
        if (freq[sym] > 0) {
            nodes[n].weight = freq[sym];
            nodes[n].left = -1;
            nodes[n].right = -1;
            nodes[n].symbol = sym;
            nodes[n].order = order++;
            nheap_push(&heap, n);
            n++;
            distinct++;
        }
    }
    if (distinct == 0) {
        return;
    }
    if (distinct == 1) {
        code_len[nodes[0].symbol] = 1; /* a lone symbol still needs one bit */
        return;
    }
    while (heap.size > 1) {
        int a = nheap_pop(&heap);
        int b = nheap_pop(&heap);
        nodes[n].weight = nodes[a].weight + nodes[b].weight;
        nodes[n].left = a;
        nodes[n].right = b;
        nodes[n].symbol = -1;
        nodes[n].order = order++;
        nheap_push(&heap, n);
        n++;
    }
    /* DFS the tree; a leaf's depth is its code length. Iterative to avoid deep
     * recursion (depth is bounded by the alphabet size). */
    {
        int stack_node[512];
        int stack_depth[512];
        int sp = 0;
        stack_node[sp] = nheap_pop(&heap); /* root */
        stack_depth[sp] = 0;
        sp++;
        while (sp > 0) {
            int node = stack_node[--sp];
            int depth = stack_depth[sp];
            if (nodes[node].symbol >= 0) {
                code_len[nodes[node].symbol] = (uint8_t)(depth == 0 ? 1 : depth);
            } else {
                stack_node[sp] = nodes[node].left;
                stack_depth[sp] = depth + 1;
                sp++;
                stack_node[sp] = nodes[node].right;
                stack_depth[sp] = depth + 1;
                sp++;
            }
        }
    }
}

/* A (symbol, length) table entry, sorted by (length, symbol). */
typedef struct {
    uint16_t symbol;
    uint8_t length;
} sym_len;

static int cmp_sym_len(const void *pa, const void *pb) {
    const sym_len *a = (const sym_len *)pa;
    const sym_len *b = (const sym_len *)pb;
    if (a->length != b->length) {
        return a->length < b->length ? -1 : 1;
    }
    return a->symbol < b->symbol ? -1 : (a->symbol > b->symbol ? 1 : 0);
}

/* Longest code length either side of the wire accepts.
 *
 * WHY 56 (and why the encoder never needs length-limiting):
 *   A Huffman tree of depth d needs a total weight of at least Fibonacci(d + 2)
 *   (the worst case is the "Fibonacci" frequency distribution 1,1,2,3,5,8,...).
 *   The CMP04 header stores the input length in a u32, so the total weight is
 *   at most 2^32 - 1, and Fibonacci(48) = 4,807,526,976 > 2^32 caps the depth at
 *   46. 46 <= 56, so with `len <= UINT32_MAX` every code the encoder builds fits
 *   in the 64-bit code accumulator with 8+ bits to spare, and the decoder's
 *   1..56 limit can never reject encoder output. (A depth of 33..46 DOES occur,
 *   for ~15 MB of Fibonacci-distributed bytes, and overflowed the previous
 *   32-bit code values.) No package-merge or zlib-style length cap is needed,
 *   and because nothing is capped the canonical codes (hence the output bytes)
 *   are unchanged for every input the old encoder handled correctly. */
#define HUFFMAN_MAX_CODE_LEN 56u

/* Assign canonical codes (DEFLATE-style) to the sorted table. codes[i] is the
 * code for table[i], right-justified in the low `table[i].length` bits.
 *
 * Codes are 64-bit: a code longer than 32 bits does not fit in a uint32_t, and
 * shifting a 32-bit value by >= 32 is undefined behaviour. Callers guarantee
 * every length is in 1..HUFFMAN_MAX_CODE_LEN, so the code stays below 2^56. */
static void canonical_codes(const sym_len *table, size_t n, uint64_t *codes) {
    size_t i;
    uint64_t code = 0;
    if (n == 0) {
        return;
    }
    codes[0] = 0;
    for (i = 1; i < n; i++) {
        code = (code + 1) << (table[i].length - table[i - 1].length);
        codes[i] = code;
    }
}

int huffman_compress(const uint8_t *data, size_t len, uint8_t **out,
                     size_t *out_len) {
    uint32_t freq[256];
    uint8_t code_len[256];
    sym_len table[256];
    uint64_t codes[256];
    size_t n = 0;
    bytebuf bits; /* the LSB-first packed bit stream */
    unsigned bit_acc = 0, bit_cnt = 0;
    size_t i;
    /* Per-symbol code + length, indexed by byte value for fast encode lookup. */
    uint64_t sym_code[256];
    uint8_t sym_len_by_byte[256];

#if SIZE_MAX > 0xFFFFFFFFu
    /* The header's length field and the frequency counters are 32-bit. Refuse
     * larger inputs instead of silently truncating (which would also void the
     * depth bound documented at HUFFMAN_MAX_CODE_LEN). */
    if (len > 0xFFFFFFFFu) {
        return 0;
    }
#endif

    if (len == 0) {
        /* Empty input → an 8-byte header of zeros. */
        uint8_t *buf = (uint8_t *)calloc(8, 1);
        if (buf == NULL) {
            return 0;
        }
        *out = buf;
        *out_len = 8;
        return 1;
    }

    memset(freq, 0, sizeof freq);
    for (i = 0; i < len; i++) {
        freq[data[i]]++;
    }
    compute_code_lengths(freq, code_len);

    for (i = 0; i < 256; i++) {
        /* Defence in depth: unreachable for len <= UINT32_MAX (depth <= 46),
         * but never emit a table the decoder would reject. */
        if (code_len[i] > HUFFMAN_MAX_CODE_LEN) {
            return 0;
        }
        if (code_len[i] > 0) {
            table[n].symbol = (uint16_t)i;
            table[n].length = code_len[i];
            n++;
        }
    }
    qsort(table, n, sizeof(sym_len), cmp_sym_len);
    canonical_codes(table, n, codes);
    for (i = 0; i < n; i++) {
        sym_code[table[i].symbol] = codes[i];
        sym_len_by_byte[table[i].symbol] = table[i].length;
    }

    /* Emit each byte's canonical code, most-significant bit first, into a
     * bit stream packed LSB-first. */
    bits.data = NULL;
    bits.len = 0;
    bits.cap = 0;
    bits.failed = 0;
    for (i = 0; i < len; i++) {
        uint8_t clen = sym_len_by_byte[data[i]];
        uint64_t ccode = sym_code[data[i]];
        int j;
        for (j = clen - 1; j >= 0; j--) {
            unsigned bit = (unsigned)((ccode >> j) & 1u);
            bit_acc |= bit << bit_cnt;
            bit_cnt++;
            if (bit_cnt == 8) {
                bb_push(&bits, (uint8_t)bit_acc);
                bit_acc = 0;
                bit_cnt = 0;
            }
        }
    }
    if (bit_cnt > 0) {
        bb_push(&bits, (uint8_t)bit_acc); /* zero-padded final byte */
    }
    if (bits.failed) {
        free(bits.data);
        return 0;
    }

    /* Assemble: 8-byte header + 2N-byte lengths table + bit stream. */
    {
        uint32_t original_length = (uint32_t)len;
        uint32_t symbol_count = (uint32_t)n;
        size_t total;
        uint8_t *buf;
        if (n > (SIZE_MAX - 8) / 2 || 8 + 2 * n > SIZE_MAX - bits.len) {
            free(bits.data);
            return 0;
        }
        total = 8 + 2 * n + bits.len;
        buf = (uint8_t *)malloc(total);
        if (buf == NULL) {
            free(bits.data);
            return 0;
        }
        buf[0] = (uint8_t)(original_length >> 24);
        buf[1] = (uint8_t)(original_length >> 16);
        buf[2] = (uint8_t)(original_length >> 8);
        buf[3] = (uint8_t)(original_length);
        buf[4] = (uint8_t)(symbol_count >> 24);
        buf[5] = (uint8_t)(symbol_count >> 16);
        buf[6] = (uint8_t)(symbol_count >> 8);
        buf[7] = (uint8_t)(symbol_count);
        for (i = 0; i < n; i++) {
            buf[8 + 2 * i] = (uint8_t)table[i].symbol;
            buf[8 + 2 * i + 1] = table[i].length;
        }
        memcpy(buf + 8 + 2 * n, bits.data, bits.len);
        free(bits.data);
        *out = buf;
        *out_len = total;
    }
    return 1;
}

/* Longest code length the decoder accepts. Real codes over 256 symbols are far
 * shorter; 56 keeps every 64-bit shift (and the code value itself) in range. */
#define HC_MAX_CODE_LEN 56u

/* Symbols are single bytes, so a valid table has at most 256 entries. */
#define HC_MAX_SYMBOLS 256u

int huffman_decompress(const uint8_t *data, size_t len, uint8_t **out,
                       size_t *out_len) {
    size_t original_length, symbol_count, table_end, i, payload_bytes;
    uint8_t syms[256];
    uint8_t seen[256];
    uint64_t first_code[HUFFMAN_MAX_CODE_LEN + 1];
    size_t counts[HUFFMAN_MAX_CODE_LEN + 1];
    size_t base[HUFFMAN_MAX_CODE_LEN + 1];
    uint64_t code = 0;
    unsigned prev_len = 0, max_len = 0;
    bytebuf output;
    size_t bit_pos = 0, total_bits;
    uint64_t cur = 0;
    unsigned cur_len = 0;
    size_t produced = 0;
    int ok = 1;

    if (len < 8) {
        return 0;
    }
    original_length = ((size_t)data[0] << 24) | ((size_t)data[1] << 16) |
                      ((size_t)data[2] << 8) | data[3];
    symbol_count = ((size_t)data[4] << 24) | ((size_t)data[5] << 16) |
                   ((size_t)data[6] << 8) | data[7];
    if (original_length == 0) {
        *out = NULL;
        *out_len = 0;
        return 1; /* empty */
    }
    /* SECURITY: the header is attacker-controlled. */
    if (symbol_count == 0 || symbol_count > 256) {
        return 0;
    }
    table_end = 8 + 2 * symbol_count;
    if (len < table_end) {
        return 0;
    }

    /* Rebuild canonical codes (DEFLATE style) while validating the hostile
     * table: no repeated symbol, length in 1..56, sorted by length, and not
     * over-subscribed (the running code must still fit in `length` bits).
     * Codes of one length are consecutive integers from first_code[L]; the
     * symbol for code c is syms[base[L] + (c - first_code[L])], so each probe is
     * O(1) rather than a scan of the whole table per bit. */
    memset(seen, 0, sizeof seen);
    memset(first_code, 0, sizeof first_code);
    memset(counts, 0, sizeof counts);
    memset(base, 0, sizeof base);
    for (i = 0; i < symbol_count; i++) {
        uint8_t symbol = data[8 + 2 * i];
        unsigned length = data[8 + 2 * i + 1];
        if (seen[symbol] || length == 0 || length > HUFFMAN_MAX_CODE_LEN ||
            (i > 0 && length < prev_len)) {
            return 0;
        }
        seen[symbol] = 1;
        if (i > 0) {
            code <<= (length - prev_len);
        }
        if ((code >> length) != 0) {
            return 0; /* over-subscribed */
        }
        if (counts[length] == 0) {
            first_code[length] = code;
            base[length] = i;
        }
        counts[length]++;
        syms[i] = symbol;
        code++;
        prev_len = length;
        max_len = length;
    }

    /* SECURITY: every code is >= 1 bit, so N payload bytes hold at most 8N
     * symbols. Reject a larger declared length BEFORE allocating from it. */
    payload_bytes = len - table_end;
    if (original_length / 8 + (original_length % 8 != 0) > payload_bytes) {
        return 0;
    }

    output.data = NULL;
    output.len = 0;
    output.cap = 0;
    output.failed = 0;

    /* Read the LSB-first bit stream, accumulating a code MSB-first. Reaching
     * the longest code length without a match means no later bit can match. */
    total_bits = payload_bytes * 8;
    while (produced < original_length) {
        unsigned bit;
        if (bit_pos >= total_bits) {
            ok = 0;
            break;
        }
        bit = (data[table_end + bit_pos / 8] >> (bit_pos % 8)) & 1u;
        bit_pos++;
        cur = (cur << 1) | bit;
        cur_len++;
        if (counts[cur_len] != 0 && cur >= first_code[cur_len] &&
            cur - first_code[cur_len] < counts[cur_len]) {
            bb_push(&output,
                    syms[base[cur_len] + (size_t)(cur - first_code[cur_len])]);
            produced++;
            cur = 0;
            cur_len = 0;
        } else if (cur_len >= max_len) {
            ok = 0; /* no valid code: malformed */
            break;
        }
    }
    if (!ok || output.failed || produced != original_length) {
        free(output.data);
        return 0;
    }
    *out = output.data;
    *out_len = output.len;
    return 1;
}
