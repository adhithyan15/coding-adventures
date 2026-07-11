/*
 * wasm_leb128.c — implementation of LEB128 coding (see wasm_leb128.h). A
 * faithful port of the Rust `wasm-leb128` crate.
 */
#include "wasm_leb128.h"

#include <limits.h> /* CHAR_BIT */
#include <string.h> /* memcpy */

/* LEB128_MAX_BYTES (10) assumes a value no wider than 70 bits (ceil(70/7)=10).
 * Fail loudly on an exotic platform with a wider `long long` rather than let an
 * encode overflow a caller's 10-byte buffer. */
_Static_assert(sizeof(unsigned long long) * CHAR_BIT <= 70,
               "LEB128_MAX_BYTES assumes a value width of at most 70 bits");

const char *leb128_status_message(Leb128Status status) {
    switch (status) {
        case LEB128_OK:
            return "ok";
        case LEB128_ERR_OFFSET:
            return "offset is out of bounds for the data";
        case LEB128_ERR_OVERFLOW:
            return "LEB128 sequence exceeds maximum 64-bit width (70 bits)";
        case LEB128_ERR_UNTERMINATED:
            return "unexpected end of data: LEB128 sequence is unterminated";
    }
    return "unknown";
}

/* Arithmetic right shift by 7 of a 64-bit signed value, spelled so the result
 * is well-defined regardless of the platform's signed-shift behaviour (it is
 * arithmetic on every target compiler, and C23 mandates it, but we do not rely
 * on that). */
static long long arith_shr7(long long v) {
    unsigned long long u;
    memcpy(&u, &v, sizeof u);
    u >>= 7;
    if (v < 0) {
        u |= ~(~0ULL >> 7); /* set the top 7 bits vacated by the shift */
    }
    memcpy(&v, &u, sizeof v);
    return v;
}

size_t leb128_encode_unsigned(unsigned long long value, unsigned char *out) {
    size_t n = 0;
    for (;;) {
        unsigned char byte = (unsigned char)(value & 0x7Fu);
        value >>= 7;
        if (value != 0) {
            byte |= 0x80u; /* more bytes follow */
        }
        out[n++] = byte;
        if (value == 0) {
            break;
        }
    }
    return n; /* <= LEB128_MAX_BYTES */
}

size_t leb128_encode_signed(long long value, unsigned char *out) {
    size_t n = 0;
    int done = 0;
    while (!done) {
        /* Low 7 bits of the two's-complement representation. */
        unsigned char byte = (unsigned char)((unsigned long long)value & 0x7Fu);
        value = arith_shr7(value);
        /* Done when no meaningful bits remain and the sign bit of this group
         * already reflects the sign (so decode neither drops nor spuriously
         * adds sign extension). */
        done = (value == 0 && (byte & 0x40) == 0) ||
               (value == -1 && (byte & 0x40) != 0);
        if (!done) {
            byte |= 0x80u;
        }
        out[n++] = byte;
    }
    return n; /* <= LEB128_MAX_BYTES */
}

Leb128Status leb128_decode_unsigned(const unsigned char *data, size_t len,
                                    size_t offset, unsigned long long *value,
                                    size_t *bytes_consumed) {
    unsigned long long val = 0;
    unsigned int shift = 0;
    size_t consumed = 0, i;
    *value = 0;
    *bytes_consumed = 0;
    if (offset >= len) {
        return LEB128_ERR_OFFSET;
    }
    for (i = offset; i < len; i++) {
        unsigned char byte = data[i];
        unsigned long long data_bits = (unsigned long long)(byte & 0x7Fu);
        val |= data_bits << shift; /* shift is <= 63 here, so this is defined */
        consumed++;
        shift += 7;
        if ((byte & 0x80u) == 0) {
            *value = val;
            *bytes_consumed = consumed;
            return LEB128_OK;
        }
        if (shift >= 70) {
            return LEB128_ERR_OVERFLOW;
        }
    }
    return LEB128_ERR_UNTERMINATED;
}

Leb128Status leb128_decode_signed(const unsigned char *data, size_t len,
                                  size_t offset, long long *value,
                                  size_t *bytes_consumed) {
    unsigned long long val = 0;
    unsigned int shift = 0;
    size_t consumed = 0, i;
    *value = 0;
    *bytes_consumed = 0;
    if (offset >= len) {
        return LEB128_ERR_OFFSET;
    }
    for (i = offset; i < len; i++) {
        unsigned char byte = data[i];
        unsigned long long data_bits = (unsigned long long)(byte & 0x7Fu);
        val |= data_bits << shift;
        consumed++;
        shift += 7;
        if ((byte & 0x80u) == 0) {
            long long signed_val;
            if (shift < 64 && (byte & 0x40u) != 0) {
                val |= (~(unsigned long long)0) << shift; /* sign-extend */
            }
            memcpy(&signed_val, &val, sizeof signed_val); /* two's complement */
            *value = signed_val;
            *bytes_consumed = consumed;
            return LEB128_OK;
        }
        if (shift >= 70) {
            return LEB128_ERR_OVERFLOW;
        }
    }
    return LEB128_ERR_UNTERMINATED;
}
