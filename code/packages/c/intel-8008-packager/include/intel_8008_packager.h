/*
 * intel_8008_packager.h — Intel HEX ROM image encoder/decoder, pure ISO C17.
 * =========================================================================
 *
 * A faithful port of the Rust `intel-8008-packager` crate. Converts raw binary
 * machine code into the Intel HEX format used by EPROM programmers, and parses
 * Intel HEX back to binary for round-trip verification.
 *
 * ## Intel HEX record: `:LLAAAATTDD...CC`
 *
 *   :     start code
 *   LL    byte count (hex, 0-255 data bytes)
 *   AAAA  16-bit load address (big-endian hex)
 *   TT    record type: 00 = data, 01 = end-of-file
 *   DD    data bytes (LL x 2 hex chars)
 *   CC    checksum: two's complement of the byte-sum of all fields, so summing
 *         every byte of the record (checksum included) yields 0 mod 256.
 *
 * The decoder handles only type-00 (data) and type-01 (EOF) records — the
 * subset the encoder produces — and rejects malformed, mis-checksummed,
 * overlapping, over-long, or unterminated input.
 *
 * ## Divergences from Rust (documented)
 *
 *   - Rust `Result<_, PackagerError(String)>` -> a `PakStatus` code; the dynamic
 *     Rust error strings become one representative static string per code (see
 *     `pak_error_message`), each containing the same keyword the Rust message
 *     does.
 *   - Rust `String` / `Vec<u8>` results -> malloc'd buffers the caller frees.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no <math.h>, no compiler extensions.
 */
#ifndef CA_INTEL_8008_PACKAGER_H
#define CA_INTEL_8008_PACKAGER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Maximum decoded image span: the Intel 8008's 14-bit address space (16 KB). */
#define PAK_MAX_IMAGE_SIZE 0x4000u

/* Status / error codes. Non-OK codes mirror the Rust `PackagerError` cases. */
typedef enum {
    PAK_OK = 0,
    PAK_ERR_EMPTY_BINARY,     /* encode: binary is empty */
    PAK_ERR_ORIGIN_TOO_LARGE, /* encode: origin > 0xFFFF */
    PAK_ERR_IMAGE_OVERFLOW,   /* encode: origin + len > 0x10000 */
    PAK_ERR_MISSING_COLON,    /* decode: a record lacks the leading ':' */
    PAK_ERR_INVALID_HEX,      /* decode: odd-length or non-hex record body */
    PAK_ERR_RECORD_TOO_SHORT, /* decode: fewer bytes than the record claims */
    PAK_ERR_BAD_CHECKSUM,     /* decode: checksum mismatch */
    PAK_ERR_UNSUPPORTED_TYPE, /* decode: record type >= 0x02 */
    PAK_ERR_IMAGE_TOO_LARGE,  /* decode: span exceeds PAK_MAX_IMAGE_SIZE */
    PAK_ERR_MISSING_EOF,      /* decode: no type-0x01 EOF record */
    PAK_ERR_OVERLAP,          /* decode: two records overlap / duplicate */
    PAK_ERR_LINE_TOO_LONG,    /* decode: a line exceeds the length cap */
    PAK_ERR_ALLOC             /* out of memory (no Rust analogue) */
} PakStatus;

/* A representative, static, human-readable message for a status code. Each
 * error string contains the same keyword the Rust `PackagerError` message uses
 * (e.g. "checksum", "overlap", "EOF"). Never NULL. */
const char *pak_error_message(PakStatus status);

/* Result of decoding: the lowest load address and the assembled payload. */
typedef struct {
    size_t origin;
    uint8_t *binary; /* malloc'd; NULL when binary_len == 0 */
    size_t binary_len;
} PakDecoded;

void pak_decoded_free(PakDecoded *d);

/* ── Encode / decode ──────────────────────────────────────────────────────*/

/* Encode `binary` (length `len`, must be non-empty) to an Intel HEX string
 * loaded at `origin`. On PAK_OK, *out is a malloc'd NUL-terminated string of
 * *out_len bytes (excluding the terminator) that the caller must free(). On
 * error *out is NULL. */
PakStatus pak_encode_hex(const uint8_t *binary, size_t len, size_t origin,
                         char **out, size_t *out_len);

/* Decode the NUL-terminated Intel HEX string `text` into *out. On PAK_OK *out
 * owns its buffer (free with pak_decoded_free); on error *out is zeroed. */
PakStatus pak_decode_hex(const char *text, PakDecoded *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_INTEL_8008_PACKAGER_H */
