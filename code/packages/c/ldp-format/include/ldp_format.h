/*
 * ldp_format.h — a versioned binary codec for `.ldp` artefacts, pure ISO C17.
 * ==========================================================================
 *
 * A faithful port of the Rust `ldp-format` crate: read/write of the LANG22
 * "profile artefact" binary format — a compact, deterministic on-disk record of
 * a JIT/AOT profiler's observations.
 *
 * Format (version 1, little-endian): a 32-byte header (magic "LDP\0", version,
 * 16-byte NUL-padded ASCII language, flags, record_count, reserved), a
 * deduplicated string table (every record string is a u32 index into it), then
 * module → function → instruction records.
 *
 * DETERMINISM. `ldp_write` produces byte-identical output for equal input; the
 * string table is built in first-occurrence order.
 *
 * SAFETY. `ldp_read` treats its input as UNTRUSTED — every field is bounds
 * checked (returning LDP_ERR_UNEXPECTED_EOF / LDP_ERR_BAD_STRING_INDEX where the
 * Rust code relies on typed errors), and nested arrays grow incrementally as
 * elements are read, so a corrupt record/string count can never drive a huge
 * speculative allocation. All growable buffers guard `size_t` overflow.
 *
 * OWNERSHIP. `ldp_read` returns a fully-owned LdpFile (free with ldp_file_free).
 * `ldp_write` reads a caller-owned LdpFile without taking ownership. Strings are
 * NUL-terminated (the format's names/opcodes are text; embedded NULs are not
 * modelled).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef LDP_FORMAT_H
#define LDP_FORMAT_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

typedef enum { LDP_FULLY_TYPED = 0, LDP_PARTIALLY_TYPED = 1, LDP_UNTYPED = 2 } LdpTypeStatus;
typedef enum { LDP_INTERP = 0, LDP_JITTED = 1, LDP_DEOPTED = 2 } LdpPromotionState;
typedef enum { LDP_UNINIT = 0, LDP_MONO = 1, LDP_POLY = 2, LDP_MEGA = 3 } LdpObservedKind;

typedef enum {
    LDP_OK = 0,
    LDP_ERR_BAD_MAGIC,
    LDP_ERR_UNSUPPORTED_MAJOR,
    LDP_ERR_UNEXPECTED_EOF,
    LDP_ERR_BAD_STRING_INDEX,
    LDP_ERR_BAD_OBSERVED_KIND,
    LDP_ERR_BAD_TYPE_STATUS,
    LDP_ERR_BAD_PROMOTION_STATE,
    LDP_ERR_LANGUAGE_TOO_LONG,
    LDP_ERR_LANGUAGE_NOT_ASCII,
    LDP_ERR_STRING_TABLE_OVERFLOW,
    LDP_ERR_STRING_TOO_LONG,
    LDP_ERR_OUT_OF_MEMORY
} LdpStatus;

/* ── Data model ─────────────────────────────────────────────────────────────*/
typedef struct {
    char *type_name;
    uint32_t count;
} LdpTypeSeen;

typedef struct {
    uint32_t instr_index;
    char *opcode;
    uint32_t observation_count;
    LdpObservedKind observed_kind;
    uint32_t observation_count_at_promotion;
    uint64_t time_to_first_observation_ns;
    uint64_t time_to_promotion_ns;
    LdpTypeSeen *types_seen;
    size_t types_seen_len;
} LdpInstruction;

typedef struct {
    char *name;
    char **params;
    size_t params_len;
    uint64_t call_count;
    uint64_t total_self_time_ns;
    LdpTypeStatus type_status;
    LdpPromotionState promotion_state;
    LdpInstruction *instructions;
    size_t instructions_len;
} LdpFunction;

typedef struct {
    char *name;
    LdpFunction *functions;
    size_t functions_len;
} LdpModule;

typedef struct {
    uint16_t version_major;
    uint16_t version_minor;
    char *language;
    uint32_t flags;
    LdpModule *modules;
    size_t modules_len;
} LdpFile;

/* ── API ────────────────────────────────────────────────────────────────────*/

/* Serialise `file` into a malloc'd buffer stored in *out (free with free) of
 * length *out_len. Does not take ownership of `file`. */
LdpStatus ldp_write(const LdpFile *file, uint8_t **out, size_t *out_len);

/* Parse a buffer into a fully-owned LdpFile stored in *out (free with
 * ldp_file_free). Never reads out of bounds on malformed input. */
LdpStatus ldp_read(const uint8_t *data, size_t len, LdpFile **out);

/* Free an owned LdpFile (as returned by ldp_read). NULL-safe. Do NOT call on a
 * caller-constructed file whose strings are literals / not heap-owned. */
void ldp_file_free(LdpFile *file);

/* Deep structural equality (for round-trip verification). */
int ldp_file_equal(const LdpFile *a, const LdpFile *b);

#ifdef __cplusplus
}
#endif

#endif /* LDP_FORMAT_H */
