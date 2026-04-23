/*
 * bitset_c.h -- C declarations for the Rust bitset-c library.
 *
 * This header describes the stable C ABI exposed by the Rust `bitset-c`
 * wrapper crate. The underlying implementation lives in the pure Rust
 * `bitset` crate; this layer simply translates between native callers and the
 * Rust data structure.
 *
 * HANDLE MODEL
 * ------------
 * Bitsets are opaque heap-allocated handles. Create them with one of the
 * constructor functions and release them with `bitset_c_free`.
 *
 * ERROR MODEL
 * -----------
 * Functions that fail return a sentinel value (`NULL`, `0`, or false) and set
 * thread-local error information. Read it immediately via:
 *
 *   bitset_c_had_error()
 *   bitset_c_last_error_code()
 *   bitset_c_last_error_message()
 *
 * Error codes:
 *   0 = no error
 *   1 = invalid binary string
 *   2 = null pointer
 *   3 = invalid UTF-8
 *   4 = value too large for this platform
 *   5 = caught Rust panic
 */

#ifndef BITSET_C_H
#define BITSET_C_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct bitset_c_handle bitset_c_handle;

bitset_c_handle *bitset_c_new(uint64_t size);
bitset_c_handle *bitset_c_from_u128(uint64_t low, uint64_t high);
bitset_c_handle *bitset_c_from_binary_str(const char *binary);
void bitset_c_free(bitset_c_handle *handle);

void bitset_c_set(bitset_c_handle *handle, uint64_t index);
void bitset_c_clear(bitset_c_handle *handle, uint64_t index);
uint8_t bitset_c_test(const bitset_c_handle *handle, uint64_t index);
void bitset_c_toggle(bitset_c_handle *handle, uint64_t index);

bitset_c_handle *bitset_c_and(const bitset_c_handle *left, const bitset_c_handle *right);
bitset_c_handle *bitset_c_or(const bitset_c_handle *left, const bitset_c_handle *right);
bitset_c_handle *bitset_c_xor(const bitset_c_handle *left, const bitset_c_handle *right);
bitset_c_handle *bitset_c_not(const bitset_c_handle *handle);
bitset_c_handle *bitset_c_and_not(const bitset_c_handle *left, const bitset_c_handle *right);

uint64_t bitset_c_popcount(const bitset_c_handle *handle);
uint64_t bitset_c_len(const bitset_c_handle *handle);
uint64_t bitset_c_capacity(const bitset_c_handle *handle);
uint8_t bitset_c_any(const bitset_c_handle *handle);
uint8_t bitset_c_all(const bitset_c_handle *handle);
uint8_t bitset_c_none(const bitset_c_handle *handle);
uint8_t bitset_c_is_empty(const bitset_c_handle *handle);
uint8_t bitset_c_to_u64(const bitset_c_handle *handle, uint64_t *out);
uint8_t bitset_c_equals(const bitset_c_handle *left, const bitset_c_handle *right);

uint8_t bitset_c_had_error(void);
uint32_t bitset_c_last_error_code(void);
const char *bitset_c_last_error_message(void);

#ifdef __cplusplus
}
#endif

#endif /* BITSET_C_H */
