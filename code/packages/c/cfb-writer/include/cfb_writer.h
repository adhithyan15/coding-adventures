/*
 * cfb_writer.h — Compound File Binary Format writer, pure ISO C17.
 * ==============================================================
 *
 * A faithful port of the Rust `cfb-writer` crate: a from-scratch, zero-dependency
 * writer for the OLE2 / Compound File Binary Format ([MS-CFB]) — the container
 * inside legacy .xls / .doc / .ppt files. You hand it named streams; it produces
 * a byte buffer that a conforming CFB reader (and real Office tooling) accepts.
 *
 * ── Mental model ───────────────────────────────────────────────────────────
 * A CFB file is a FAT filesystem crammed into one file: a fixed 512-byte header,
 * then equal-sized 512-byte sectors. A File Allocation Table (FAT) holds one
 * "next sector" u32 per sector, so a multi-sector stream is a linked list ending
 * in ENDOFCHAIN. A directory (itself a FAT-stored stream of 128-byte entries)
 * names the objects. Streams smaller than the 4096-byte cutoff are packed into a
 * mini-stream of 64-byte mini-sectors chained by a parallel mini-FAT.
 *
 * Output is version 3 (512-byte sectors) and fully deterministic (CLSIDs and
 * timestamps zeroed), so the same input always yields identical bytes.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef CFB_WRITER_H
#define CFB_WRITER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

#ifdef __cplusplus
extern "C" {
#endif

/* An accumulating set of named streams, in insertion order. */
typedef struct CfbWriter CfbWriter;

/* Create an empty writer (NULL on allocation failure). Pair with cfb_writer_free
 * or consume it with cfb_writer_finish. */
CfbWriter *cfb_writer_new(void);
/* Destroy a writer that was NOT consumed by cfb_writer_finish. */
void cfb_writer_free(CfbWriter *w);

/* Add a named stream (the name is UTF-8; it is transcoded to the on-disk
 * UTF-16LE and truncated to 31 UTF-16 code units). Copies `data`. Returns 1 on
 * success, 0 on allocation failure. */
int cfb_writer_add_stream(CfbWriter *w, const char *name, const uint8_t *data,
                          size_t data_len);

/* Serialise everything into a finished CFB byte buffer. Returns a malloc'd
 * buffer (caller frees) and writes its length to `*out_len`; returns NULL on
 * allocation failure. The writer is DESTROYED by this call (do not use or free
 * `w` afterwards). */
uint8_t *cfb_writer_finish(CfbWriter *w, size_t *out_len);

/* One-shot convenience: build a CFB from `n` (name, data, data_len) triples.
 * `names[i]` / `data[i]` / `data_len[i]` describe stream i. Returns a malloc'd
 * buffer (caller frees) + `*out_len`, or NULL on failure. */
uint8_t *cfb_write(const char *const *names, const uint8_t *const *data,
                   const size_t *data_len, size_t n, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* CFB_WRITER_H */
