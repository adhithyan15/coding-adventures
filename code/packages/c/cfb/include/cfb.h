/*
 * cfb.h — OLE2 / Compound File Binary Format reader, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `cfb` crate: a from-scratch reader for the
 * Microsoft Compound File Binary Format ([MS-CFB]) — the container inside legacy
 * `.xls`, `.doc`, and `.ppt` files. It is the read counterpart to the ported
 * `cfb-writer`.
 *
 * ## Mental model
 *
 * A CFB file is a FAT filesystem crammed into one file: chopped into fixed-size
 * sectors (512 or 4096 bytes), with a File Allocation Table (an array of
 * "next sector" pointers) chaining multi-sector streams. A directory (itself a
 * FAT-stored stream) names the objects — a *stream* is a file, a *storage* a
 * folder. Tiny streams live packed in a mini-stream chained by a parallel
 * mini-FAT.
 *
 * ## Hostile input
 *
 * CFB files arrive as email attachments, so every sector-chain walk is
 * cycle-guarded (bounded by the number of FAT slots — a valid chain never
 * revisits a sector), every sector offset is bounds-checked with overflow-safe
 * arithmetic, and assembled output is capped at 256 MiB. Malformed input yields
 * an error, never an out-of-bounds access or a hang.
 *
 * Divergence from the Rust (documented): stream/entry names are decoded
 * UTF-16 LE → UTF-8 into fixed 128-byte buffers; `CfbError` drops the sector
 * size the Rust `UnsupportedSectorSize` variant carries. Case-insensitive name
 * matching is ASCII-only (CFB names are ASCII in practice).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef CFB_H
#define CFB_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

#define CFB_NAME_CAP 128

/* Everything that can go wrong reading a (possibly hostile) CFB file. */
typedef enum {
    CFB_OK = 0,
    CFB_BAD_SIGNATURE,
    CFB_TRUNCATED,
    CFB_UNSUPPORTED_SECTOR_SIZE,
    CFB_BAD_SECTOR_CHAIN,
    CFB_CYCLE_DETECTED,
    CFB_OUTPUT_TOO_LARGE,
    CFB_BAD_DIRECTORY,
    CFB_NOT_A_STREAM
} CfbError;

const char *cfb_error_str(CfbError e);

/* What kind of object a directory entry is. */
typedef enum {
    CFB_ENTRY_STREAM,
    CFB_ENTRY_STORAGE,
    CFB_ENTRY_ROOT_STORAGE
} CfbEntryKind;

/* One enumerated object: decoded name, byte size, kind, and directory id. */
typedef struct {
    char name[CFB_NAME_CAP]; /* UTF-8 (leading control chars preserved) */
    uint64_t size;
    CfbEntryKind kind;
    uint32_t id;
} CfbEntry;

/* Opaque parsed compound file (owns a copy of the input). */
typedef struct CompoundFile CompoundFile;

/* Parse a CFB file. On success returns CFB_OK and sets *out to a new
 * CompoundFile (free with cfb_free); otherwise returns the error, *out NULL. */
CfbError cfb_open(const uint8_t *bytes, size_t len, CompoundFile **out);
void cfb_free(CompoundFile *cf);

/* Sector size (512 or 4096). */
size_t cfb_sector_size(const CompoundFile *cf);

/* Enumerated non-root objects plus the root (streams + storages + root). */
size_t cfb_entry_count(const CompoundFile *cf);
const CfbEntry *cfb_entry(const CompoundFile *cf, size_t i); /* NULL if oob */

/* Read a top-level stream by name (ASCII case-insensitive). Returns 1 on
 * success and allocates *out_data (caller frees) / sets *out_len; returns 0 if
 * no such stream exists or it could not be read. */
int cfb_read_stream(const CompoundFile *cf, const char *name,
                    uint8_t **out_data, size_t *out_len);

/* Read a stream precisely by directory id. Returns CFB_OK and allocates
 * *out_data (caller frees; may be NULL when *out_len == 0) / sets *out_len, or
 * an error. */
CfbError cfb_read_stream_by_id(const CompoundFile *cf, uint32_t id,
                               uint8_t **out_data, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* CFB_H */
