/*
 * zip_test.c — tests for zip.h/zip.c, covering every TC-1..TC-12 from
 * code/specs/CMP09-zip.md plus targeted robustness/security checks for the
 * malformed-input handling documented in zip.h ("Security").
 */
#include "zip.h"

#include "iso_test.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---- small test helpers ------------------------------------------------ */

static unsigned char *repeat_bytes(const unsigned char *src, size_t src_len,
                                   size_t times, size_t *out_len) {
    size_t total = src_len * times;
    unsigned char *buf = (unsigned char *)malloc(total > 0 ? total : 1);
    size_t i;
    for (i = 0; i < times; i++) {
        memcpy(buf + i * src_len, src, src_len);
    }
    *out_len = total;
    return buf;
}

/* Deterministic pseudo-random bytes (LCG) — poorly compressible, used to
 * exercise the Stored fallback (TC-7) without relying on a real RNG. */
static unsigned char *lcg_bytes(size_t n) {
    unsigned char *buf = (unsigned char *)malloc(n > 0 ? n : 1);
    uint32_t seed = 42u;
    size_t i;
    for (i = 0; i < n; i++) {
        seed = seed * 1664525u + 1013904223u;
        buf[i] = (unsigned char)(seed >> 24);
    }
    return buf;
}

static unsigned char *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    long sz;
    unsigned char *buf;
    *out_len = 0;
    if (!f) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    sz = ftell(f);
    if (sz < 0) {
        fclose(f);
        return NULL;
    }
    if (fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return NULL;
    }
    buf = (unsigned char *)malloc((size_t)sz > 0 ? (size_t)sz : 1);
    if (sz > 0 && fread(buf, 1, (size_t)sz, f) != (size_t)sz) {
        fclose(f);
        free(buf);
        return NULL;
    }
    fclose(f);
    *out_len = (size_t)sz;
    return buf;
}

static int write_file(const char *path, const unsigned char *data, size_t len) {
    FILE *f = fopen(path, "wb");
    if (!f) {
        return 0;
    }
    if (len > 0 && fwrite(data, 1, len, f) != len) {
        fclose(f);
        return 0;
    }
    fclose(f);
    return 1;
}

/* ---- CRC-32 -------------------------------------------------------------- */

static void test_crc32_known_values(void) {
    /* Standard CRC-32 check value for the ASCII digits "123456789". */
    ISO_CHECK_EQ_UINT(zip_crc32((const unsigned char *)"123456789", 9, 0),
                      0xCBF43926ul);
    /* Cross-checked against Python's binascii.crc32() / the sibling
     * rust/zip crate's own test. */
    ISO_CHECK_EQ_UINT(
        zip_crc32((const unsigned char *)"hello world", 11, 0), 0x0D4A1185ul);
    ISO_CHECK_EQ_UINT(zip_crc32(NULL, 0, 0), 0x00000000ul);
}

static void test_crc32_incremental(void) {
    uint32_t full = zip_crc32((const unsigned char *)"hello world", 11, 0);
    uint32_t part1 = zip_crc32((const unsigned char *)"hello ", 6, 0);
    uint32_t part2 = zip_crc32((const unsigned char *)"world", 5, part1);
    ISO_CHECK_EQ_UINT(part2, full);
}

/* ---- MS-DOS date/time ----------------------------------------------------- */

static void test_dos_datetime_epoch(void) {
    uint32_t dt = zip_dos_datetime(1980, 1, 1, 0, 0, 0);
    ISO_CHECK_EQ_UINT(dt, (unsigned long)ZIP_DOS_EPOCH);
    ISO_CHECK_EQ_UINT(dt >> 16, 33ul); /* date field: (0<<9)|(1<<5)|1 */
    ISO_CHECK_EQ_UINT(dt & 0xFFFFu, 0ul);
}

/* ---- TC-1: round-trip single file (Stored) -------------------------------- */

static void test_tc1_stored_roundtrip(void) {
    const unsigned char data[] = "hello, world";
    size_t data_len = sizeof(data) - 1;
    ZipWriter *w = NULL;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;

    ISO_CHECK(zip_writer_new(&w) == ZIP_OK);
    ISO_CHECK(zip_writer_add_file(w, "hello.txt", data, data_len, 0) == ZIP_OK);
    ISO_CHECK(zip_writer_finish(w, &archive, &archive_len) == ZIP_OK);
    zip_writer_free(w);
    ISO_CHECK(archive != NULL);

    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 1ul);
    if (count == 1) {
        ISO_CHECK_STR_EQ(files[0].name, "hello.txt");
        ISO_CHECK_EQ_UINT(files[0].len, data_len);
        ISO_CHECK_MEM_EQ(files[0].data, data, data_len);
    }
    zip_files_free(files, count);
    free(archive);
}

/* ---- TC-2: round-trip single file (DEFLATE) ------------------------------- */

static void test_tc2_deflate_roundtrip(void) {
    const unsigned char base[] = "the quick brown fox jumps over the lazy dog ";
    size_t text_len;
    unsigned char *text = repeat_bytes(base, sizeof(base) - 1, 10, &text_len);
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;

    entry.name = (char *)"text.txt";
    entry.name_len = strlen("text.txt");
    entry.data = text;
    entry.len = text_len;

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 1ul);
    if (count == 1) {
        ISO_CHECK_STR_EQ(files[0].name, "text.txt");
        ISO_CHECK_EQ_UINT(files[0].len, text_len);
        ISO_CHECK_MEM_EQ(files[0].data, text, text_len);
    }

    zip_files_free(files, count);
    free(archive);
    free(text);
}

/* ---- TC-3: multiple files -------------------------------------------------- */

static void test_tc3_multiple_files(void) {
    unsigned char all_bytes[256];
    ZipFile entries[3];
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;
    size_t i;

    for (i = 0; i < 256; i++) {
        all_bytes[i] = (unsigned char)i;
    }

    entries[0].name = (char *)"a.txt";
    entries[0].data = (unsigned char *)"file A content";
    entries[0].len = strlen("file A content");
    entries[1].name = (char *)"b.txt";
    entries[1].data = (unsigned char *)"file B content";
    entries[1].len = strlen("file B content");
    entries[2].name = (char *)"c.bin";
    entries[2].data = all_bytes;
    entries[2].len = 256;

    ISO_CHECK(zip_bytes(entries, 3, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 3ul);

    for (i = 0; i < 3; i++) {
        size_t j;
        int found = 0;
        for (j = 0; j < count; j++) {
            if (strcmp(files[j].name, entries[i].name) == 0) {
                found = 1;
                ISO_CHECK_EQ_UINT(files[j].len, entries[i].len);
                ISO_CHECK_MEM_EQ(files[j].data, entries[i].data, entries[i].len);
                break;
            }
        }
        ISO_CHECK_MSG(found, "entry missing from unzip() result");
    }

    zip_files_free(files, count);
    free(archive);
}

/* ---- TC-4: directory entry -------------------------------------------------- */

static void test_tc4_directory_entry(void) {
    ZipWriter *w = NULL;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    size_t i, count;
    int saw_dir = 0, saw_file = 0;
    const ZipEntry *dir_entry = NULL;

    ISO_CHECK(zip_writer_new(&w) == ZIP_OK);
    ISO_CHECK(zip_writer_add_directory(w, "mydir/") == ZIP_OK);
    ISO_CHECK(zip_writer_add_file(w, "mydir/file.txt",
                                  (const unsigned char *)"contents", 8, 1) ==
             ZIP_OK);
    ISO_CHECK(zip_writer_finish(w, &archive, &archive_len) == ZIP_OK);
    zip_writer_free(w);

    ISO_CHECK(zip_reader_new(archive, archive_len, &r) == ZIP_OK);
    count = zip_reader_entry_count(r);
    ISO_CHECK_EQ_UINT(count, 2ul);
    for (i = 0; i < count; i++) {
        const ZipEntry *e = zip_reader_entry(r, i);
        if (strcmp(e->name, "mydir/") == 0) {
            saw_dir = 1;
            dir_entry = e;
        } else if (strcmp(e->name, "mydir/file.txt") == 0) {
            saw_file = 1;
        }
    }
    ISO_CHECK_MSG(saw_dir, "directory entry missing");
    ISO_CHECK_MSG(saw_file, "file inside directory missing");
    if (dir_entry) {
        ISO_CHECK(dir_entry->is_directory);
    }

    zip_reader_free(r);
    free(archive);
}

/* ---- TC-5: CRC-32 verification ---------------------------------------------- */

static void test_tc5_crc_mismatch_detected(void) {
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;
    ZipStatus st;

    entry.name = (char *)"f.txt";
    entry.data = (unsigned char *)"test data";
    entry.len = strlen("test data");

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(archive_len > 35);
    /* Corrupt a data byte directly: 30-byte fixed Local Header + 5-byte name
     * "f.txt" = offset 35 is the first byte of the file data. Flipping it
     * changes the decompressed content without touching the stored CRC, so
     * the reader's CRC-32 check must catch it (matches the reference Rust
     * test's TC-5). */
    archive[35] ^= 0xFF;

    st = zip_unzip(archive, archive_len, &files, &count);
    ISO_CHECK_MSG(st == ZIP_ERR_CRC_MISMATCH,
                 "expected ZIP_ERR_CRC_MISMATCH for corrupted data");
    zip_files_free(files, count);
    free(archive);
}

/* ---- TC-6: EOCD detection / random access ----------------------------------- */

static void test_tc6_random_access(void) {
    ZipFile entries[10];
    char names[10][16];
    char datas[10][16];
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    const ZipEntry *e5 = NULL;
    unsigned char *out = NULL;
    size_t out_len = 0, i, count;

    for (i = 0; i < 10; i++) {
        /* snprintf (C99+) rather than sprintf: no functional difference at
         * these fixed small widths, just the more defensive habit. */
        snprintf(names[i], sizeof(names[i]), "f%u.txt", (unsigned)i);
        snprintf(datas[i], sizeof(datas[i]), "content %u", (unsigned)i);
        entries[i].name = names[i];
        entries[i].data = (unsigned char *)datas[i];
        entries[i].len = strlen(datas[i]);
    }

    ISO_CHECK(zip_bytes(entries, 10, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_reader_new(archive, archive_len, &r) == ZIP_OK);

    count = zip_reader_entry_count(r);
    for (i = 0; i < count; i++) {
        const ZipEntry *e = zip_reader_entry(r, i);
        if (strcmp(e->name, "f5.txt") == 0) {
            e5 = e;
        }
    }
    ISO_CHECK_MSG(e5 != NULL, "f5.txt not found among entries");
    if (e5) {
        /* zip_reader_read returns a raw (data, len) pair, NOT a NUL-terminated
         * C string -- compare with the known length via ISO_CHECK_MEM_EQ, not
         * strcmp/ISO_CHECK_STR_EQ (which would read one byte past the
         * allocation on the exact-length buffer). */
        ISO_CHECK(zip_reader_read(r, e5, &out, &out_len) == ZIP_OK);
        ISO_CHECK_EQ_UINT(out_len, strlen("content 5"));
        ISO_CHECK_MEM_EQ(out, "content 5", strlen("content 5"));
        free(out);
    }

    zip_reader_free(r);
    free(archive);
}

/* ---- TC-7: incompressible data stored without compression -------------------- */

static void test_tc7_incompressible_stored(void) {
    unsigned char *data = lcg_bytes(1024);
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    const ZipEntry *e;
    unsigned char *out = NULL;
    size_t out_len = 0;

    entry.name = (char *)"random.bin";
    entry.data = data;
    entry.len = 1024;

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_reader_new(archive, archive_len, &r) == ZIP_OK);
    ISO_CHECK_EQ_UINT(zip_reader_entry_count(r), 1ul);
    e = zip_reader_entry(r, 0);
    ISO_CHECK(e != NULL);
    if (e) {
        ISO_CHECK_EQ_UINT(e->method, 0ul); /* Stored: DEFLATE would not help */
        ISO_CHECK(zip_reader_read(r, e, &out, &out_len) == ZIP_OK);
        ISO_CHECK_EQ_UINT(out_len, 1024ul);
        ISO_CHECK_MEM_EQ(out, data, 1024);
        free(out);
    }

    zip_reader_free(r);
    free(archive);
    free(data);
}

/* ---- TC-8: empty file ---------------------------------------------------------- */

static void test_tc8_empty_file(void) {
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;

    entry.name = (char *)"empty.txt";
    entry.data = NULL;
    entry.len = 0;

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 1ul);
    if (count == 1) {
        ISO_CHECK_STR_EQ(files[0].name, "empty.txt");
        ISO_CHECK_EQ_UINT(files[0].len, 0ul);
    }

    zip_files_free(files, count);
    free(archive);
}

/* ---- TC-9: large file, multi-block DEFLATE compresses ------------------------- */

static void test_tc9_large_file_compressed(void) {
    size_t data_len;
    unsigned char *data =
        repeat_bytes((const unsigned char *)"abcdefghij", 10, 10000, &data_len);
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;

    entry.name = (char *)"big.bin";
    entry.data = data;
    entry.len = data_len;

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 1ul);
    if (count == 1) {
        ISO_CHECK_EQ_UINT(files[0].len, data_len);
        ISO_CHECK_MEM_EQ(files[0].data, data, data_len);
    }
    ISO_CHECK_MSG(archive_len < data_len,
                 "repetitive 100KB input must compress smaller than itself");

    zip_files_free(files, count);
    free(archive);
    free(data);
}

/* ---- TC-10: CLI interop with the system zip/unzip tools ----------------------
 *
 * Manual/subprocess-based, per the spec ("every other port documents this as
 * manual/subprocess-based, match that" — see the dart/zip and rust/zip
 * precedent noted in the task brief). This package shells out via the
 * standard ISO C `system()` (declared in <stdlib.h>) rather than POSIX
 * `popen`/`fork`+`exec`, so the pure-ISO-C conformance the iso-harness
 * enforces under -pedantic-errors holds on every platform, including MSVC.
 * `system()`'s exact return-value encoding is implementation-defined, but a
 * 0 result reliably means "the shell found and ran the command" on every
 * platform this harness targets (POSIX sh and cmd.exe both surface a
 * nonzero/nonexistent-command status otherwise) — good enough to detect
 * whether Info-ZIP's `zip`/`unzip` are on PATH and skip gracefully if not,
 * the same pattern the dart/zip port uses for the same reason.
 */

static int tool_available(const char *probe_cmd) {
    return system(probe_cmd) == 0;
}

static void test_tc10_cli_interop(void) {
    const unsigned char src_a[] = "Hello from the c/zip CLI interop test!\n";
    const unsigned char src_b[] = "\x00\x01\x02\x03\xFF\xFE\xFD binary payload";
    size_t len_a = sizeof(src_a) - 1;
    size_t len_b = sizeof(src_b) - 1;

    if (!tool_available("zip -v > _build/tc10_probe.out 2>&1") ||
        !tool_available("unzip -v > _build/tc10_probe.out 2>&1")) {
        printf("  SKIP TC-10: system zip/unzip not found on PATH\n");
        remove("_build/tc10_probe.out");
        return;
    }
    remove("_build/tc10_probe.out");

    /* Direction 1: write with our library, extract with the system unzip. */
    {
        ZipFile entries[2];
        unsigned char *archive = NULL;
        size_t archive_len = 0;
        unsigned char *extracted_a = NULL;
        unsigned char *extracted_b = NULL;
        size_t elen_a = 0, elen_b = 0;

        entries[0].name = (char *)"tc10_a.txt";
        entries[0].data = (unsigned char *)src_a;
        entries[0].len = len_a;
        entries[1].name = (char *)"tc10_b.bin";
        entries[1].data = (unsigned char *)src_b;
        entries[1].len = len_b;

        ISO_CHECK(zip_bytes(entries, 2, &archive, &archive_len) == ZIP_OK);
        ISO_CHECK(write_file("_build/tc10_from_us.zip", archive, archive_len));
        free(archive);

        system("unzip -o -q _build/tc10_from_us.zip -d _build/tc10_extract"
              " > _build/tc10_probe.out 2>&1");

        extracted_a = read_file("_build/tc10_extract/tc10_a.txt", &elen_a);
        extracted_b = read_file("_build/tc10_extract/tc10_b.bin", &elen_b);
        ISO_CHECK_MSG(extracted_a != NULL,
                     "system unzip failed to extract tc10_a.txt");
        ISO_CHECK_MSG(extracted_b != NULL,
                     "system unzip failed to extract tc10_b.bin");
        if (extracted_a) {
            ISO_CHECK_EQ_UINT(elen_a, len_a);
            ISO_CHECK_MEM_EQ(extracted_a, src_a, len_a);
        }
        if (extracted_b) {
            ISO_CHECK_EQ_UINT(elen_b, len_b);
            ISO_CHECK_MEM_EQ(extracted_b, src_b, len_b);
        }
        free(extracted_a);
        free(extracted_b);
    }

    /* Direction 2: write with the system zip, read with our library. */
    {
        unsigned char *archive_bytes = NULL;
        size_t archive_len = 0;
        ZipFile *files = NULL;
        size_t count = 0, i;
        int found_a = 0, found_b = 0;

        ISO_CHECK(write_file("_build/tc10_src_a.txt", src_a, len_a));
        ISO_CHECK(write_file("_build/tc10_src_b.bin", src_b, len_b));
        remove("_build/tc10_from_system.zip");

        system("zip -q -j _build/tc10_from_system.zip "
              "_build/tc10_src_a.txt _build/tc10_src_b.bin"
              " > _build/tc10_probe.out 2>&1");

        archive_bytes = read_file("_build/tc10_from_system.zip", &archive_len);
        ISO_CHECK_MSG(archive_bytes != NULL,
                     "system zip failed to produce an archive");
        if (archive_bytes) {
            ISO_CHECK(zip_unzip(archive_bytes, archive_len, &files, &count) ==
                     ZIP_OK);
            for (i = 0; i < count; i++) {
                if (strcmp(files[i].name, "tc10_src_a.txt") == 0) {
                    found_a = 1;
                    ISO_CHECK_EQ_UINT(files[i].len, len_a);
                    ISO_CHECK_MEM_EQ(files[i].data, src_a, len_a);
                } else if (strcmp(files[i].name, "tc10_src_b.bin") == 0) {
                    found_b = 1;
                    ISO_CHECK_EQ_UINT(files[i].len, len_b);
                    ISO_CHECK_MEM_EQ(files[i].data, src_b, len_b);
                }
            }
            ISO_CHECK_MSG(found_a, "tc10_src_a.txt missing from system zip output");
            ISO_CHECK_MSG(found_b, "tc10_src_b.bin missing from system zip output");
            zip_files_free(files, count);
        }
        free(archive_bytes);
    }

    remove("_build/tc10_probe.out");
    remove("_build/tc10_from_us.zip");
    remove("_build/tc10_from_system.zip");
    remove("_build/tc10_src_a.txt");
    remove("_build/tc10_src_b.bin");
    remove("_build/tc10_extract/tc10_a.txt");
    remove("_build/tc10_extract/tc10_b.bin");
}

/* ---- TC-11: Unicode filename ------------------------------------------------- */

static void test_tc11_unicode_filename(void) {
    /* UTF-8 bytes for "\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E/r\xC3\xA9sum\xC3\xA9.txt"
     * i.e. (Japanese "nihongo")/resume.txt with an accented e -- written as a
     * raw UTF-8 byte literal so the source file itself stays plain ASCII. */
    const char *name =
        "\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E/r\xC3\xA9sum\xC3\xA9.txt";
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;

    entry.name = (char *)name;
    entry.data = (unsigned char *)"content";
    entry.len = strlen("content");

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 1ul);
    if (count == 1) {
        ISO_CHECK_STR_EQ(files[0].name, name);
        ISO_CHECK_MEM_EQ(files[0].data, "content", 7);
    }

    zip_files_free(files, count);
    free(archive);
}

/* ---- TC-12: nested paths --------------------------------------------------- */

static void test_tc12_nested_paths(void) {
    ZipFile entries[3];
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipFile *files = NULL;
    size_t count = 0;
    size_t i;

    entries[0].name = (char *)"root.txt";
    entries[0].data = (unsigned char *)"root";
    entries[0].len = 4;
    entries[1].name = (char *)"dir/file.txt";
    entries[1].data = (unsigned char *)"nested";
    entries[1].len = 6;
    entries[2].name = (char *)"dir/sub/deep.txt";
    entries[2].data = (unsigned char *)"deep";
    entries[2].len = 4;

    ISO_CHECK(zip_bytes(entries, 3, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_unzip(archive, archive_len, &files, &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 3ul);

    for (i = 0; i < 3; i++) {
        size_t j;
        int found = 0;
        for (j = 0; j < count; j++) {
            if (strcmp(files[j].name, entries[i].name) == 0) {
                found = 1;
                ISO_CHECK_MEM_EQ(files[j].data, entries[i].data, entries[i].len);
                break;
            }
        }
        ISO_CHECK_MSG(found, "nested entry missing");
    }

    zip_files_free(files, count);
    free(archive);
}

/* ---- read_by_name ----------------------------------------------------------- */

static void test_read_by_name(void) {
    ZipFile entries[2];
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    unsigned char *out = NULL;
    size_t out_len = 0;

    entries[0].name = (char *)"alpha.txt";
    entries[0].data = (unsigned char *)"AAA";
    entries[0].len = 3;
    entries[1].name = (char *)"beta.txt";
    entries[1].data = (unsigned char *)"BBB";
    entries[1].len = 3;

    ISO_CHECK(zip_bytes(entries, 2, &archive, &archive_len) == ZIP_OK);
    ISO_CHECK(zip_reader_new(archive, archive_len, &r) == ZIP_OK);

    ISO_CHECK(zip_reader_read_by_name(r, "beta.txt", &out, &out_len) == ZIP_OK);
    ISO_CHECK_MEM_EQ(out, "BBB", 3);
    free(out);
    out = NULL;

    ISO_CHECK(zip_reader_read_by_name(r, "nope.txt", &out, &out_len) ==
             ZIP_ERR_NOT_FOUND);
    ISO_CHECK(out == NULL);

    zip_reader_free(r);
    free(archive);
}

/* ---- real-world dynamic-Huffman entry ----------------------------------------
 *
 * A ZIP produced by Python's `zipfile` (zlib level 9); its single entry
 * `sheet1.xml` uses a DYNAMIC Huffman DEFLATE block (BTYPE=10) -- like
 * virtually every real-world producer (Microsoft Office, `zip`(1), Java),
 * NOT the fixed-Huffman-only blocks a minimal writer might emit. This is the
 * load-bearing proof that c/deflate's dynamic-Huffman decode path is
 * correctly exercised through c/zip, not just through c/deflate's own tests.
 * Identical fixture bytes to the one already verified in
 * code/packages/rust/zip/src/lib.rs (same archive, same expected payload).
 */
static const unsigned char DYNAMIC_HUFFMAN_ZIP[] = {
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
    0x00, 0x00, 0x00, 0x00, 0x00
};

static void test_read_dynamic_huffman_entry(void) {
    const char *expected_line =
        "SpreadsheetML cell A1: revenue=1000; "
        "A2: revenue=2000; total=SUM(A1:A2). "
        "Office Open XML parts are raw DEFLATE inside a ZIP. ";
    size_t line_len = strlen(expected_line);
    size_t expected_len;
    unsigned char *expected = repeat_bytes((const unsigned char *)expected_line,
                                           line_len, 12, &expected_len);
    ZipFile *files = NULL;
    size_t count = 0;
    ZipReader *r = NULL;
    unsigned char *out = NULL;
    size_t out_len = 0;

    ISO_CHECK(zip_unzip(DYNAMIC_HUFFMAN_ZIP, sizeof(DYNAMIC_HUFFMAN_ZIP), &files,
                        &count) == ZIP_OK);
    ISO_CHECK_EQ_UINT(count, 1ul);
    if (count == 1) {
        ISO_CHECK_STR_EQ(files[0].name, "sheet1.xml");
        ISO_CHECK_EQ_UINT(files[0].len, expected_len);
        ISO_CHECK_MEM_EQ(files[0].data, expected, expected_len);
    }
    zip_files_free(files, count);

    /* And via the low-level reader, exercising the CRC-32 check too. */
    ISO_CHECK(zip_reader_new(DYNAMIC_HUFFMAN_ZIP, sizeof(DYNAMIC_HUFFMAN_ZIP),
                             &r) == ZIP_OK);
    ISO_CHECK(zip_reader_read_by_name(r, "sheet1.xml", &out, &out_len) == ZIP_OK);
    ISO_CHECK_EQ_UINT(out_len, expected_len);
    ISO_CHECK_MEM_EQ(out, expected, expected_len);
    free(out);
    zip_reader_free(r);

    free(expected);
}

/* ---- robustness / security: malformed archives ------------------------------ */

static void test_malformed_too_short(void) {
    unsigned char tiny[5] = {0x50, 0x4b, 0x03, 0x04, 0x00};
    ZipReader *r = NULL;
    ISO_CHECK(zip_reader_new(tiny, sizeof(tiny), &r) == ZIP_ERR_MALFORMED);
    ISO_CHECK(r == NULL);
}

static void test_malformed_no_eocd(void) {
    unsigned char garbage[128];
    ZipReader *r = NULL;
    size_t i;
    for (i = 0; i < sizeof(garbage); i++) {
        garbage[i] = (unsigned char)(i * 7u);
    }
    ISO_CHECK(zip_reader_new(garbage, sizeof(garbage), &r) == ZIP_ERR_MALFORMED);
    ISO_CHECK(r == NULL);
}

/* Craft a bare 22-byte EOCD (no Central Directory before it) whose cd_offset
 * and cd_size fields are both close to UINT32_MAX. This is exactly the
 * "untrusted 32-bit offset/size" case from zip.h's Security section: the sum
 * cd_offset + cd_size must be computed in a wider-than-size_t intermediate
 * so it cannot wrap around and slip past the bounds check on a 32-bit
 * size_t platform. Either way, this tiny 22-byte buffer obviously cannot
 * contain a Central Directory of that declared size, so this must be
 * rejected as malformed rather than read out of bounds. */
static void test_malformed_cd_offset_overflow(void) {
    unsigned char eocd[22];
    ZipReader *r = NULL;
    uint32_t huge = 0xFFFFFFF0u;

    memset(eocd, 0, sizeof(eocd));
    eocd[0] = 0x50; eocd[1] = 0x4b; eocd[2] = 0x05; eocd[3] = 0x06; /* sig */
    /* disk_num, cd_disk, entries_this_disk, entries_total left at 0.
     * cd_size (offset 12) and cd_offset (offset 16) are filled byte-by-byte
     * below, independent of host endianness. */
    eocd[12] = (unsigned char)(huge & 0xFFu);
    eocd[13] = (unsigned char)((huge >> 8) & 0xFFu);
    eocd[14] = (unsigned char)((huge >> 16) & 0xFFu);
    eocd[15] = (unsigned char)((huge >> 24) & 0xFFu); /* cd_size */
    eocd[16] = (unsigned char)(huge & 0xFFu);
    eocd[17] = (unsigned char)((huge >> 8) & 0xFFu);
    eocd[18] = (unsigned char)((huge >> 16) & 0xFFu);
    eocd[19] = (unsigned char)((huge >> 24) & 0xFFu); /* cd_offset */
    /* comment_len (bytes 20-21) left at 0, matching this exact 22-byte buffer */

    ISO_CHECK(zip_reader_new(eocd, sizeof(eocd), &r) == ZIP_ERR_MALFORMED);
    ISO_CHECK(r == NULL);
}

static void test_malformed_unsupported_method(void) {
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    size_t cd_start;
    ZipReader *r = NULL;
    const ZipEntry *e;
    unsigned char *out = NULL;
    size_t out_len = 0;
    ZipStatus st;

    entry.name = (char *)"f.txt";
    entry.data = (unsigned char *)"test data";
    entry.len = strlen("test data");

    /* compress=0 forces Stored, so sizes are exactly known: Local Header
     * (30 bytes) + name ("f.txt", 5 bytes) + data (9 bytes) = 44 bytes, then
     * the Central Directory entry begins immediately (single-entry
     * archive). Patch its method field (offset +10 into the CD header) to
     * an unsupported value. */
    {
        ZipWriter *w = NULL;
        ISO_CHECK(zip_writer_new(&w) == ZIP_OK);
        ISO_CHECK(zip_writer_add_file(w, entry.name, entry.data, entry.len, 0) ==
                 ZIP_OK);
        ISO_CHECK(zip_writer_finish(w, &archive, &archive_len) == ZIP_OK);
        zip_writer_free(w);
    }
    cd_start = 30 + strlen("f.txt") + strlen("test data");
    ISO_CHECK(archive_len > cd_start + 11);
    archive[cd_start + 10] = 99; /* method low byte */
    archive[cd_start + 11] = 0;  /* method high byte */

    ISO_CHECK(zip_reader_new(archive, archive_len, &r) == ZIP_OK);
    e = zip_reader_entry(r, 0);
    ISO_CHECK(e != NULL);
    if (e) {
        ISO_CHECK_EQ_UINT(e->method, 99ul);
        st = zip_reader_read(r, e, &out, &out_len);
        ISO_CHECK_MSG(st == ZIP_ERR_UNSUPPORTED_METHOD,
                     "expected ZIP_ERR_UNSUPPORTED_METHOD");
        ISO_CHECK(out == NULL);
    }

    zip_reader_free(r);
    free(archive);
}

static void test_malformed_encrypted_entry(void) {
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    const ZipEntry *e;
    unsigned char *out = NULL;
    size_t out_len = 0;
    ZipStatus st;

    entry.name = (char *)"f.txt";
    entry.data = (unsigned char *)"test data";
    entry.len = strlen("test data");

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    /* Local Header General Purpose Bit Flag is at offset 6 (2 bytes, LE);
     * set bit 0 (encrypted). */
    ISO_CHECK(archive_len > 6);
    archive[6] |= 0x01u;

    ISO_CHECK(zip_reader_new(archive, archive_len, &r) == ZIP_OK);
    e = zip_reader_entry(r, 0);
    ISO_CHECK(e != NULL);
    if (e) {
        st = zip_reader_read(r, e, &out, &out_len);
        ISO_CHECK_MSG(st == ZIP_ERR_ENCRYPTED, "expected ZIP_ERR_ENCRYPTED");
        ISO_CHECK(out == NULL);
    }

    zip_reader_free(r);
    free(archive);
}

/* ---- aggregate decompression-bomb budget ------------------------------------- */

static void test_aggregate_budget_single_entry(void) {
    unsigned char *data = (unsigned char *)malloc(1000);
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    const ZipEntry *e;
    unsigned char *out = NULL;
    size_t out_len = 0;

    memset(data, 'A', 1000);
    entry.name = (char *)"big.bin";
    entry.data = data;
    entry.len = 1000;

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    /* A budget far smaller than the entry's declared uncompressed size must
     * be rejected up front, without needing to actually decompress. */
    ISO_CHECK(zip_reader_new_with_budget(archive, archive_len, 10, &r) ==
             ZIP_OK);
    e = zip_reader_entry(r, 0);
    ISO_CHECK(e != NULL);
    if (e) {
        ISO_CHECK(zip_reader_read(r, e, &out, &out_len) == ZIP_ERR_TOO_LARGE);
        ISO_CHECK(out == NULL);
    }

    zip_reader_free(r);
    free(archive);
    free(data);
}

static void test_aggregate_budget_many_small_entries(void) {
    /* Three entries, ~100 bytes each (Stored, so declared size == actual
     * size exactly), against a budget of 250: the first two reads (200
     * bytes total) must succeed, the third (which would push the running
     * total to 300) must be rejected -- proving the budget is tracked
     * ACROSS calls on the same reader, not just checked per entry. */
    unsigned char chunk[100];
    char name0[] = "e0.bin";
    char name1[] = "e1.bin";
    char name2[] = "e2.bin";
    ZipFile entries[3];
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    ZipReader *r = NULL;
    unsigned char *out = NULL;
    size_t out_len = 0;
    size_t i;

    memset(chunk, 'x', sizeof(chunk));
    entries[0].name = name0;
    entries[0].data = chunk;
    entries[0].len = sizeof(chunk);
    entries[1].name = name1;
    entries[1].data = chunk;
    entries[1].len = sizeof(chunk);
    entries[2].name = name2;
    entries[2].data = chunk;
    entries[2].len = sizeof(chunk);

    {
        ZipWriter *w = NULL;
        ISO_CHECK(zip_writer_new(&w) == ZIP_OK);
        for (i = 0; i < 3; i++) {
            ISO_CHECK(zip_writer_add_file(w, entries[i].name, entries[i].data,
                                          entries[i].len, 0) == ZIP_OK);
        }
        ISO_CHECK(zip_writer_finish(w, &archive, &archive_len) == ZIP_OK);
        zip_writer_free(w);
    }

    ISO_CHECK(zip_reader_new_with_budget(archive, archive_len, 250, &r) ==
             ZIP_OK);
    ISO_CHECK_EQ_UINT(zip_reader_entry_count(r), 3ul);

    for (i = 0; i < 2; i++) {
        const ZipEntry *e = zip_reader_entry(r, i);
        ISO_CHECK(e != NULL);
        if (e) {
            ISO_CHECK(zip_reader_read(r, e, &out, &out_len) == ZIP_OK);
            free(out);
            out = NULL;
        }
    }
    {
        const ZipEntry *e = zip_reader_entry(r, 2);
        ISO_CHECK(e != NULL);
        if (e) {
            ISO_CHECK(zip_reader_read(r, e, &out, &out_len) ==
                     ZIP_ERR_TOO_LARGE);
            ISO_CHECK(out == NULL);
        }
    }

    zip_reader_free(r);
    free(archive);
}

/* Regression test for a security-review finding: the aggregate budget must
 * be charged against the REAL decompressed size, not the declared
 * (attacker-controlled) Central Directory `uncompressed_size` field.
 *
 * Without the fix, a crafted entry could declare a tiny uncompressed_size
 * (with a CRC-32 matching just that many trimmed bytes) while its DEFLATE
 * stream actually inflates to something far larger -- so the budget check
 * (which only looked at the declared size) would never trip, even though
 * real, expensive decompression work happened on every such read. This
 * builds exactly that: a real 50,000-byte compressible payload, written
 * normally (so it round-trips and compresses for real), then the Central
 * Directory copy's uncompressed_size and crc32 fields are patched by hand to
 * describe only the first 4 bytes of that payload -- a "small on paper,
 * large for real" entry. */
static void test_aggregate_budget_rejects_declared_size_lie(void) {
    size_t payload_len;
    unsigned char *payload =
        repeat_bytes((const unsigned char *)"0123456789", 10, 5000, &payload_len);
    ZipFile entry;
    unsigned char *archive = NULL;
    size_t archive_len = 0;
    size_t cd_pos = 0;
    int found_cd = 0;
    size_t i;
    uint32_t lie_size = 4;
    uint32_t lie_crc;
    ZipReader *r = NULL;
    unsigned char *out = NULL;
    size_t out_len = 0;
    const ZipEntry *e;

    entry.name = (char *)"lie.bin";
    entry.data = payload;
    entry.len = payload_len;

    ISO_CHECK(zip_bytes(&entry, 1, &archive, &archive_len) == ZIP_OK);
    /* This payload is highly repetitive, so DEFLATE must have compressed it
     * (method 8) -- if it hadn't, the "declared size lie" below wouldn't
     * demonstrate anything, since Stored data's on-wire length already
     * equals the true decompressed length. */
    ISO_CHECK_MSG(archive_len < payload_len,
                 "test payload must actually compress for this regression"
                 " test to be meaningful");

    /* Find the single Central Directory entry by its signature bytes. */
    for (i = 0; i + 4 <= archive_len; i++) {
        if (archive[i] == 0x50 && archive[i + 1] == 0x4b &&
            archive[i + 2] == 0x01 && archive[i + 3] == 0x02) {
            cd_pos = i;
            found_cd = 1;
            break;
        }
    }
    ISO_CHECK_MSG(found_cd, "Central Directory entry signature not found");
    ISO_CHECK(archive_len > cd_pos + 28);

    /* Patch uncompressed_size (CD offset +24) down to 4 bytes... */
    archive[cd_pos + 24] = (unsigned char)(lie_size & 0xFFu);
    archive[cd_pos + 25] = (unsigned char)((lie_size >> 8) & 0xFFu);
    archive[cd_pos + 26] = (unsigned char)((lie_size >> 16) & 0xFFu);
    archive[cd_pos + 27] = (unsigned char)((lie_size >> 24) & 0xFFu);
    /* ...and crc32 (CD offset +16) to match just those first 4 real bytes,
     * so the post-trim CRC check still passes -- this is what a real
     * attacker crafting the archive by hand would also do. */
    lie_crc = zip_crc32(payload, lie_size, 0);
    archive[cd_pos + 16] = (unsigned char)(lie_crc & 0xFFu);
    archive[cd_pos + 17] = (unsigned char)((lie_crc >> 8) & 0xFFu);
    archive[cd_pos + 18] = (unsigned char)((lie_crc >> 16) & 0xFFu);
    archive[cd_pos + 19] = (unsigned char)((lie_crc >> 24) & 0xFFu);

    /* Budget comfortably above the LIE (4 bytes) but well below the REAL
     * decompressed size (50,000 bytes): must be rejected. */
    ISO_CHECK(zip_reader_new_with_budget(archive, archive_len, 1000, &r) ==
             ZIP_OK);
    e = zip_reader_entry(r, 0);
    ISO_CHECK(e != NULL);
    if (e) {
        ISO_CHECK_EQ_UINT(e->size, lie_size); /* the lie, as parsed */
        ISO_CHECK_MSG(
            zip_reader_read(r, e, &out, &out_len) == ZIP_ERR_TOO_LARGE,
            "a declared-size lie must not bypass the aggregate budget");
        ISO_CHECK(out == NULL);
    }

    zip_reader_free(r);
    free(archive);
    free(payload);
}

/* ---- main ------------------------------------------------------------------- */

int main(void) {
    test_crc32_known_values();
    test_crc32_incremental();
    test_dos_datetime_epoch();

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

    test_read_by_name();
    test_read_dynamic_huffman_entry();

    test_malformed_too_short();
    test_malformed_no_eocd();
    test_malformed_cd_offset_overflow();
    test_malformed_unsupported_method();
    test_malformed_encrypted_entry();

    test_aggregate_budget_single_entry();
    test_aggregate_budget_many_small_entries();
    test_aggregate_budget_rejects_declared_size_lie();

    return ISO_TEST_RESULT();
}
