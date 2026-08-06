// Tests for the C++ zip (CMP09), using the iso_test.h harness. TC-1..TC-12
// below are code/specs/CMP09-zip.md's mandatory test cases (renumbered to
// match this repo's other `zip` ports, which treat CLI-interop as TC-10 and
// shift Unicode/nested-paths to TC-11/TC-12 — see the Rust reference at
// code/packages/rust/zip/src/lib.rs). The dynamic-Huffman fixture is the
// EXACT byte sequence from that Rust suite (itself produced independently by
// CPython's `zipfile` module), so this test proves `ca::zip::ZipReader`
// reads a real dynamic-Huffman DEFLATE stream through `ca::deflate::inflate`
// — not just its own writer's fixed-Huffman output.
#include "iso_test.h"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <utility>
#include <vector>

#include "zip.hpp"

namespace zip = ca::zip;
using Bytes = std::vector<std::uint8_t>;

static Bytes bytes(const char* s) {
    Bytes b;
    for (const char* p = s; *p; ++p) {
        b.push_back(static_cast<std::uint8_t>(*p));
    }
    return b;
}

static Bytes repeat(const Bytes& unit, std::size_t n) {
    Bytes out;
    out.reserve(unit.size() * n);
    for (std::size_t i = 0; i < n; ++i) {
        out.insert(out.end(), unit.begin(), unit.end());
    }
    return out;
}

// Locate the first Central Directory Header by its signature bytes
// (0x02014B50 little-endian = 50 4B 01 02), for tests that need to corrupt a
// specific CD field by hand-crafted byte offset. Returns SIZE_MAX if not
// found (caller should ISO_CHECK the result before indexing with it).
static std::size_t find_cd_signature(const Bytes& archive) {
    for (std::size_t i = 0; i + 4 <= archive.size(); ++i) {
        if (archive[i] == 0x50 && archive[i + 1] == 0x4B && archive[i + 2] == 0x01 &&
            archive[i + 3] == 0x02) {
            return i;
        }
    }
    return static_cast<std::size_t>(-1);
}

// ── CRC-32 ───────────────────────────────────────────────────────────────

static void test_crc32() {
    // "hello world" and the standard "123456789" check vector, matching
    // every other language port and Python's binascii.crc32.
    ISO_CHECK_EQ_UINT(zip::crc32(bytes("hello world"), 0), 0x0D4A1185u);
    ISO_CHECK_EQ_UINT(zip::crc32(bytes("123456789"), 0), 0xCBF43926u);
    ISO_CHECK_EQ_UINT(zip::crc32(bytes(""), 0), 0x00000000u);

    // Incremental == one-shot over the concatenation.
    std::uint32_t part1 = zip::crc32(bytes("hello "), 0);
    std::uint32_t part2 = zip::crc32(bytes("world"), part1);
    ISO_CHECK_EQ_UINT(part2, zip::crc32(bytes("hello world"), 0));
}

// ── dos_datetime ─────────────────────────────────────────────────────────

static void test_dos_datetime() {
    // 1980-01-01 00:00:00 -> date=(0<<9)|(1<<5)|1=33, time=0.
    std::uint32_t dt = zip::dos_datetime(1980, 1, 1, 0, 0, 0);
    ISO_CHECK_EQ_UINT(dt >> 16, 33u);
    ISO_CHECK_EQ_UINT(dt & 0xFFFFu, 0u);
    ISO_CHECK_EQ_UINT(zip::DOS_EPOCH, dt);
}

// ── TC-1: Stored round-trip ─────────────────────────────────────────────

static void test_tc1_stored_roundtrip() {
    Bytes data = bytes("hello, world");
    zip::ZipWriter w;
    w.add_file("hello.txt", data, false);  // compress=false -> Stored
    Bytes archive = w.finish();

    auto files = zip::unzip(archive);
    ISO_CHECK_EQ_UINT(files.size(), 1u);
    ISO_CHECK(files[0].first == "hello.txt");
    ISO_CHECK(files[0].second == data);

    zip::ZipReader reader(archive);
    ISO_CHECK_EQ_UINT(reader.entries().size(), 1u);
    ISO_CHECK_EQ_UINT(reader.entries()[0].method, 0u);
}

// ── TC-2: DEFLATE round-trip ─────────────────────────────────────────────

static void test_tc2_deflate_roundtrip() {
    Bytes text = repeat(bytes("the quick brown fox jumps over the lazy dog "), 10);
    Bytes archive = zip::zip({{"text.txt", text}});
    auto files = zip::unzip(archive);
    ISO_CHECK_EQ_UINT(files.size(), 1u);
    ISO_CHECK(files[0].first == "text.txt");
    ISO_CHECK(files[0].second == text);

    zip::ZipReader reader(archive);
    ISO_CHECK_EQ_UINT(reader.entries()[0].method, 8u);  // must actually compress
}

// ── TC-3: Multiple files ─────────────────────────────────────────────────

static void test_tc3_multiple_files() {
    Bytes all_bytes;
    for (int i = 0; i < 256; ++i) {
        all_bytes.push_back(static_cast<std::uint8_t>(i));
    }
    std::vector<std::pair<std::string, Bytes>> entries = {
        {"a.txt", bytes("file A content")},
        {"b.txt", bytes("file B content")},
        {"c.bin", all_bytes},
    };
    Bytes archive = zip::zip(entries);
    auto files = zip::unzip(archive);
    ISO_CHECK_EQ_UINT(files.size(), 3u);
    for (const auto& want : entries) {
        bool found = false;
        for (const auto& got : files) {
            if (got.first == want.first) {
                found = true;
                ISO_CHECK(got.second == want.second);
            }
        }
        ISO_CHECK_MSG(found, ("missing entry " + want.first).c_str());
    }
}

// ── TC-4: Directory entry ────────────────────────────────────────────────

static void test_tc4_directory_entry() {
    zip::ZipWriter w;
    w.add_directory("mydir/");
    w.add_file("mydir/file.txt", bytes("contents"), true);
    Bytes archive = w.finish();

    zip::ZipReader reader(archive);
    bool has_dir = false, has_file = false;
    for (const auto& e : reader.entries()) {
        if (e.name == "mydir/") {
            has_dir = true;
            ISO_CHECK(e.is_directory);
        }
        if (e.name == "mydir/file.txt") {
            has_file = true;
            ISO_CHECK(!e.is_directory);
        }
    }
    ISO_CHECK(has_dir);
    ISO_CHECK(has_file);
}

// ── TC-5: CRC-32 corruption detection ────────────────────────────────────

static void test_tc5_crc_mismatch_detected() {
    Bytes archive = zip::zip({{"f.txt", bytes("test data")}});
    Bytes corrupted = archive;
    // Corrupt a data byte directly (offset 35 = 30-byte fixed Local Header +
    // 5-byte name "f.txt"). Corrupting only the Local Header's CRC field
    // would have no effect, since `ZipReader::read` verifies against the
    // decompressed content's actual CRC, not a trusted-as-given field.
    corrupted[35] ^= 0xFF;

    bool threw = false;
    try {
        (void)zip::unzip(corrupted);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::Crc32Mismatch);
    }
    ISO_CHECK_MSG(threw, "expected a ZipException for corrupted data");
}

// ── TC-6: EOCD detection / random access ─────────────────────────────────

static void test_tc6_random_access() {
    std::vector<std::pair<std::string, Bytes>> entries;
    for (int i = 0; i < 10; ++i) {
        std::string name = "f" + std::to_string(i) + ".txt";
        std::string content = "content " + std::to_string(i);
        entries.emplace_back(name, Bytes(content.begin(), content.end()));
    }
    Bytes archive = zip::zip(entries);

    zip::ZipReader reader(archive);
    const zip::ZipEntry* entry5 = nullptr;
    for (const auto& e : reader.entries()) {
        if (e.name == "f5.txt") {
            entry5 = &e;
        }
    }
    ISO_CHECK(entry5 != nullptr);
    if (entry5 != nullptr) {
        ISO_CHECK(reader.read(*entry5) == bytes("content 5"));
    }
}

// ── TC-7: Incompressible data falls back to Stored ───────────────────────

static void test_tc7_incompressible_stored() {
    // Pseudo-random data via a simple LCG (seed=42): compresses poorly.
    std::uint32_t seed = 42;
    Bytes data;
    for (int i = 0; i < 1024; ++i) {
        seed = seed * 1664525u + 1013904223u;
        data.push_back(static_cast<std::uint8_t>(seed >> 24));
    }

    Bytes archive = zip::zip({{"random.bin", data}});
    zip::ZipReader reader(archive);
    ISO_CHECK_EQ_UINT(reader.entries().size(), 1u);
    ISO_CHECK_EQ_UINT(reader.entries()[0].method, 0u);  // Stored — DEFLATE would grow it
    ISO_CHECK(reader.read(reader.entries()[0]) == data);
}

// ── TC-8: Empty file ─────────────────────────────────────────────────────

static void test_tc8_empty_file() {
    Bytes archive = zip::zip({{"empty.txt", Bytes{}}});
    auto files = zip::unzip(archive);
    ISO_CHECK_EQ_UINT(files.size(), 1u);
    ISO_CHECK(files[0].first == "empty.txt");
    ISO_CHECK(files[0].second.empty());
}

// ── TC-9: Large file with compression ────────────────────────────────────

static void test_tc9_large_file_compressed() {
    Bytes data = repeat(bytes("abcdefghij"), 10000);  // 100 KB
    Bytes archive = zip::zip({{"big.bin", data}});
    auto files = zip::unzip(archive);
    ISO_CHECK(files[0].second == data);
    ISO_CHECK_MSG(archive.size() < data.size(), "repetitive 100 KB must compress");
}

// ── TC-10: CLI interop with the system `zip`/`unzip` tools ──────────────
//
// Unlike this repo's other language ports of CMP09 (which the spec allows to
// leave this "manual or subprocess-based" and undocumented in code — see
// code/specs/CMP09-zip.md TC-10), this C++ port actually shells out via
// `std::system` in both directions:
//
//   1. We write an archive with `ZipWriter`; the SYSTEM `unzip` extracts it.
//   2. The SYSTEM `zip` tool creates an archive; `ZipReader` reads it back.
//
// Both directions must round-trip byte-for-byte. When `zip`/`unzip` are not
// on PATH (e.g. a minimal Windows CI runner) this SKIPS rather than fails —
// their absence is a property of the host, not of this code. `std::system`
// on a POSIX host returns 0 exactly when the shell ran the command AND that
// command exited with status 0 (a raw wait-status of 0 iff exit code 0); on
// Windows it returns cmd.exe's errorlevel directly. Either way, `== 0` is a
// portable "succeeded" check without needing POSIX-only `WEXITSTATUS`.
static bool shell_ok(const char* cmd) { return std::system(cmd) == 0; }

static Bytes read_whole_file(const char* path, bool& ok) {
    Bytes out;
    FILE* f = std::fopen(path, "rb");
    ok = (f != nullptr);
    if (f == nullptr) {
        return out;
    }
    std::uint8_t buf[4096];
    std::size_t n;
    while ((n = std::fread(buf, 1, sizeof(buf), f)) > 0) {
        out.insert(out.end(), buf, buf + n);
    }
    std::fclose(f);
    return out;
}

static bool write_whole_file(const char* path, const Bytes& data) {
    FILE* f = std::fopen(path, "wb");
    if (f == nullptr) {
        return false;
    }
    bool ok = data.empty() || std::fwrite(data.data(), 1, data.size(), f) == data.size();
    std::fclose(f);
    return ok;
}

static void test_tc10_cli_interop() {
    if (!shell_ok("zip -v > _build/_iso_zip_probe.tmp 2>&1") ||
        !shell_ok("unzip -v >> _build/_iso_zip_probe.tmp 2>&1")) {
        std::printf("  SKIP TC-10: system `zip`/`unzip` not found on PATH; "
                     "CLI interop not exercised on this host\n");
        return;
    }

    // ── Direction 1: we write, the SYSTEM `unzip` reads ──────────────────
    {
        Bytes content = repeat(bytes("cpp wrote this zip; system unzip must read it. "), 20);
        zip::ZipWriter w;
        w.add_file("cpp_wrote_this.txt", content, true);
        Bytes archive = w.finish();

        ISO_CHECK(write_whole_file("_build/tc10_ours.zip", archive));
        ISO_CHECK(shell_ok("cd _build && rm -rf tc10_extract && mkdir tc10_extract && "
                            "unzip -o -q tc10_ours.zip -d tc10_extract"));

        bool ok = false;
        Bytes extracted = read_whole_file("_build/tc10_extract/cpp_wrote_this.txt", ok);
        ISO_CHECK(ok);
        ISO_CHECK(extracted == content);
    }

    // ── Direction 2: the SYSTEM `zip` writes, we read ────────────────────
    {
        std::string content = "written by the system zip tool; ca::zip::ZipReader must read it";
        Bytes want(content.begin(), content.end());
        ISO_CHECK(write_whole_file("_build/tc10_srcfile.txt", want));

        // `-j` junks the stored path so the entry name is just the basename
        // — keeps the fixture simple to read back by name.
        ISO_CHECK(shell_ok("cd _build && rm -f tc10_theirs.zip && "
                            "zip -q -j tc10_theirs.zip tc10_srcfile.txt"));

        bool ok = false;
        Bytes archive_bytes = read_whole_file("_build/tc10_theirs.zip", ok);
        ISO_CHECK(ok);
        if (ok) {
            zip::ZipReader reader(archive_bytes);
            Bytes got = reader.read_by_name("tc10_srcfile.txt");
            ISO_CHECK(got == want);
        }
    }
}

// ── TC-11: Unicode filename ───────────────────────────────────────────────

static void test_tc11_unicode_filename() {
    // UTF-8 bytes for "日本語/résumé.txt".
    std::string name = "\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e/r\xc3\xa9sum\xc3\xa9.txt";
    Bytes archive = zip::zip({{name, bytes("content")}});
    auto files = zip::unzip(archive);
    ISO_CHECK_EQ_UINT(files.size(), 1u);
    ISO_CHECK(files[0].first == name);
    ISO_CHECK(files[0].second == bytes("content"));
}

// ── TC-12: Nested paths ────────────────────────────────────────────────────

static void test_tc12_nested_paths() {
    std::vector<std::pair<std::string, Bytes>> entries = {
        {"root.txt", bytes("root")},
        {"dir/file.txt", bytes("nested")},
        {"dir/sub/deep.txt", bytes("deep")},
    };
    Bytes archive = zip::zip(entries);
    auto files = zip::unzip(archive);
    for (const auto& want : entries) {
        bool found = false;
        for (const auto& got : files) {
            if (got.first == want.first) {
                found = true;
                ISO_CHECK(got.second == want.second);
            }
        }
        ISO_CHECK_MSG(found, ("missing nested entry " + want.first).c_str());
    }
}

// ── Extra: empty archive ──────────────────────────────────────────────────

static void test_empty_archive() {
    Bytes archive = zip::zip({});
    auto files = zip::unzip(archive);
    ISO_CHECK(files.empty());
    zip::ZipReader reader(archive);
    ISO_CHECK(reader.entries().empty());
}

// ── Extra: read_by_name ─────────────────────────────────────────────────

static void test_read_by_name() {
    Bytes archive = zip::zip({{"alpha.txt", bytes("AAA")}, {"beta.txt", bytes("BBB")}});
    zip::ZipReader reader(archive);
    ISO_CHECK(reader.read_by_name("beta.txt") == bytes("BBB"));

    bool threw = false;
    try {
        (void)reader.read_by_name("nope.txt");
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::EntryNotFound);
    }
    ISO_CHECK(threw);
}

// ── Extra: no EOCD -> ZipException ───────────────────────────────────────

static void test_no_eocd() {
    Bytes garbage = bytes("this is not a zip file at all");
    bool threw = false;
    try {
        zip::ZipReader reader(garbage);
        (void)reader;
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::NoEocdFound);
    }
    ISO_CHECK(threw);
}

// ── Extra: encrypted entry rejected ──────────────────────────────────────

static void test_encrypted_entry_rejected() {
    Bytes archive = zip::zip({{"f.txt", bytes("secret")}});
    // Local Header General-Purpose flag is at byte offset 6; set bit 0
    // (encrypted). The Central Directory copy is left untouched, so
    // `ZipReader::new` still parses the entry list fine — only `read()`
    // must reject it, since that is what actually reads the LOCAL header's
    // flags (the copy a real extractor would decrypt against).
    Bytes corrupted = archive;
    corrupted[6] |= 0x01;

    zip::ZipReader reader(corrupted);
    bool threw = false;
    try {
        (void)reader.read(reader.entries()[0]);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::EncryptedEntryUnsupported);
    }
    ISO_CHECK(threw);
}

// ── Extra: unsupported compression method rejected ───────────────────────

static void test_unsupported_method_rejected() {
    zip::ZipWriter w;
    w.add_file("f.txt", bytes("hello"), false);  // compress=false -> Stored, exact byte layout
    Bytes archive = w.finish();
    Bytes corrupted = archive;

    // Method field is at CD offset +10.
    std::size_t cd_start = find_cd_signature(corrupted);
    ISO_CHECK(cd_start != static_cast<std::size_t>(-1));
    corrupted[cd_start + 10] = 99;  // method=99, not 0 or 8

    zip::ZipReader reader(corrupted);
    bool threw = false;
    try {
        (void)reader.read(reader.entries()[0]);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::UnsupportedCompressionMethod);
    }
    ISO_CHECK(threw);
}

// ── Extra: a declared size that understates the real decompressed size is
//    REJECTED, not silently trimmed (security-review regression test) ─────
//
// This is a direct regression test for a real vulnerability caught in
// review before this package's first push: an earlier version of
// `ZipReader::read` silently trimmed `decompressed` down to `entry.size`
// whenever it was LARGER than declared, instead of rejecting the mismatch.
// Since `entry.size` is an attacker-controlled Central Directory field, a
// crafted entry could declare `Uncompressed_Size = 0` while its real DEFLATE
// stream still cost real CPU/memory to decompress (up to
// `ca::deflate::MAX_INFLATE_OUTPUT`, 256 MB, per entry) — and the trimmed
// 0-byte result would make `unzip()`'s aggregate decompression-bomb budget
// believe nothing had been decompressed at all, letting many such entries
// each smuggle real work past it. `read` must throw `DeclaredSizeMismatch`
// here, not return a truncated (and therefore misleadingly small) result.
static void test_declared_size_mismatch_rejected() {
    // Compressible content so DEFLATE actually shrinks it (method=8) — the
    // interesting case, since Stored's uncompressed size is tautologically
    // tied to the bytes actually present in the archive.
    Bytes content = repeat(bytes("0123456789"), 500);  // 5000 bytes, compresses well
    zip::ZipWriter w;
    w.add_file("f.bin", content, true);
    Bytes archive = w.finish();
    Bytes corrupted = archive;

    std::size_t cd_start = find_cd_signature(corrupted);
    ISO_CHECK(cd_start != static_cast<std::size_t>(-1));
    ISO_CHECK_EQ_UINT(corrupted[cd_start + 10], 8u);  // sanity: really DEFLATE, not Stored

    // Uncompressed_Size is at CD offset +24 (a 4-byte little-endian field).
    // Lie: declare 0 instead of the true 5000.
    corrupted[cd_start + 24] = 0;
    corrupted[cd_start + 25] = 0;
    corrupted[cd_start + 26] = 0;
    corrupted[cd_start + 27] = 0;

    zip::ZipReader reader(corrupted);
    ISO_CHECK_EQ_UINT(reader.entries()[0].size, 0u);  // the lie parsed through, as expected

    bool threw = false;
    try {
        (void)reader.read(reader.entries()[0]);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::DeclaredSizeMismatch);
    }
    ISO_CHECK_MSG(threw,
                  "a declared size smaller than the true decompressed size must be REJECTED, "
                  "not silently trimmed");

    // unzip() must propagate the same rejection rather than quietly
    // returning a truncated / all-zero-budget-consuming entry.
    bool threw_via_unzip = false;
    try {
        (void)zip::unzip(corrupted);
    } catch (const zip::ZipException& e) {
        threw_via_unzip = true;
        ISO_CHECK(e.error() == zip::ZipError::DeclaredSizeMismatch);
    }
    ISO_CHECK(threw_via_unzip);
}

// ── Extra: oversized writer input rejected, not silently truncated ───────

static void test_writer_name_too_long_rejected() {
    std::string huge_name(70000, 'x');  // > 65535, the ZIP (non-ZIP64) name-length limit
    zip::ZipWriter w;
    bool threw = false;
    try {
        w.add_file(huge_name, bytes("hi"), true);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::NameTooLong);
    }
    ISO_CHECK(threw);
}

// ── Extra: writer rejects more than 65535 entries (security-review
//    regression test — the EOCD entry-count fields are 16 bits wide, so a
//    wrapped-to-0 declared count on a real 65536+-entry archive would
//    mislead any consumer that trusts it as a fast-path total) ────────────

static void test_writer_too_many_entries_rejected() {
    zip::ZipWriter w;
    // Directory entries are cheap (no compression attempted, no data), so
    // this stays fast even at 65536 entries.
    for (int i = 0; i < 65536; ++i) {
        w.add_directory("d" + std::to_string(i) + "/");
    }
    bool threw = false;
    try {
        (void)w.finish();
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::TooManyEntries);
    }
    ISO_CHECK(threw);
}

// ── Extra: an out-of-range local_offset is rejected cleanly ─────────────
//
// General robustness check alongside the security-review fix that widened
// every `entry.local_offset`-derived offset computation in `ZipReader::read`
// to `uint64_t` before adding to it (closing a theoretical 32-bit `size_t`
// wraparound in the offset arithmetic itself, not just in `read_u16`/
// `read_u32`'s own internal bounds check). This can't observe the
// wraparound directly on a 64-bit test host, but it does confirm the
// resulting behavior for an extreme `local_offset` is exactly what it
// should be: a clean rejection, not a crash or a wrong-but-successful read.
static void test_extreme_local_offset_rejected() {
    zip::ZipWriter w;
    w.add_file("f.txt", bytes("hello"), false);
    Bytes archive = w.finish();
    Bytes corrupted = archive;

    std::size_t cd_start = find_cd_signature(corrupted);
    ISO_CHECK(cd_start != static_cast<std::size_t>(-1));
    // Relative_Offset_Of_Local_Header is at CD offset +42 (4-byte LE).
    corrupted[cd_start + 42] = 0xF0;
    corrupted[cd_start + 43] = 0xFF;
    corrupted[cd_start + 44] = 0xFF;
    corrupted[cd_start + 45] = 0xFF;  // local_offset ~= 0xFFFFFFF0

    zip::ZipReader reader(corrupted);
    bool threw = false;
    try {
        (void)reader.read(reader.entries()[0]);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::LocalHeaderOutOfBounds);
    }
    ISO_CHECK(threw);
}

// ── Extra: aggregate uncompressed-size budget in unzip() ─────────────────

static void test_aggregate_budget_exceeded() {
    Bytes data = repeat(bytes("0123456789"), 100);  // 1000 bytes, compresses well
    Bytes archive = zip::zip({{"a.bin", data}, {"b.bin", data}});

    // A budget smaller than a single entry's uncompressed size must throw,
    // even though the ARCHIVE itself is tiny (DEFLATE makes it small) — the
    // budget is checked against the DECLARED uncompressed size, not the
    // compressed bytes on disk.
    bool threw = false;
    try {
        (void)zip::unzip(archive, 500);
    } catch (const zip::ZipException& e) {
        threw = true;
        ISO_CHECK(e.error() == zip::ZipError::UncompressedSizeBudgetExceeded);
    }
    ISO_CHECK(threw);

    // A budget that covers both entries succeeds.
    auto files = zip::unzip(archive, 4096);
    ISO_CHECK_EQ_UINT(files.size(), 2u);
}

// ── Extra: a real dynamic-Huffman ZIP entry (Python zipfile output) ──────
//
// Byte-for-byte the same fixture used by code/packages/rust/zip/src/lib.rs's
// `test_read_dynamic_huffman_entry` — a ZIP produced by CPython's `zipfile`
// module (zlib level 9). Its single entry `sheet1.xml` uses a DYNAMIC
// Huffman block (BTYPE=10), which is what virtually every real-world
// producer (Microsoft Office, `zip`(1), Java's jar) emits, unlike this
// package's own writer, which only needs fixed-Huffman blocks for these
// small test fixtures. Reading it end-to-end (through `ca::deflate::inflate`
// and the CRC-32 check) is the load-bearing proof that `cpp/zip` can open
// files people actually have — the exact gap that made `dart/deflate`
// unusable for `dart/zip` (see lessons.md).
static const Bytes kDynamicHuffmanZip = {
    0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00, 0x08, 0x00, 0x90, 0x88, 0xe2, 0x5c, 0x50, 0x87,
    0x66, 0x1d, 0x7f, 0x00, 0x00, 0x00, 0xdc, 0x05, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x73, 0x68,
    0x65, 0x65, 0x74, 0x31, 0x2e, 0x78, 0x6d, 0x6c, 0xed, 0xcd, 0xb1, 0x0a, 0xc2, 0x30, 0x14, 0x85,
    0xe1, 0x57, 0x39, 0xa3, 0x2e, 0x25, 0xcd, 0xa8, 0x74, 0x08, 0x58, 0x41, 0x68, 0xa9, 0x10, 0x05,
    0x71, 0xbb, 0xb4, 0xb7, 0x18, 0x08, 0x69, 0xb8, 0x89, 0xfa, 0xfa, 0x16, 0x17, 0x9f, 0xc0, 0x2d,
    0xeb, 0xcf, 0xe1, 0x7c, 0x36, 0x0a, 0xd3, 0x94, 0x1e, 0xcc, 0xb9, 0xef, 0x30, 0xb2, 0xf7, 0x30,
    0xf5, 0x0e, 0xc2, 0x2f, 0x0e, 0x4f, 0x6e, 0x6a, 0xa5, 0xd4, 0x1e, 0x46, 0xff, 0x8a, 0xfe, 0x96,
    0xbc, 0x64, 0xf2, 0x8d, 0xbd, 0xf6, 0x9b, 0x75, 0x6d, 0xf4, 0xb6, 0xc2, 0x30, 0xcf, 0x6e, 0x64,
    0x0c, 0x91, 0x03, 0x6e, 0xeb, 0x55, 0x24, 0xc9, 0x09, 0x24, 0x0c, 0xa1, 0x37, 0x0e, 0xed, 0xb1,
    0x33, 0x97, 0x16, 0x2e, 0x24, 0x37, 0x31, 0x08, 0xf7, 0xd3, 0xb9, 0x82, 0x2d, 0x78, 0xc1, 0x0b,
    0x5e, 0xf0, 0x82, 0xff, 0x03, 0xff, 0x00, 0x50, 0x4b, 0x01, 0x02, 0x14, 0x03, 0x14, 0x00, 0x00,
    0x00, 0x08, 0x00, 0x90, 0x88, 0xe2, 0x5c, 0x50, 0x87, 0x66, 0x1d, 0x7f, 0x00, 0x00, 0x00, 0xdc,
    0x05, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x73, 0x68, 0x65, 0x65, 0x74, 0x31, 0x2e, 0x78, 0x6d, 0x6c, 0x50,
    0x4b, 0x05, 0x06, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x38, 0x00, 0x00, 0x00, 0xa7,
    0x00, 0x00, 0x00, 0x00, 0x00,
};

static void test_real_dynamic_huffman_entry() {
    std::string expected_str =
        "SpreadsheetML cell A1: revenue=1000; "
        "A2: revenue=2000; total=SUM(A1:A2). "
        "Office Open XML parts are raw DEFLATE inside a ZIP. ";
    Bytes expected;
    for (int i = 0; i < 12; ++i) {
        expected.insert(expected.end(), expected_str.begin(), expected_str.end());
    }

    auto files = zip::unzip(kDynamicHuffmanZip);
    ISO_CHECK_EQ_UINT(files.size(), 1u);
    ISO_CHECK(files[0].first == "sheet1.xml");
    ISO_CHECK(files[0].second == expected);

    zip::ZipReader reader(kDynamicHuffmanZip);
    ISO_CHECK(reader.read_by_name("sheet1.xml") == expected);
    ISO_CHECK_EQ_UINT(reader.entries()[0].method, 8u);
}

int main() {
    test_crc32();
    test_dos_datetime();

    test_tc1_stored_roundtrip();
    test_tc2_deflate_roundtrip();
    test_tc3_multiple_files();
    test_tc4_directory_entry();
    test_tc5_crc_mismatch_detected();
    test_tc6_random_access();
    test_tc7_incompressible_stored();
    test_tc8_empty_file();
    test_tc9_large_file_compressed();
    test_tc10_cli_interop();
    test_tc11_unicode_filename();
    test_tc12_nested_paths();

    test_empty_archive();
    test_read_by_name();
    test_no_eocd();
    test_encrypted_entry_rejected();
    test_unsupported_method_rejected();
    test_declared_size_mismatch_rejected();
    test_writer_name_too_long_rejected();
    test_writer_too_many_entries_rejected();
    test_extreme_local_offset_rejected();
    test_aggregate_budget_exceeded();
    test_real_dynamic_huffman_entry();

    return ISO_TEST_RESULT();
}
