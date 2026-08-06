/*
 * zip.h — the ZIP archive format (PKZIP, Phil Katz 1989), in pure ISO C17. A
 * faithful port of the Rust `zip` crate (CMP09).
 * ===========================================================================
 *
 * ZIP bundles one or more files into a single archive, compressing each entry
 * INDEPENDENTLY with DEFLATE (method 8, RFC 1951) or storing it verbatim
 * (method 0). The same format underlies Java JARs, Office Open XML
 * (.docx/.xlsx/.pptx), Android APKs, Python wheels, and many more.
 *
 *   Series:
 *     CMP02 (LZSS,    1982) — LZ77 + flag bits.
 *     CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
 *     CMP09 (ZIP,     1989) — DEFLATE container; universal archive. (this file)
 *
 * # Architecture
 *
 *     +-------------------------------------------------------+
 *     |  [Local File Header + File Data]  <- entry 1           |
 *     |  [Local File Header + File Data]  <- entry 2           |
 *     |  ...                                                   |
 *     |  ========== Central Directory ==========               |
 *     |  [Central Dir Header]  <- entry 1 (carries local offset)|
 *     |  [Central Dir Header]  <- entry 2                       |
 *     |  [End of Central Directory Record (EOCD)]                |
 *     +-------------------------------------------------------+
 *
 * The dual-header design supports two workflows:
 *   - Sequential write: append Local Headers one-by-one, write the Central
 *     Directory + EOCD at the end (`ZipWriter`).
 *   - Random-access read: seek to the EOCD at the end of the file, read the
 *     Central Directory, then jump straight to any entry's data (`ZipReader`).
 *
 * The Central Directory — not the Local Header — is the AUTHORITATIVE source
 * for each entry's size and compression method; the Local Header is consulted
 * only for its variable-length name/extra fields, needed to locate the entry's
 * data bytes.
 *
 * # Wire Format (all multi-byte integers little-endian)
 *
 * Local File Header (30 + n + e bytes):
 *
 *     [0x04034B50]              signature
 *     [version_needed  u16]     20 = DEFLATE, 10 = Stored
 *     [flags           u16]     bit 0 = encrypted, bit 11 = UTF-8 filename
 *     [method          u16]     0 = Stored, 8 = DEFLATE
 *     [mod_time        u16]     MS-DOS packed time
 *     [mod_date        u16]     MS-DOS packed date
 *     [crc32           u32]
 *     [compressed_size u32]
 *     [uncompressed_size u32]
 *     [name_len        u16]  (n)
 *     [extra_len       u16]  (e)
 *     [name bytes...]
 *     [extra bytes...]
 *     [file data...]
 *
 * Central Directory Header (46 + n + e + c bytes), one per entry, all written
 * after the last Local Header:
 *
 *     [0x02014B50]              signature
 *     [version_made_by u16]
 *     [version_needed  u16]
 *     [flags           u16]
 *     [method          u16]
 *     [mod_time        u16]
 *     [mod_date        u16]
 *     [crc32           u32]
 *     [compressed_size u32]
 *     [uncompressed_size u32]
 *     [name_len        u16]  (n)
 *     [extra_len       u16]  (e)
 *     [comment_len     u16]  (c) -- always 0 here
 *     [disk_start      u16]
 *     [internal_attrs  u16]
 *     [external_attrs  u32]     Unix: (mode << 16)
 *     [local_offset    u32]     byte offset of the Local Header
 *     [name bytes...]
 *     [extra bytes...]
 *     [comment bytes...]
 *
 * End of Central Directory Record (fixed 22 bytes):
 *
 *     [0x06054B50]              signature
 *     [disk_num          u16]
 *     [cd_disk           u16]
 *     [entries_this_disk u16]
 *     [entries_total     u16]
 *     [cd_size           u32]
 *     [cd_offset         u32]
 *     [comment_len       u16]   0 in this implementation
 *
 * # DEFLATE Inside ZIP
 *
 * ZIP method 8 carries RAW RFC 1951 DEFLATE — no zlib wrapper (no CMF/FLG
 * header, no Adler-32 checksum). This package depends directly on the sibling
 * `c/deflate` package (CMP05, RFC 1951): `deflate_compress` for writing (fixed
 * or dynamic Huffman, whichever is smaller) and `deflate_decompress` for
 * reading (all three RFC 1951 block types — stored, fixed, and dynamic
 * Huffman — so archives from `zip`(1), Python's `zipfile`, Java, and Microsoft
 * Office all decode correctly, not just this package's own output).
 *
 * NOTE ON REPO-WIDE PRECEDENT: `code/specs/CMP09-zip.md` documents that most
 * language ports of `zip` do NOT depend on that language's `deflate` package,
 * because several of those `deflate` packages use a private, non-standard wire
 * format for their own internal round-tripping (see `dart/deflate`, and
 * `lessons.md` Lesson 98). `c/deflate` is a documented exception: it was built
 * specifically to be a genuine RFC 1951 codec (verified by decoding a real
 * `zlib`-produced dynamic-Huffman stream — see its own header comment), so
 * `c/zip` depends on it directly rather than re-implementing DEFLATE framing
 * here, matching the Haskell port's documented divergence for the same reason
 * in spirit (though Haskell reimplements DEFLATE itself; C does not need to).
 *
 * # CRC-32
 *
 * ZIP uses CRC-32 (polynomial 0xEDB88320, reflected) to detect ACCIDENTAL
 * corruption of the decompressed content — it is NOT a cryptographic hash and
 * must not be relied on to detect tampering.
 *
 * # Security
 *
 * `zip_reader_*` treats its input as untrusted bytes:
 *   - The EOCD search is bounded to the last (22 + 65535) bytes of the input
 *     (the maximum possible EOCD comment length), never an unbounded scan.
 *   - Every multi-byte field read is bounds-checked before use.
 *   - Central Directory offset/size arithmetic is performed in a 64-bit
 *     intermediate before narrowing to `size_t`, so a maliciously large
 *     32-bit offset or size cannot wrap around and pass a bounds check on a
 *     32-bit `size_t` platform.
 *   - The number of Central Directory entries actually parsed is capped at
 *     `ZIP_MAX_ENTRIES` (65535, the largest count the non-ZIP64 format can
 *     declare) as a defensive iteration bound.
 *   - Per-entry decompressed size is bounded by `c/deflate`'s own
 *     `DEFLATE_MAX_OUTPUT` (256 MiB) cap. In ADDITION, `ZipReader` tracks the
 *     AGGREGATE number of decompressed bytes handed back across every
 *     `zip_reader_read` / `zip_reader_read_by_name` call made through it (so a
 *     "many small entries, each individually fine, that sum to gigabytes"
 *     bomb is rejected too — not just a single oversized entry) and fails
 *     with `ZIP_ERR_TOO_LARGE` once the configured budget
 *     (`ZIP_DEFAULT_MAX_TOTAL_UNCOMPRESSED`, 256 MiB by default; see
 *     `zip_reader_new_with_budget`) would be exceeded.
 *   - Entries whose General Purpose Bit Flag has bit 0 (encrypted) set are
 *     rejected with `ZIP_ERR_ENCRYPTED` rather than silently mis-decoded.
 *   - Compression methods other than 0 (Stored) and 8 (DEFLATE) are rejected
 *     with `ZIP_ERR_UNSUPPORTED_METHOD`.
 *
 * ZIP SLIP / PATH TRAVERSAL: this library is purely in-memory — `ZipEntry`
 * names are returned as plain byte strings and NOTHING in this package ever
 * writes to the filesystem, so path traversal cannot happen INSIDE this
 * package. Any caller that builds a "write entry to disk" helper on top of
 * `zip_reader_read` MUST independently reject or sanitise names containing
 * `..` path components or starting with `/` (or a drive letter on Windows)
 * before joining them to a destination directory — this package does not do
 * that for you, matching the spec's guidance that only a disk-writing wrapper
 * is exposed to zip-slip in the first place.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Standard library only.
 *
 * Dependencies: `c/deflate` (CMP05) for RFC 1951 DEFLATE compress/decompress
 * (which itself depends on `c/lzss`, CMP02, for LZ77 match-finding — `c/zip`
 * does not call `c/lzss` directly).
 */
#ifndef ZIP_H
#define ZIP_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint16_t, uint32_t */

typedef enum {
    ZIP_OK = 0,
    ZIP_ERR_ALLOC,               /* out of memory */
    ZIP_ERR_MALFORMED,           /* corrupt/truncated archive structure */
    ZIP_ERR_CRC_MISMATCH,        /* decompressed bytes don't match stored CRC-32 */
    ZIP_ERR_UNSUPPORTED_METHOD,  /* compression method isn't 0 (Stored) or 8 (DEFLATE) */
    ZIP_ERR_ENCRYPTED,           /* General Purpose Bit Flag bit 0 is set */
    ZIP_ERR_NOT_FOUND,           /* zip_reader_read_by_name: no entry with that name */
    ZIP_ERR_TOO_LARGE            /* aggregate decompressed-bytes budget exceeded */
} ZipStatus;

/* Largest number of entries the classic (non-ZIP64) Central Directory can
 * declare; also enforced as a hard cap on how many entries this package will
 * actually parse out of a Central Directory, regardless of what any header
 * field claims (see the Security section above). */
#define ZIP_MAX_ENTRIES 65535u

/* Default aggregate decompressed-bytes budget per ZipReader (see
 * zip_reader_new / zip_reader_new_with_budget and the Security section
 * above). Matches the spec's suggested default of 256 MB. */
#define ZIP_DEFAULT_MAX_TOTAL_UNCOMPRESSED ((size_t)256 * 1024 * 1024)

/* ---- CRC-32 ------------------------------------------------------------ */

/* zip_crc32 — CRC-32 (polynomial 0xEDB88320) over `data[0..len)`, continuing
 * from `initial` (pass 0 for a fresh checksum, or a prior return value to
 * checksum incrementally across chunks). NOT a cryptographic hash. */
uint32_t zip_crc32(const unsigned char *data, size_t len, uint32_t initial);

/* ---- MS-DOS date/time --------------------------------------------------- */

/* zip_dos_datetime — pack (year, month, day, hour, minute, second) into the
 * 32-bit MS-DOS date+time ZIP stores in its headers: `(date << 16) | time`.
 * `year` before 1980 clamps to 1980 (DOS has no earlier representable year). */
uint32_t zip_dos_datetime(unsigned year, unsigned month, unsigned day,
                          unsigned hour, unsigned minute, unsigned second);

/* Fixed timestamp (1980-01-01 00:00:00) used by zip_writer_add_* when no real
 * modification time is supplied — this library operates on in-memory byte
 * buffers, which have no filesystem mtime of their own. */
#define ZIP_DOS_EPOCH 0x00210000ul

/* ---- Writer -------------------------------------------------------------
 *
 * Usage:
 *
 *     ZipWriter *w;
 *     zip_writer_new(&w);
 *     zip_writer_add_file(w, "hello.txt", data, len, 1);  // 1 = try DEFLATE
 *     zip_writer_add_directory(w, "mydir/");
 *
 *     unsigned char *archive; size_t archive_len;
 *     zip_writer_finish(w, &archive, &archive_len);  // malloc'd, free() it
 *     zip_writer_free(w);
 */

typedef struct ZipWriter ZipWriter; /* opaque */

/* zip_writer_new — allocate an empty writer. On ZIP_OK, *out is heap-owned;
 * pair with zip_writer_free. */
ZipStatus zip_writer_new(ZipWriter **out);

/* zip_writer_add_file — append a file entry. `data`/`len` are copied
 * immediately (the writer does not borrow them past this call). If `compress`
 * is nonzero, DEFLATE is attempted and used only when it is STRICTLY smaller
 * than the uncompressed original; otherwise (or when `compress` is 0) the
 * entry is Stored (method 0). */
ZipStatus zip_writer_add_file(ZipWriter *w, const char *name,
                              const unsigned char *data, size_t len,
                              int compress);

/* zip_writer_add_directory — append a directory entry. `name` should end in
 * '/' (not enforced — callers that omit it get a directory entry whose name
 * doesn't look like one to most extractors' UIs, but it round-trips fine
 * through this library either way). Directory entries are always Stored,
 * zero-length. */
ZipStatus zip_writer_add_directory(ZipWriter *w, const char *name);

/* zip_writer_finish — assemble the Central Directory + EOCD and return the
 * complete archive. On ZIP_OK, *out is a malloc'd buffer of *out_len bytes
 * (free with free()). The writer is left in a finished state; only
 * zip_writer_free may be called on it afterward. */
ZipStatus zip_writer_finish(ZipWriter *w, unsigned char **out, size_t *out_len);

/* zip_writer_free — release the writer and everything it owns. NULL-safe.
 * Safe to call whether or not zip_writer_finish was called first. */
void zip_writer_free(ZipWriter *w);

/* ---- Reader --------------------------------------------------------------
 *
 * Usage:
 *
 *     ZipReader *r;
 *     zip_reader_new(archive, archive_len, &r);   // BORROWS archive
 *     for (size_t i = 0; i < zip_reader_entry_count(r); i++) {
 *         const ZipEntry *e = zip_reader_entry(r, i);
 *         unsigned char *data; size_t data_len;
 *         zip_reader_read(r, e, &data, &data_len); // verifies CRC-32
 *         free(data);
 *     }
 *     zip_reader_free(r);
 *
 * IMPORTANT: ZipReader BORROWS the `data` buffer passed to zip_reader_new; it
 * must remain valid and unmodified for the reader's entire lifetime (this
 * mirrors the Rust port's `ZipReader<'a>` borrow — copying multi-megabyte
 * archives on open would be wasteful, and callers already have the bytes).
 */

typedef struct {
    char *name;              /* malloc'd, NUL-terminated UTF-8; owned by the entry */
    size_t name_len;          /* byte length of name, excluding the NUL */
    uint32_t size;             /* uncompressed size */
    uint32_t compressed_size;
    uint16_t method;           /* 0 = Stored, 8 = DEFLATE */
    uint32_t crc32;
    int is_directory;         /* 1 if name ends with '/' */
    uint32_t local_offset;     /* internal: byte offset of the Local Header */
} ZipEntry;

typedef struct ZipReader ZipReader; /* opaque */

/* zip_reader_new — parse the Central Directory of an in-memory archive
 * (`data`, `len` bytes; BORROWED, see above), using the default aggregate
 * decompressed-bytes budget (ZIP_DEFAULT_MAX_TOTAL_UNCOMPRESSED). Returns
 * ZIP_ERR_MALFORMED if no valid EOCD is found or the structure is corrupt,
 * ZIP_ERR_ALLOC on allocation failure. On ZIP_OK, *out is heap-owned; pair
 * with zip_reader_free. */
ZipStatus zip_reader_new(const unsigned char *data, size_t len, ZipReader **out);

/* zip_reader_new_with_budget — like zip_reader_new, but with an explicit
 * aggregate decompressed-bytes budget (see the Security section in this
 * header) instead of the default. */
ZipStatus zip_reader_new_with_budget(const unsigned char *data, size_t len,
                                     size_t max_total_uncompressed,
                                     ZipReader **out);

/* zip_reader_entry_count / zip_reader_entry — the parsed entry list, in
 * Central Directory order. The returned ZipEntry* is borrowed; valid until
 * zip_reader_free. `index` must be < zip_reader_entry_count(r). */
size_t zip_reader_entry_count(const ZipReader *r);
const ZipEntry *zip_reader_entry(const ZipReader *r, size_t index);

/* zip_reader_read — decompress `entry`'s data and verify its CRC-32. On
 * ZIP_OK, *out is a malloc'd buffer of *out_len bytes (free with free(); NULL
 * when *out_len == 0, e.g. for a directory entry or an empty file). Consumes
 * from the reader's aggregate decompressed-bytes budget; returns
 * ZIP_ERR_TOO_LARGE if this read would exceed it. */
ZipStatus zip_reader_read(ZipReader *r, const ZipEntry *entry,
                          unsigned char **out, size_t *out_len);

/* zip_reader_read_by_name — find the first entry named `name` and read it
 * (see zip_reader_read). ZIP_ERR_NOT_FOUND if no entry has that name. */
ZipStatus zip_reader_read_by_name(ZipReader *r, const char *name,
                                  unsigned char **out, size_t *out_len);

/* zip_reader_free — release the reader and every entry it owns. Does NOT free
 * the `data` buffer passed to zip_reader_new (borrowed, not owned).
 * NULL-safe. */
void zip_reader_free(ZipReader *r);

/* ---- One-shot convenience API -------------------------------------------
 *
 * zip_bytes([(name,data)...]) -> archive bytes
 * zip_unzip(archive bytes)    -> [(name,data)...]   (directories skipped)
 */

typedef struct {
    char *name;            /* malloc'd, NUL-terminated */
    size_t name_len;
    unsigned char *data;   /* malloc'd; NULL if len == 0 */
    size_t len;
} ZipFile;

/* zip_bytes — compress `count` (name, data) pairs into a complete archive.
 * Each entry is compressed with DEFLATE if it helps, else Stored (same policy
 * as zip_writer_add_file(..., compress=1)). On ZIP_OK, *out is malloc'd (free
 * with free()). */
ZipStatus zip_bytes(const ZipFile *files, size_t count, unsigned char **out,
                    size_t *out_len);

/* zip_unzip — decompress every non-directory entry of an archive. On ZIP_OK,
 * *out_files is a malloc'd array of *out_count ZipFile (each name/data
 * malloc'd; free with zip_files_free). Directory entries are skipped. Uses
 * the default aggregate decompressed-bytes budget
 * (ZIP_DEFAULT_MAX_TOTAL_UNCOMPRESSED). */
ZipStatus zip_unzip(const unsigned char *data, size_t len, ZipFile **out_files,
                    size_t *out_count);

/* zip_files_free — release an array returned by zip_unzip. NULL-safe. */
void zip_files_free(ZipFile *files, size_t count);

#endif /* ZIP_H */
