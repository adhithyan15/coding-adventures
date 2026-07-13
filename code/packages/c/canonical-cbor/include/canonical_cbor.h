/*
 * canonical_cbor.h — deterministic CBOR (RFC 8949) codec, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `canonical-cbor` crate. Encodes and decodes CBOR
 * values in a **canonical** (deterministic) profile so that decode(encode(v))
 * round-trips and encode(v) is the same bytes on every platform.
 *
 * ## Profile (RFC 8949 §4.2.3, "length-first map key ordering")
 *
 *   - Definite length only (indefinite-length items are rejected).
 *   - Smallest-form integer encoding (expanded forms rejected by the decoder).
 *   - Map keys sorted length-first, ties broken bytewise on the encoded key.
 *   - No floats (rejected); tags pass through opaquely; no `undefined`.
 *
 * ## Value model
 *
 * Every `CborValue` is heap-allocated and owns its children (mirroring the Rust
 * `Box`/`Vec` ownership). Build values with the constructors, compose them with
 * `cbor_array_push` / `cbor_map_push` / `cbor_tag` (each takes ownership of what
 * you pass), and release the whole tree with a single `cbor_free`.
 *
 * The struct is exposed so callers can inspect a decoded value directly
 * (`v->type`, `v->as.u`, `v->as.array.items[i]`, `v->as.map.entries[i].key`…).
 *
 * ## Divergences from Rust (documented)
 *
 *   - Rust `Vec<u8>` (encode) -> a malloc'd buffer + length the caller frees.
 *   - Rust `Result<CborValue, CborError>` (decode) -> a `CborStatus` code plus
 *     an owned out-parameter; an extra `CBOR_ERR_ALLOC` covers OOM.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no <math.h>, no compiler extensions.
 */
#ifndef CA_CANONICAL_CBOR_H
#define CA_CANONICAL_CBOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Maximum recursion depth accepted by cbor_decode. A generous cap that guards
 * against attacker-crafted "chain of nested arrays/tags" stack-overflow DoS. */
#define CBOR_MAX_DECODE_DEPTH 128

/* The nine value kinds of the canonical profile (CBOR major types 0..7). */
typedef enum {
    CBOR_UNSIGNED, /* major 0: the non-negative integer n */
    CBOR_NEGATIVE, /* major 1: represents -1 - n */
    CBOR_BYTES,    /* major 2: byte string */
    CBOR_TEXT,     /* major 3: UTF-8 text string */
    CBOR_ARRAY,    /* major 4: ordered elements */
    CBOR_MAP,      /* major 5: key/value pairs, canonically ordered */
    CBOR_TAG,      /* major 6: tag number + inner value */
    CBOR_BOOL,     /* major 7, simple 20/21 */
    CBOR_NULL      /* major 7, simple 22 */
} CborType;

typedef struct CborValue CborValue;

/* One map entry: an owned key and an owned value. */
typedef struct {
    CborValue *key;
    CborValue *val;
} CborPair;

struct CborValue {
    CborType type;
    union {
        uint64_t u; /* Unsigned / Negative */
        int boolean;
        struct {
            uint8_t *data;
            size_t len;
        } bytes; /* Bytes / Text (Text is UTF-8, not NUL-terminated) */
        struct {
            CborValue **items;
            size_t len;
            size_t cap;
        } array;
        struct {
            CborPair *entries;
            size_t len;
            size_t cap;
        } map;
        struct {
            uint64_t number;
            CborValue *inner;
        } tag;
    } as;
};

/* Status / error codes. The non-OK codes mirror the Rust `CborError` variants;
 * CBOR_ERR_ALLOC has no Rust analogue (Rust allocates infallibly). */
typedef enum {
    CBOR_OK = 0,
    CBOR_ERR_UNEXPECTED_EOF,
    CBOR_ERR_TRAILING_BYTES,
    CBOR_ERR_RESERVED,
    CBOR_ERR_INDEFINITE,
    CBOR_ERR_NON_MINIMAL_INTEGER,
    CBOR_ERR_INVALID_UTF8,
    CBOR_ERR_NON_CANONICAL_MAP_ORDER,
    CBOR_ERR_UNSUPPORTED_SIMPLE,
    CBOR_ERR_FLOAT_NOT_SUPPORTED,
    CBOR_ERR_TOO_DEEP,
    CBOR_ERR_LENGTH_TOO_LARGE,
    CBOR_ERR_ALLOC
} CborStatus;

/* ── Constructors (return NULL on allocation failure) ─────────────────────*/

CborValue *cbor_unsigned(uint64_t n);
CborValue *cbor_negative(uint64_t n); /* encodes -1 - n */
CborValue *cbor_bool(int b);
CborValue *cbor_null(void);
CborValue *cbor_bytes(const uint8_t *data, size_t len); /* copies `data` */
CborValue *cbor_text(const char *utf8, size_t len);     /* copies `utf8` */
CborValue *cbor_array(void);                             /* empty array */
CborValue *cbor_map(void);                               /* empty map */

/* Wrap `inner` in a tag (takes ownership of `inner`; frees it and returns NULL
 * on allocation failure). */
CborValue *cbor_tag(uint64_t number, CborValue *inner);

/* Append to an array / map. Take ownership of the pushed value(s); on failure
 * the pushed value(s) are freed and a non-OK status is returned. */
CborStatus cbor_array_push(CborValue *array, CborValue *item);
CborStatus cbor_map_push(CborValue *map, CborValue *key, CborValue *val);

/* Recursively free a value tree. Safe on NULL. */
void cbor_free(CborValue *v);

/* Deep structural equality (mirrors Rust `PartialEq`). NULLs compare equal. */
int cbor_equal(const CborValue *a, const CborValue *b);

/* ── Encode / decode ──────────────────────────────────────────────────────*/

/* Encode `v` to canonical bytes. On CBOR_OK, *out is a malloc'd buffer of
 * *out_len bytes (NULL/0 only if the encoding is genuinely empty, which never
 * happens — every value is >= 1 byte) that the caller must free(). On error
 * *out is NULL. Only CBOR_ERR_ALLOC can be returned. */
CborStatus cbor_encode(const CborValue *v, uint8_t **out, size_t *out_len);

/* Decode exactly one canonical CBOR item from `bytes[0..len)`. On CBOR_OK *out
 * owns the value tree (free with cbor_free); on error *out is NULL and the
 * returned code identifies the violation. */
CborStatus cbor_decode(const uint8_t *bytes, size_t len, CborValue **out);

#ifdef __cplusplus
}
#endif

#endif /* CA_CANONICAL_CBOR_H */
