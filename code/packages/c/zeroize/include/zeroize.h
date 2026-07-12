/*
 * zeroize.h — secure in-memory wiping for secrets, in pure ISO C17. A faithful
 * port of the Rust `zeroize` crate.
 * ===========================================================================
 *
 * When a program clears a secret (a key, a password, a token) by writing zeros,
 * an optimizing compiler is allowed to DELETE the clear: from its point of view
 * nothing reads the buffer afterward, so the store is "dead". The secret then
 * lingers in RAM — exposed to a later swap-to-disk, core dump, or use-after-free
 * elsewhere.
 *
 * To make the clear *observably happen*, each store must be VOLATILE. The C
 * standard classifies a volatile access as observable behavior that the
 * implementation may not optimize away — which is exactly the guarantee we need.
 * `zeroize_bytes` writes zeros through a `volatile unsigned char *`, the
 * canonical portable secure-zero (the same technique used by `memset_s`-style
 * fallbacks in crypto libraries).
 *
 * DIVERGENCE FROM RUST. Rust pairs `write_volatile` with a `compiler_fence`;
 * this C port relies on volatile stores alone (the load-bearing defense against
 * dead-store elimination), omitting an explicit fence for maximal MSVC
 * portability. Rust's `Zeroizing<T>` RAII wrapper and `Option` impl are
 * language features with no C analogue — use `zeroize_bytes` before `free`. The
 * `ZrBytes` growable buffer models the Rust `Vec<u8>` impl (it scrubs the FULL
 * capacity, not just the live length).
 *
 * PORTABILITY. Pure ISO C17 — no compiler extensions, no <math.h>. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_ZEROIZE_H
#define CA_ZEROIZE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define ZEROIZE_VERSION "0.1.0"

/* The primitive: overwrite `len` bytes at `ptr` with 0 using volatile stores so
 * the compiler may not elide the clear. A NULL ptr is allowed only when len is
 * 0 (a no-op). Everything else is built on this. */
void zeroize_bytes(void *ptr, size_t len);

/* Zeroize a single object of known size (e.g. zeroize_object(&key, sizeof key)).
 * Equivalent to zeroize_bytes; provided for intent. */
void zeroize_object(void *ptr, size_t len);

/* Typed convenience wipes for the fixed-width integers (each a volatile 0
 * store). 128-bit integers are intentionally omitted: pure ISO C has no such
 * type — wipe those via zeroize_bytes(&x, sizeof x). */
void zeroize_u8(uint8_t *p);
void zeroize_u16(uint16_t *p);
void zeroize_u32(uint32_t *p);
void zeroize_u64(uint64_t *p);
void zeroize_i8(int8_t *p);
void zeroize_i16(int16_t *p);
void zeroize_i32(int32_t *p);
void zeroize_i64(int64_t *p);
void zeroize_size(size_t *p);

/* A growable byte buffer whose `zeroize` scrubs the entire allocated capacity
 * (mirroring the Rust `Vec<u8>` impl), then resets the logical length to 0. */
typedef struct {
    unsigned char *data;
    size_t len;
    size_t cap;
} ZrBytes;

void zr_bytes_init(ZrBytes *b);
/* Ensure room for at least `additional` more bytes. Returns 0 or -1 on OOM. */
int zr_bytes_reserve(ZrBytes *b, size_t additional);
int zr_bytes_push(ZrBytes *b, unsigned char byte);
int zr_bytes_extend(ZrBytes *b, const unsigned char *src, size_t n);
/* Scrub the FULL capacity with volatile zeros and set len = 0 (keeps the
 * allocation, exactly like the Rust Vec::zeroize). */
void zr_bytes_zeroize(ZrBytes *b);
/* Release the allocation (does NOT scrub — call zr_bytes_zeroize first for a
 * secret). Safe on a zeroed struct. */
void zr_bytes_free(ZrBytes *b);

#ifdef __cplusplus
}
#endif

#endif /* CA_ZEROIZE_H */
