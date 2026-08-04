/*
 * wasm_leb128.h — LEB128 variable-length integer coding, in pure ISO C17. A
 * faithful port of the Rust `wasm-leb128` crate.
 * ===========================================================================
 *
 * LEB128 ("Little-Endian Base 128") is the varint format used by WebAssembly,
 * DWARF debug info, and Android DEX. Each byte carries 7 data bits in its low
 * bits; the high bit (0x80) is a continuation flag — set on every byte except
 * the last. Data is emitted least-significant group first.
 *
 *   624485 = 0b1001_1000_0011_1011_0010_0101
 *          -> 0xE5 0x8E 0x26   (three 7-bit groups, low group first)
 *
 * Unsigned values are zero-extended. SIGNED values use two's complement: the
 * encoder stops once the remaining bits are all-0 (positive) or all-1
 * (negative) AND the sign bit of the last group agrees, and the decoder
 * sign-extends from the last group's bit 6.
 *
 *   -2 -> 0x7E (one byte; bit 6 set, so decode sign-extends to -2)
 *
 * Encoding never fails: a u64 or i64 needs at most 10 bytes (LEB128_MAX_BYTES).
 * Decoding can fail three ways — see Leb128Status.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef WASM_LEB128_H
#define WASM_LEB128_H

#include <stddef.h> /* size_t */

/* The most bytes any 64-bit value can occupy: ceil(64 / 7) = 10. Encode buffers
 * must be at least this large. */
#define LEB128_MAX_BYTES 10

/* Outcome of a decode. */
typedef enum {
    LEB128_OK = 0,
    LEB128_ERR_OFFSET,       /* offset is past the end of the data */
    LEB128_ERR_OVERFLOW,     /* sequence exceeds the 64-bit (70-bit) width */
    LEB128_ERR_UNTERMINATED  /* ran out of bytes with the continuation flag set */
} Leb128Status;

/* leb128_status_message — a static human-readable description of `status`. */
const char *leb128_status_message(Leb128Status status);

/* ---- encoding (returns the number of bytes written, 1..LEB128_MAX_BYTES) --- */

/* leb128_encode_unsigned — write the unsigned LEB128 encoding of `value` into
 * `out` (which must hold at least LEB128_MAX_BYTES bytes). */
size_t leb128_encode_unsigned(unsigned long long value, unsigned char *out);

/* leb128_encode_signed — write the signed (two's complement) LEB128 encoding of
 * `value` into `out` (at least LEB128_MAX_BYTES bytes). */
size_t leb128_encode_signed(long long value, unsigned char *out);

/* ---- decoding --------------------------------------------------------- */

/* leb128_decode_unsigned — decode an unsigned LEB128 value from
 * `data[offset .. len]`. On LEB128_OK, *value holds the result and
 * *bytes_consumed the number of bytes read; otherwise both are set to 0. */
Leb128Status leb128_decode_unsigned(const unsigned char *data, size_t len,
                                    size_t offset, unsigned long long *value,
                                    size_t *bytes_consumed);

/* leb128_decode_signed — decode a signed LEB128 value from
 * `data[offset .. len]`, sign-extending as needed. Same output contract. */
Leb128Status leb128_decode_signed(const unsigned char *data, size_t len,
                                  size_t offset, long long *value,
                                  size_t *bytes_consumed);

#endif /* WASM_LEB128_H */
