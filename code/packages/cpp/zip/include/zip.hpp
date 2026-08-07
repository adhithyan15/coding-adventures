// zip.hpp — the ZIP archive format (PKZIP, Phil Katz / Gary Conway, 1989), in
// pure ISO C++17, header-only, in namespace ca::zip. A faithful port of the
// Rust `zip` crate (CMP09).
// ===========================================================================
//
// ZIP bundles one or more files into a single archive, compressing each
// entry independently with DEFLATE (method 8, CMP05) or storing it verbatim
// (method 0). The same format underlies Java JARs, Office Open XML
// (`.docx`/`.xlsx`/`.pptx`), Android APKs, Python wheels, `.epub`, and many
// more — this package is a straight CONTAINER around the sibling `deflate`
// package (CMP05), which itself sits on `lzss` (CMP02): CMP09 does no
// entropy coding or match-finding of its own, it only frames DEFLATE streams
// with headers, directories, and a CRC-32 integrity check.
//
// ARCHITECTURE
//
//   ┌─────────────────────────────────────────────────────┐
//   │  [Local File Header + File Data]  ← entry 1          │
//   │  [Local File Header + File Data]  ← entry 2          │
//   │  ...                                                 │
//   │  ══════════ Central Directory ══════════             │
//   │  [Central Dir Header]  ← entry 1 (has local offset)  │
//   │  [Central Dir Header]  ← entry 2                     │
//   │  [End of Central Directory Record]                   │
//   └─────────────────────────────────────────────────────┘
//
// The dual-header design enables two workflows:
//   - Sequential write: append Local Headers one-by-one, write the Central
//     Directory (CD) and End-of-Central-Directory record (EOCD) at the end.
//   - Random-access read: seek to the EOCD at the end, read the CD, jump
//     straight to any entry's Local Header without scanning the others.
//
// WIRE FORMAT (all integers little-endian; see code/specs/CMP09-zip.md for
// the byte-exact tables this mirrors):
//
//   Local File Header (30 + n + e bytes, n=name length, e=extra length):
//     [0x04034B50]           signature
//     [version_needed  u16]  20=DEFLATE, 10=Stored
//     [flags           u16]  bit 11 = UTF-8 filename
//     [method          u16]  0=Stored, 8=DEFLATE
//     [mod_time        u16]  MS-DOS packed time
//     [mod_date        u16]  MS-DOS packed date
//     [crc32           u32]
//     [compressed_size u32]
//     [uncompressed_size u32]
//     [name_len        u16]
//     [extra_len       u16]
//     [name bytes...] [extra bytes...] [file data...]
//
//   Central Directory Header (46 + n + e + c bytes, one per entry):
//     [0x02014B50]  signature, then version_made_by/needed, flags, method,
//     mod_time/date, crc32, compressed/uncompressed size, name/extra/comment
//     lengths, disk_start, int_attrs, ext_attrs (Unix: mode << 16),
//     local_offset, name, extra, comment.
//
//   End of Central Directory Record (fixed 22 bytes):
//     [0x06054B50]  signature, disk_num, cd_disk, entries_this_disk,
//     entries_total, cd_size, cd_offset, comment_len.
//
// DEFLATE INSIDE ZIP: method 8 stores raw RFC 1951 DEFLATE (no zlib
// wrapper — no CMF/FLG header, no Adler-32). `ZipWriter` calls the sibling
// `ca::deflate::compress`, which picks fixed vs. dynamic Huffman per block by
// exact bit count and is real, standards-conformant DEFLATE — not a private
// subset. `ZipReader` calls `ca::deflate::inflate`, which decodes ALL THREE
// RFC 1951 block types (stored, fixed, dynamic Huffman), so it opens
// archives this package never wrote — `zip`(1), Python's `zipfile`, and
// Microsoft Office all emit dynamic Huffman routinely. (NOTE: unlike the
// `dart/deflate` sibling documented in this repo's lessons.md as a private,
// non-standard wire format that its `dart/zip` had to work around by
// self-containing DEFLATE, `cpp/deflate` was independently verified against
// real `zlib` dynamic-Huffman output — see its own module doc — so this
// package can and does depend on it directly.)
//
// ERROR HANDLING (this repo's convention for decoders of untrusted bytes,
// mirroring `deflate`'s `DeflateException` / `canonical-cbor`'s
// `CborException`): `ZipReader`'s constructor and its `read`/`read_by_name`
// throw `ZipException` (carrying a `ZipError`) on any malformed input.
// `ZipWriter` throws only if asked to write an entry name/size, a
// cumulative archive offset/size, or an entry count, that would silently
// truncate under this format's (non-ZIP64) 16-bit name / 32-bit size and
// offset / 16-bit entry-count fields (`NameTooLong` / `DataTooLarge` /
// `ArchiveTooLarge` / `TooManyEntries`) — every other `ZipWriter` call never
// fails; `finish()` returns `std::vector<uint8_t>` directly.
//
// ROBUSTNESS. Every multi-byte field read from untrusted archive bytes is
// bounds-checked before use; offset arithmetic that combines two untrusted
// u32 fields (Central Directory offset + size, Local Header data start +
// compressed size) is done in `uint64_t` so it cannot silently wrap before
// the bounds check runs. `ZipReader::read` rejects encrypted entries
// (General-Purpose flag bit 0) and any compression method other than
// Stored/DEFLATE. Decompression-bomb protection is two-layered: each single
// DEFLATE entry is capped by `ca::deflate::MAX_INFLATE_OUTPUT` (256 MB,
// enforced inside `inflate` itself), and `unzip()` additionally enforces a
// configurable AGGREGATE budget across every entry it decompresses (default
// 256 MB) plus a hard cap of 65535 parsed entries — so a small archive with
// many entries, each individually under the per-entry cap, still cannot
// exhaust memory when read via the convenience API.
//
// ZIP-SLIP / PATH TRAVERSAL. This package is in-memory only: `ZipReader` and
// `unzip()` never write to disk, so a malicious entry name like
// `"../../etc/passwd"` or `"/etc/passwd"` is just a `std::string` key in the
// returned data — it cannot escape any directory because no directory is
// ever written to. A caller who builds a disk-extraction feature ON TOP of
// `ZipEntry::name` is responsible for sanitizing it first (reject leading
// `/`, reject any `..` path component) before joining it to a destination
// directory; this package deliberately provides no such disk-writing
// function, precisely to keep that responsibility from being silently
// assumed away.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only. No <span>
// (that is C++20); byte buffers are `std::vector<uint8_t>`, matching the
// sibling `lzss`/`deflate` packages' convention.
//
// Dependency: the sibling `deflate` package (CMP05) supplies
// `ca::deflate::compress` / `inflate` for method-8 entries — see `BUILD`.
//
// SERIES
//
//   CMP00 (LZ77,     1977) — Sliding-window backreferences.
//   CMP01 (LZ78,     1978) — Explicit dictionary (trie).
//   CMP02 (LZSS,     1982) — LZ77 + flag bits.
//   CMP03 (LZW,      1984) — LZ78 + pre-initialized alphabet; GIF.
//   CMP04 (Huffman,  1952) — Entropy coding.
//   CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//   CMP09 (ZIP,      1989) — DEFLATE container; universal archive. (this file)
#ifndef CA_ZIP_HPP
#define CA_ZIP_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <string>
#include <utility>
#include <vector>

#include "deflate.hpp"

namespace ca {
namespace zip {

using Bytes = std::vector<std::uint8_t>;

// ===========================================================================
// Errors
// ===========================================================================

enum class ZipError {
    NoEocdFound,
    EocdTruncated,
    CentralDirectoryOutOfBounds,
    CentralDirectoryEntryTruncated,
    CentralDirectoryEntryNameOutOfBounds,
    TooManyEntries,
    LocalHeaderOutOfBounds,
    LocalHeaderTruncated,
    EntryDataOutOfBounds,
    EncryptedEntryUnsupported,
    UnsupportedCompressionMethod,
    Crc32Mismatch,
    DeclaredSizeMismatch,
    UncompressedSizeBudgetExceeded,
    EntryNotFound,
    NameTooLong,
    DataTooLarge,
    ArchiveTooLarge,
};

// Thrown by `ZipReader`'s constructor and by `read`/`read_by_name`/`unzip`
// on any violation described in `ZipError` above — mirroring `deflate`'s
// `DeflateException` convention for decoders of untrusted bytes. `ZipWriter`
// additionally throws `NameTooLong`/`DataTooLarge`/`ArchiveTooLarge`/
// `TooManyEntries` if asked to write an entry, a cumulative archive, or an
// entry count, that would silently truncate under the (non-ZIP64) 16-bit
// name / 32-bit size and offset / 16-bit entry-count fields; every other
// `ZipWriter` operation never fails.
class ZipException : public std::exception {
public:
    ZipException(ZipError e, std::string msg) : err_(e), msg_(std::move(msg)) {}
    ZipError error() const noexcept { return err_; }
    const char* what() const noexcept override { return msg_.c_str(); }

private:
    ZipError err_;
    std::string msg_;
};

namespace detail {

// ===========================================================================
// CRC-32
// ===========================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (the bit-reflected form of the "CRC-32"
// generator polynomial 0x04C11DB7). It detects accidental corruption of the
// decompressed content — it is NOT a cryptographic hash; for tamper
// detection use AES-GCM or a signed manifest alongside the archive.
//
// Table construction, worked example for byte 1 (n=1):
//   c = 1
//   round 1: c&1==1 -> c = 0xEDB88320 ^ (1>>1) = 0xEDB88320 ^ 0 = 0xEDB88320
//   round 2: c&1==0 -> c = c>>1 = 0x76DC4190
//   ... (8 rounds total) ...
//   crc_table[1] ends up 0x77073096 — the standard CRC-32 table's second
//   entry, matching every other CRC-32 implementation (zlib, Python's
//   binascii, this repo's `deflate`-adjacent language ports).
inline std::array<std::uint32_t, 256> make_crc_table() {
    std::array<std::uint32_t, 256> table{};
    for (std::uint32_t i = 0; i < 256; ++i) {
        std::uint32_t c = i;
        for (int k = 0; k < 8; ++k) {
            if ((c & 1u) != 0u) {
                c = 0xEDB88320u ^ (c >> 1);
            } else {
                c = c >> 1;
            }
        }
        table[i] = c;
    }
    return table;
}

inline const std::array<std::uint32_t, 256>& crc_table() {
    static const std::array<std::uint32_t, 256> table = make_crc_table();
    return table;
}

// ===========================================================================
// Bounds-checked little-endian reads
// ===========================================================================
//
// Every multi-byte field pulled from untrusted archive bytes goes through
// one of these two helpers. Both return `false` (leaving `out` untouched) on
// out-of-bounds rather than reading past `data` — callers turn that into a
// `ZipException` with context-specific wording (e.g. "EOCD truncated" vs.
// "Central Directory entry truncated") rather than a single generic error,
// which is why this returns a bool instead of throwing directly.
//
// `offset` is `uint64_t`, NOT `size_t` — deliberately, even though the
// buffers involved never exceed real (`size_t`-representable) memory. Both
// helpers are frequently called as `read_u16(data, base + N, ...)` where
// `base` is itself derived from an attacker-controlled `uint32_t` archive
// field (e.g. a Central Directory `local_offset`) rather than from a
// position already bounded by `data.size()`. If `offset`'s type here were
// `size_t`, the caller's `base + N` addition would happen in `size_t`
// BEFORE ever reaching this function — on a platform where `size_t` is 32
// bits, `base` near `UINT32_MAX` would let `base + N` wrap to a small value
// at the call site, so this function would validate (and this function's
// own internal `uint64_t` bounds-check would pass for) the WRONG, already-
// wrapped offset instead of the real one, defeating the point of a careful
// internal check. Taking `uint64_t` here forces every call site that adds a
// constant to an untrusted base to do that addition in a width that cannot
// wrap for any `uint32_t`-derived `base` — see e.g. `ZipReader::read`, which
// widens `entry.local_offset` to `uint64_t` ONCE (`lh_off`) and adds every
// fixed field offset against that, never against a `size_t` copy of it. The
// final narrow to `size_t` for indexing (below) is safe precisely because
// it only happens after `offset + 2/4 <= data.size()` has been confirmed,
// so `offset` is by then provably a valid index into a real, `size_t`-sized
// buffer.
inline bool read_u16(const Bytes& data, std::uint64_t offset, std::uint16_t& out) {
    if (offset + 2 > data.size()) {
        return false;
    }
    std::size_t i = static_cast<std::size_t>(offset);
    out = static_cast<std::uint16_t>(static_cast<std::uint16_t>(data[i]) |
                                      (static_cast<std::uint16_t>(data[i + 1]) << 8));
    return true;
}

inline bool read_u32(const Bytes& data, std::uint64_t offset, std::uint32_t& out) {
    if (offset + 4 > data.size()) {
        return false;
    }
    std::size_t i = static_cast<std::size_t>(offset);
    out = static_cast<std::uint32_t>(data[i]) | (static_cast<std::uint32_t>(data[i + 1]) << 8) |
          (static_cast<std::uint32_t>(data[i + 2]) << 16) |
          (static_cast<std::uint32_t>(data[i + 3]) << 24);
    return true;
}

inline void write_u16(Bytes& out, std::uint16_t v) {
    out.push_back(static_cast<std::uint8_t>(v & 0xFFu));
    out.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFFu));
}

inline void write_u32(Bytes& out, std::uint32_t v) {
    out.push_back(static_cast<std::uint8_t>(v & 0xFFu));
    out.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFFu));
    out.push_back(static_cast<std::uint8_t>((v >> 16) & 0xFFu));
    out.push_back(static_cast<std::uint8_t>((v >> 24) & 0xFFu));
}

// Narrow a `size_t` byte count/offset down to `uint32_t`, throwing
// `ArchiveTooLarge` instead of silently wrapping if it does not fit. Used
// everywhere `ZipWriter` writes a cumulative offset or size (Local Header
// offset, Central Directory offset/size) into a 32-bit wire field — this
// format has no ZIP64 extension in this implementation, so an archive whose
// running byte count exceeds `0xFFFFFFFF` (4 GiB) simply cannot be
// represented, and writing a wrapped value would silently produce a
// structurally-corrupt archive (an offset pointing at the wrong byte)
// instead of a clear error.
inline std::uint32_t require_fits_u32(std::size_t v) {
    if (v > 0xFFFFFFFFull) {
        throw ZipException(ZipError::ArchiveTooLarge,
                            "zip: archive exceeds the 4 GiB ZIP (non-ZIP64) offset/size limit");
    }
    return static_cast<std::uint32_t>(v);
}

// Decompression-bomb / DoS guards for the READ side.
//
//   MAX_ENTRIES                          hard cap on parsed Central
//                                         Directory entries. A real entry is
//                                         at least 46 bytes (fixed CD header
//                                         size), so this can only bind on a
//                                         crafted file; it exists as
//                                         defense-in-depth against future
//                                         changes to the parse loop, not
//                                         because the current loop is
//                                         actually unbounded (it is already
//                                         bounded by `cd_size`, itself
//                                         bounds-checked against the file).
//   DEFAULT_MAX_TOTAL_UNCOMPRESSED_BYTES the default AGGREGATE budget
//                                         `unzip()` enforces across every
//                                         entry it decompresses. Each single
//                                         DEFLATE entry is separately capped
//                                         by `ca::deflate::MAX_INFLATE_OUTPUT`
//                                         (256 MB) inside `inflate` itself;
//                                         this second, aggregate cap exists
//                                         because an archive can legally
//                                         contain many entries each under
//                                         that per-entry cap, whose SUM would
//                                         otherwise be unbounded.
inline constexpr std::size_t MAX_ENTRIES = 65535;
inline constexpr std::size_t DEFAULT_MAX_TOTAL_UNCOMPRESSED_BYTES = 256u * 1024u * 1024u;

}  // namespace detail

// ===========================================================================
// Public API: CRC-32
// ===========================================================================

// Compute CRC-32 over `data`, starting from `initial` (use 0 for a fresh
// hash, or a previous result for an incremental update — `crc32(b, crc32(a,
// 0))` equals `crc32(a+b, 0)` for concatenated buffers `a` then `b`).
//
// The initial seed passed in is `0`, not `0xFFFFFFFF`; the pre/post XOR with
// `0xFFFFFFFF` required by the CRC-32 definition is handled internally.
//
//   crc32({'h','e','l','l','o',' ','w','o','r','l','d'}, 0) == 0x0D4A1185
//   crc32({}, 0) == 0x00000000
inline std::uint32_t crc32(const Bytes& data, std::uint32_t initial = 0) {
    const auto& table = detail::crc_table();
    std::uint32_t crc = initial ^ 0xFFFFFFFFu;
    for (std::uint8_t byte : data) {
        crc = table[(crc ^ byte) & 0xFFu] ^ (crc >> 8);
    }
    return crc ^ 0xFFFFFFFFu;
}

// ===========================================================================
// MS-DOS Date / Time Encoding
// ===========================================================================
//
// ZIP stores timestamps in the 16-bit MS-DOS packed format inherited from
// FAT:
//
//   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
//   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
//
// The combined 32-bit value is `(date << 16) | time`. Year 0 in DOS time is
// 1980; the maximum representable year is 2107 (1980 + 127).
//
// Encode a (year, month, day, hour, minute, second) tuple into the 32-bit
// MS-DOS datetime used by ZIP Local and Central Directory headers.
inline std::uint32_t dos_datetime(int year, int month, int day, int hour, int minute, int second) {
    std::uint16_t year_offset = (year > 1980) ? static_cast<std::uint16_t>(year - 1980) : 0;
    std::uint16_t t = static_cast<std::uint16_t>((static_cast<std::uint16_t>(hour) << 11) |
                                                   (static_cast<std::uint16_t>(minute) << 5) |
                                                   static_cast<std::uint16_t>(second / 2));
    std::uint16_t d = static_cast<std::uint16_t>((year_offset << 9) |
                                                   (static_cast<std::uint16_t>(month) << 5) |
                                                   static_cast<std::uint16_t>(day));
    return (static_cast<std::uint32_t>(d) << 16) | static_cast<std::uint32_t>(t);
}

// Fixed timestamp (1980-01-01 00:00:00) used when no real mtime is
// available. date field: (0<<9)|(1<<5)|1 = 33 = 0x0021; time = 0.
inline constexpr std::uint32_t DOS_EPOCH = 0x00210000u;

// ===========================================================================
// ZIP Write — ZipWriter
// ===========================================================================
//
// ZipWriter accumulates entries in memory: for each file it writes a Local
// File Header immediately, then the (possibly compressed) data, records the
// metadata needed for the Central Directory, and assembles the full archive
// on `finish()`.
//
// Auto-compression policy:
//   - Try DEFLATE. If the compressed output is smaller than the original,
//     use method=8 (DEFLATE).
//   - Otherwise use method=0 (Stored) — common for already-compressed
//     formats like JPEG, PNG, or a ZIP nested inside another ZIP.
//
// Usage:
//
//   ca::zip::ZipWriter w;
//   w.add_file("hello.txt", {'h','i'}, true);
//   w.add_directory("mydir/");
//   ca::zip::Bytes archive = w.finish();   // a valid .zip file
class ZipWriter {
public:
    ZipWriter() = default;

    // Add a file entry. If `compress` is true, DEFLATE is attempted; the
    // compressed form is used only if it is strictly smaller than the
    // uncompressed original (empty files are always Stored — DEFLATE of
    // zero bytes is never smaller than zero bytes).
    void add_file(const std::string& name, const Bytes& data, bool compress = true) {
        add_entry(name, data, compress, 0100644u);  // Unix regular file, rw-r--r--
    }

    // Add a directory entry. `name` should end with '/' — that trailing
    // slash is how readers (including this package's `ZipReader`) recognize
    // a directory entry; there is no separate "is directory" flag on the
    // wire.
    void add_directory(const std::string& name) {
        add_entry(name, Bytes{}, false, 0040755u);  // Unix directory, rwxr-xr-x
    }

    // Finish writing: append the Central Directory and EOCD, and return the
    // full archive bytes. The writer is left in a valid-but-unspecified
    // state after this call (mirroring the Rust reference's by-value
    // `finish(self)` — call it once, at the end).
    Bytes finish() {
        // EOCD's entry-count fields are 16 bits wide (this format has no
        // ZIP64 extension in this implementation). Reject rather than
        // silently wrap `entries_.size()` into `num_entries` below — an
        // archive with, say, 65536 real entries in its Central Directory
        // but a wrapped-to-0 declared count would mislead any consumer that
        // trusts the EOCD count as a fast-path entry total (this package's
        // own `ZipReader` does not — it always walks the CD by signature —
        // but many real-world unzip implementations do).
        if (entries_.size() > 0xFFFFu) {
            throw ZipException(ZipError::TooManyEntries,
                                "zip: archive has more than 65535 entries (ZIP64 not supported)");
        }

        std::uint32_t cd_offset = detail::require_fits_u32(buf_.size());
        std::uint16_t num_entries = static_cast<std::uint16_t>(entries_.size());

        // ── Central Directory ───────────────────────────────────────────
        std::size_t cd_start = buf_.size();
        for (const CdRecord& e : entries_) {
            std::uint16_t version_needed = (e.method == 8) ? 20 : 10;
            detail::write_u32(buf_, 0x02014B50u);              // signature
            detail::write_u16(buf_, 0x031Eu);                  // version_made_by (Unix, v30)
            detail::write_u16(buf_, version_needed);
            detail::write_u16(buf_, 0x0800u);                  // flags (UTF-8 filename)
            detail::write_u16(buf_, e.method);
            detail::write_u16(buf_, static_cast<std::uint16_t>(e.dos_datetime & 0xFFFFu));        // mod_time
            detail::write_u16(buf_, static_cast<std::uint16_t>((e.dos_datetime >> 16) & 0xFFFFu)); // mod_date
            detail::write_u32(buf_, e.crc);
            detail::write_u32(buf_, e.compressed_size);
            detail::write_u32(buf_, e.uncompressed_size);
            detail::write_u16(buf_, static_cast<std::uint16_t>(e.name.size()));
            detail::write_u16(buf_, 0);  // extra_len
            detail::write_u16(buf_, 0);  // comment_len
            detail::write_u16(buf_, 0);  // disk_start
            detail::write_u16(buf_, 0);  // internal_attrs
            detail::write_u32(buf_, e.external_attrs);
            detail::write_u32(buf_, e.local_offset);
            buf_.insert(buf_.end(), e.name.begin(), e.name.end());
            // (no extra field, no comment)
        }
        std::uint32_t cd_size = detail::require_fits_u32(buf_.size() - cd_start);

        // ── End of Central Directory Record ─────────────────────────────
        detail::write_u32(buf_, 0x06054B50u);  // signature
        detail::write_u16(buf_, 0);            // disk_number
        detail::write_u16(buf_, 0);            // cd_disk
        detail::write_u16(buf_, num_entries);  // entries on this disk
        detail::write_u16(buf_, num_entries);  // entries total
        detail::write_u32(buf_, cd_size);
        detail::write_u32(buf_, cd_offset);
        detail::write_u16(buf_, 0);  // comment_len

        return buf_;
    }

private:
    // Metadata recorded per entry during writing, used to build the Central
    // Directory once every entry's Local Header + data has been appended.
    struct CdRecord {
        std::string name;
        std::uint16_t method;
        std::uint32_t dos_datetime;
        std::uint32_t crc;
        std::uint32_t compressed_size;
        std::uint32_t uncompressed_size;
        std::uint32_t local_offset;
        std::uint32_t external_attrs;
    };

    void add_entry(const std::string& name, const Bytes& data, bool compress, std::uint32_t unix_mode) {
        // The ZIP format's non-ZIP64 fields are 16 bits (name length) and 32
        // bits (sizes) wide — this package, like code/specs/CMP09-zip.md,
        // does not implement ZIP64. Reject inputs that would silently
        // truncate into a structurally-corrupt archive (a name/size that
        // wraps when cast down) rather than writing one: `static_cast`ing
        // `name.size()` to `uint16_t` or `data.size()` to `uint32_t` without
        // this check would silently produce a header whose declared
        // length/size no longer matches the actual bytes written.
        if (name.size() > 0xFFFFu) {
            throw ZipException(ZipError::NameTooLong,
                                "zip: entry name exceeds the 65535-byte ZIP (non-ZIP64) limit");
        }
        if (data.size() > 0xFFFFFFFFull) {
            throw ZipException(ZipError::DataTooLarge,
                                "zip: entry '" + name +
                                    "' exceeds the 4 GiB ZIP (non-ZIP64) size limit");
        }

        std::uint32_t crc = crc32(data, 0);
        std::uint32_t uncompressed_size = static_cast<std::uint32_t>(data.size());

        // Compress if requested; fall back to Stored if it doesn't help.
        std::uint16_t method = 0;
        Bytes file_data = data;
        if (compress && !data.empty()) {
            Bytes compressed = ca::deflate::compress(data);
            if (compressed.size() < data.size()) {
                method = 8;
                file_data = std::move(compressed);
            }
        }

        std::uint32_t compressed_size = static_cast<std::uint32_t>(file_data.size());
        std::uint32_t local_offset = detail::require_fits_u32(buf_.size());

        // ── Local File Header ────────────────────────────────────────────
        std::uint16_t version_needed = (method == 8) ? 20 : 10;
        std::uint16_t flags = 0x0800u;  // GP flag bit 11 = UTF-8 filename

        detail::write_u32(buf_, 0x04034B50u);  // signature
        detail::write_u16(buf_, version_needed);
        detail::write_u16(buf_, flags);
        detail::write_u16(buf_, method);
        detail::write_u16(buf_, static_cast<std::uint16_t>(DOS_EPOCH & 0xFFFFu));         // mod_time
        detail::write_u16(buf_, static_cast<std::uint16_t>((DOS_EPOCH >> 16) & 0xFFFFu)); // mod_date
        detail::write_u32(buf_, crc);
        detail::write_u32(buf_, compressed_size);
        detail::write_u32(buf_, uncompressed_size);
        detail::write_u16(buf_, static_cast<std::uint16_t>(name.size()));
        detail::write_u16(buf_, 0);  // extra_field_length
        buf_.insert(buf_.end(), name.begin(), name.end());
        // (no extra field)
        buf_.insert(buf_.end(), file_data.begin(), file_data.end());

        entries_.push_back(CdRecord{name, method, DOS_EPOCH, crc, compressed_size, uncompressed_size,
                                     local_offset, unix_mode << 16});
    }

    Bytes buf_;
    std::vector<CdRecord> entries_;
};

// ===========================================================================
// ZIP Read — ZipEntry and ZipReader
// ===========================================================================
//
// ZipReader uses the "EOCD-first" strategy for reliable random-access:
//
//   1. Scan backwards for the EOCD signature (PK\x05\x06).
//      Limit the scan to the last 65535 + 22 bytes (EOCD comment max=65535).
//   2. Read the CD offset and size from EOCD.
//   3. Parse all Central Directory headers into ZipEntry objects.
//   4. On `read(entry)`: seek to the Local Header via `local_offset`, skip
//      the variable-length name + extra fields, read compressed data,
//      decompress, verify CRC-32.
//
// We use CD entries as the authoritative source for sizes and compression
// method — the Central Directory, not the Local Header, per
// code/specs/CMP09-zip.md's Security Considerations. Local headers are only
// consulted for their variable-length fields (name_len + extra_len) so we
// can skip to the data start.

// Metadata for a single entry inside a ZIP archive, as recorded in the
// Central Directory.
struct ZipEntry {
    std::string name;              // File name (UTF-8).
    std::uint32_t size;            // Uncompressed size in bytes.
    std::uint32_t compressed_size; // Compressed size in bytes.
    std::uint16_t method;          // Compression method: 0=Stored, 8=DEFLATE.
    std::uint32_t crc32;           // CRC-32 of the uncompressed content.
    bool is_directory;             // True if `name` ends with '/'.
    std::uint32_t local_offset;    // Byte offset of the Local Header. Internal
                                    // to `ZipReader::read`; not meaningful on
                                    // its own to callers.
};

// Reads entries from an in-memory ZIP archive.
//
//   ca::zip::Bytes archive = ...;
//   ca::zip::ZipReader reader(archive);
//   for (const auto& entry : reader.entries()) {
//       ca::zip::Bytes content = reader.read(entry);
//   }
//
// `ZipReader` BORROWS the `data` buffer passed to its constructor (stored by
// reference, like the Rust reference's `ZipReader<'a>`) — the caller must
// keep that buffer alive for the reader's entire lifetime. This avoids
// copying the whole archive just to read a few entries out of it.
class ZipReader {
public:
    // Parse an in-memory ZIP archive. Throws `ZipException` if no valid EOCD
    // record is found or the archive is structurally malformed.
    explicit ZipReader(const Bytes& data) : data_(data) {
        std::size_t eocd_offset = find_eocd(data);

        std::uint32_t cd_offset_u32 = 0;
        std::uint32_t cd_size_u32 = 0;
        if (!detail::read_u32(data, eocd_offset + 16, cd_offset_u32) ||
            !detail::read_u32(data, eocd_offset + 12, cd_size_u32)) {
            throw ZipException(ZipError::EocdTruncated, "zip: EOCD too short");
        }

        // Validate the Central Directory range with 64-bit arithmetic so two
        // untrusted u32 fields summed together cannot silently overflow
        // before the bounds check runs.
        std::uint64_t cd_offset = cd_offset_u32;
        std::uint64_t cd_size = cd_size_u32;
        if (cd_offset + cd_size > data.size()) {
            throw ZipException(ZipError::CentralDirectoryOutOfBounds,
                                "zip: Central Directory out of bounds");
        }

        // ── Parse all Central Directory headers ─────────────────────────
        std::size_t pos = static_cast<std::size_t>(cd_offset);
        std::size_t cd_end = static_cast<std::size_t>(cd_offset + cd_size);
        while (pos + 4 <= cd_end) {
            std::uint32_t sig = 0;
            if (!detail::read_u32(data, pos, sig) || sig != 0x02014B50u) {
                break;  // end of CD or padding
            }
            if (entries_.size() >= detail::MAX_ENTRIES) {
                throw ZipException(ZipError::TooManyEntries, "zip: too many Central Directory entries");
            }

            std::uint16_t method = 0;
            std::uint32_t crc = 0;
            std::uint32_t compressed_size = 0;
            std::uint32_t size = 0;
            std::uint16_t name_len16 = 0;
            std::uint16_t extra_len16 = 0;
            std::uint16_t comment_len16 = 0;
            std::uint32_t local_offset = 0;
            if (!detail::read_u16(data, pos + 10, method) || !detail::read_u32(data, pos + 16, crc) ||
                !detail::read_u32(data, pos + 20, compressed_size) ||
                !detail::read_u32(data, pos + 24, size) ||
                !detail::read_u16(data, pos + 28, name_len16) ||
                !detail::read_u16(data, pos + 30, extra_len16) ||
                !detail::read_u16(data, pos + 32, comment_len16) ||
                !detail::read_u32(data, pos + 42, local_offset)) {
                throw ZipException(ZipError::CentralDirectoryEntryTruncated,
                                    "zip: Central Directory entry truncated");
            }

            std::size_t name_len = name_len16;
            std::size_t extra_len = extra_len16;
            std::size_t comment_len = comment_len16;

            std::size_t name_start = pos + 46;
            std::uint64_t name_end64 = static_cast<std::uint64_t>(name_start) + name_len;
            if (name_end64 > data.size()) {
                throw ZipException(ZipError::CentralDirectoryEntryNameOutOfBounds,
                                    "zip: Central Directory entry name out of bounds");
            }
            std::size_t name_end = static_cast<std::size_t>(name_end64);
            std::string name(data.begin() + static_cast<std::ptrdiff_t>(name_start),
                              data.begin() + static_cast<std::ptrdiff_t>(name_end));
            bool is_directory = !name.empty() && name.back() == '/';

            entries_.push_back(ZipEntry{std::move(name), size, compressed_size, method, crc, is_directory,
                                         local_offset});

            pos = name_end + extra_len + comment_len;
        }
    }

    // All entries in the archive (files and directories), in Central
    // Directory order.
    const std::vector<ZipEntry>& entries() const noexcept { return entries_; }

    // Decompress and return the data for `entry`. Verifies CRC-32.
    //
    // Throws `ZipException` on CRC mismatch, an encrypted entry, an
    // unsupported compression method, corrupt/truncated Local Header data,
    // or (propagated from `ca::deflate::inflate`) a malformed DEFLATE
    // stream.
    Bytes read(const ZipEntry& entry) const {
        if (entry.is_directory) {
            return Bytes{};
        }

        // `entry.local_offset` is an untrusted `uint32_t` Central Directory
        // field, independent of `data_.size()` — widen it to `uint64_t`
        // ONCE, here, and add every fixed field offset (`+6`, `+26`, `+28`,
        // `+30` below) against that `uint64_t` value, never against a
        // `size_t` copy of it. `read_u16`/`read_u32` take `uint64_t` offsets
        // for exactly this reason (see their doc comment): if this addition
        // were done in `size_t` first, a `local_offset` near `UINT32_MAX`
        // could wrap to a small, in-bounds-looking offset on a 32-bit
        // `size_t` platform BEFORE either function's own bounds check ever
        // ran, silently validating the wrong position instead of rejecting
        // an out-of-range one.
        std::uint64_t lh_off = entry.local_offset;

        // Reject encrypted entries (GP flag bit 0) — read from the LOCAL
        // header, since that is the copy an extractor decrypts against.
        std::uint16_t local_flags = 0;
        if (!detail::read_u16(data_, lh_off + 6, local_flags)) {
            throw ZipException(ZipError::LocalHeaderOutOfBounds, "zip: local header out of bounds");
        }
        if ((local_flags & 1u) != 0u) {
            throw ZipException(ZipError::EncryptedEntryUnsupported,
                                "zip: entry '" + entry.name + "' is encrypted; not supported");
        }

        // Skip the Local Header to reach the file data. The Local Header has
        // its OWN name_len/extra_len (which may legally differ from the CD
        // header's, though in practice they match) — re-read them here
        // rather than trusting the CD's.
        std::uint16_t lh_name_len = 0;
        std::uint16_t lh_extra_len = 0;
        if (!detail::read_u16(data_, lh_off + 26, lh_name_len) ||
            !detail::read_u16(data_, lh_off + 28, lh_extra_len)) {
            throw ZipException(ZipError::LocalHeaderTruncated, "zip: local header truncated");
        }

        // 64-bit arithmetic: `lh_off + 30 + name/extra lengths + compressed
        // size` combines several untrusted fields, so do the addition where
        // it cannot wrap before the bounds check.
        std::uint64_t data_start = lh_off + 30u + lh_name_len + lh_extra_len;
        std::uint64_t data_end = data_start + entry.compressed_size;
        if (data_end > data_.size()) {
            throw ZipException(ZipError::EntryDataOutOfBounds,
                                "zip: entry '" + entry.name + "' data out of bounds");
        }

        Bytes compressed(data_.begin() + static_cast<std::ptrdiff_t>(data_start),
                          data_.begin() + static_cast<std::ptrdiff_t>(data_end));

        // Decompress according to method.
        //
        // Method 8 (DEFLATE) delegates to `ca::deflate::inflate`, which
        // decodes ALL THREE RFC 1951 block types — stored, fixed Huffman,
        // and dynamic Huffman. Real-world producers (Microsoft Office
        // writing .docx/.xlsx/.pptx, `zip`(1), Python's zipfile, Java's jar)
        // almost always emit dynamic-Huffman blocks, so a reader that only
        // understood fixed Huffman could not open files people actually
        // have — this is the exact failure mode this repo's `dart/deflate`
        // sibling hit (see lessons.md), which `cpp/deflate` was built and
        // verified specifically to avoid.
        Bytes decompressed;
        if (entry.method == 0) {
            decompressed = std::move(compressed);
        } else if (entry.method == 8) {
            try {
                decompressed = ca::deflate::inflate(compressed);
            } catch (const ca::deflate::DeflateException&) {
                throw ZipException(ZipError::EntryDataOutOfBounds,
                                    "zip: entry '" + entry.name + "': malformed DEFLATE stream");
            }
        } else {
            throw ZipException(ZipError::UnsupportedCompressionMethod,
                                "zip: unsupported compression method " + std::to_string(entry.method) +
                                    " for '" + entry.name + "'");
        }

        // The ACTUAL decompressed size must match the Central Directory's
        // declared `Uncompressed_Size` exactly. For any honestly-produced
        // archive this always holds by construction (a writer that
        // compressed N bytes declares Uncompressed_Size=N, and a correct
        // DEFLATE stream decoding that entry reproduces exactly N bytes) —
        // for Stored entries it holds too, since `decompressed` there is
        // just `compressed`, whose length is the already-bounds-checked
        // `entry.compressed_size`.
        //
        // An EARLIER version of this check silently TRIMMED an oversized
        // `decompressed` down to `entry.size` instead of rejecting the
        // mismatch. That is a real vulnerability, not just a style choice:
        // `entry.size` is an attacker-controlled Central Directory field, so
        // a crafted entry can declare `Uncompressed_Size = 0` while its real
        // DEFLATE stream expands to the full `ca::deflate::MAX_INFLATE_OUTPUT`
        // cap (256 MB) when actually inflated above — the 256 MB of real
        // decompression work already happened by the time a silent trim ran,
        // and the trimmed 0-byte result would make every size-based
        // aggregate budget in `unzip()` believe nothing was decompressed at
        // all, defeating it for an archive with many such entries. Throwing
        // here instead means the mismatch is caught at its source, in the
        // one function that actually knows the true decompressed size,
        // rather than relying on every caller to re-derive it correctly.
        if (decompressed.size() != entry.size) {
            throw ZipException(ZipError::DeclaredSizeMismatch,
                                "zip: entry '" + entry.name +
                                    "' decompressed size does not match the declared Uncompressed_Size");
        }

        // Verify CRC-32.
        std::uint32_t actual_crc = crc32(decompressed, 0);
        if (actual_crc != entry.crc32) {
            throw ZipException(ZipError::Crc32Mismatch, "zip: CRC-32 mismatch for '" + entry.name + "'");
        }

        return decompressed;
    }

    // Find an entry by name and return its decompressed data. Throws
    // `ZipException` (`EntryNotFound`) if no entry with that name exists.
    Bytes read_by_name(const std::string& name) const {
        for (const ZipEntry& e : entries_) {
            if (e.name == name) {
                return read(e);
            }
        }
        throw ZipException(ZipError::EntryNotFound, "zip: entry '" + name + "' not found");
    }

private:
    // Scan backwards from the end of `data` for the EOCD signature
    // 0x06054B50.
    //
    // The EOCD record is at most 22 + 65535 bytes from the end (the comment
    // field can be 0-65535 bytes). The scan is bounded to that range — it
    // never searches the whole file unboundedly, which matters for large
    // archives with no EOCD at all (a crafted or truncated file).
    static std::size_t find_eocd(const Bytes& data) {
        constexpr std::uint32_t SIG = 0x06054B50u;
        constexpr std::size_t MAX_COMMENT = 65535;
        constexpr std::size_t EOCD_MIN_SIZE = 22;

        if (data.size() < EOCD_MIN_SIZE) {
            throw ZipException(ZipError::NoEocdFound, "zip: no End of Central Directory record found");
        }

        std::size_t scan_start = (data.size() > EOCD_MIN_SIZE + MAX_COMMENT)
                                      ? data.size() - EOCD_MIN_SIZE - MAX_COMMENT
                                      : 0;
        std::size_t scan_end = data.size() - EOCD_MIN_SIZE;  // inclusive

        // Scan from the end backwards. `i` is unsigned, so the loop breaks
        // explicitly at `scan_start` rather than testing `i >= scan_start`
        // after decrementing past 0 (which would underflow when
        // `scan_start == 0`, i.e. for any archive no larger than
        // `22 + 65535` bytes).
        for (std::size_t i = scan_end;; --i) {
            std::uint32_t sig = 0;
            if (detail::read_u32(data, i, sig) && sig == SIG) {
                std::uint16_t comment_len = 0;
                if (detail::read_u16(data, i + 20, comment_len) &&
                    i + EOCD_MIN_SIZE + comment_len == data.size()) {
                    return i;
                }
            }
            if (i == scan_start) {
                break;
            }
        }
        throw ZipException(ZipError::NoEocdFound, "zip: no End of Central Directory record found");
    }

    const Bytes& data_;
    std::vector<ZipEntry> entries_;
};

// ===========================================================================
// Convenience Functions
// ===========================================================================

// Compress a list of `(name, data)` pairs into a ZIP archive. Each file is
// compressed with DEFLATE if it reduces size; otherwise stored.
inline Bytes zip(const std::vector<std::pair<std::string, Bytes>>& files) {
    ZipWriter w;
    for (const auto& kv : files) {
        w.add_file(kv.first, kv.second, true);
    }
    return w.finish();
}

// Decompress all FILE entries from a ZIP archive (directories, identified by
// a trailing '/' in the name, are skipped). Returns `(name, data)` pairs in
// Central Directory order.
//
// `max_total_uncompressed_bytes` is an AGGREGATE budget across every entry
// this call decompresses — see the module-level doc comment above on why
// this exists in addition to `ca::deflate::inflate`'s own per-entry cap.
// Throws `ZipException` (`UncompressedSizeBudgetExceeded`) if the running
// total would exceed it.
inline std::vector<std::pair<std::string, Bytes>> unzip(
    const Bytes& data,
    std::size_t max_total_uncompressed_bytes = detail::DEFAULT_MAX_TOTAL_UNCOMPRESSED_BYTES) {
    ZipReader reader(data);
    std::vector<std::pair<std::string, Bytes>> out;
    std::size_t total_uncompressed = 0;
    for (const ZipEntry& entry : reader.entries()) {
        if (entry.is_directory) {
            continue;
        }

        // PRE-check using the declared (Central Directory) size: lets an
        // oversized entry fail before any decompression work happens at
        // all. This is trustworthy for budget purposes (not just a cheap
        // early-out) precisely because `ZipReader::read` below THROWS
        // `DeclaredSizeMismatch` rather than silently truncating its output
        // if the actual decompressed size ever disagrees with `entry.size`
        // — so `entry.size` cannot understate the real cost of decompressing
        // this entry. (An earlier version of `read` silently trimmed a
        // mismatch instead; that let a crafted entry declare
        // `Uncompressed_Size = 0` while its real stream still cost up to
        // `ca::deflate::MAX_INFLATE_OUTPUT`, 256 MB, of real decompression
        // work per entry, with the trimmed 0-byte result making every
        // size-based budget here believe nothing had been decompressed —
        // see `read`'s doc comment for the full story.)
        if (total_uncompressed >= max_total_uncompressed_bytes ||
            entry.size > max_total_uncompressed_bytes - total_uncompressed) {
            throw ZipException(ZipError::UncompressedSizeBudgetExceeded,
                                "zip: aggregate uncompressed size budget exceeded");
        }

        Bytes content = reader.read(entry);
        total_uncompressed += content.size();
        out.emplace_back(entry.name, std::move(content));
    }
    return out;
}

}  // namespace zip
}  // namespace ca

#endif  // CA_ZIP_HPP
