// Tests for the C++ deflate (CMP05), using the iso_test.h harness. Test
// vectors 1-5 are taken from code/specs/CMP05-deflate.md (verified against
// the Rust `deflate` crate's own tests and Python's `zlib`). The real-world
// blob in section 7 was produced by CPython's `zlib` module itself
// (`zlib.compressobj(9, zlib.DEFLATED, -15)`, i.e. raw DEFLATE, no zlib/gzip
// envelope) so this suite proves `inflate` reads dynamic-Huffman streams it
// did not produce itself — the case that matters for a future `zip` reader.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "deflate.hpp"

namespace deflate = ca::deflate;
using Bytes = std::vector<std::uint8_t>;

static Bytes bytes(const char* s) {
    Bytes b;
    for (const char* p = s; *p; ++p) {
        b.push_back(static_cast<std::uint8_t>(*p));
    }
    return b;
}

// The real-world zlib-produced raw DEFLATE dynamic-Huffman blob (see the
// file header comment) and the plaintext it decodes to.
// LEN_TEXT=624 LEN_DATA=380
static const char* kRealText =
    "In 1996, L. Peter Deutsch documented DEFLATE as RFC 1951, independent of any "
    "implementation. Phil Katz had designed DEFLATE for PKZIP in 1989, combining LZSS-style "
    "back-references with Huffman entropy coding. The same year, Deutsch and Jean-Loup Gailly "
    "published zlib as a portable reference implementation. DEFLATE became the compression layer "
    "inside ZIP, gzip, PNG, and later HTTP content encoding and HPACK header compression. Phil Katz "
    "released the PKZIP specification publicly, which is why DEFLATE became an open standard rather "
    "than a proprietary format confined to one vendor tool chain, unlike many contemporaries. "
    ;

static const Bytes kRealZlibDynamicBlob = {
    0x65, 0x92, 0xcb, 0x6e, 0xdb, 0x30, 0x10, 0x45, 0x7f, 0xe5, 0x7e, 0x00,
    0x6d, 0x20, 0x8b, 0x16, 0xf1, 0x32, 0xc8, 0xcb, 0xa9, 0x8d, 0x42, 0x68,
    0xbc, 0xca, 0x6e, 0x44, 0x8e, 0xcc, 0x41, 0x28, 0x52, 0x20, 0xa9, 0x06,
    0xf2, 0xd7, 0x77, 0xa8, 0xa0, 0x81, 0xd1, 0x6e, 0xb4, 0x19, 0xcd, 0x7d,
    0x9c, 0xe1, 0x4b, 0xc4, 0xcd, 0x6e, 0xf7, 0xdd, 0xe0, 0xb8, 0x45, 0xc7,
    0x95, 0x33, 0x1e, 0x78, 0xae, 0xc5, 0x7a, 0xb8, 0x64, 0xe7, 0x91, 0x63,
    0x65, 0x87, 0x87, 0xc7, 0xa7, 0xe3, 0xdd, 0xe9, 0x11, 0x54, 0xf0, 0xeb,
    0xe9, 0x5e, 0x17, 0xbe, 0xdd, 0x18, 0x48, 0x74, 0x3c, 0xb1, 0x7e, 0x62,
    0x45, 0x1a, 0x40, 0x71, 0x81, 0x8c, 0x53, 0xe0, 0xb6, 0x43, 0x55, 0x52,
    0x54, 0x41, 0x2f, 0x01, 0x07, 0xaa, 0x17, 0x78, 0x72, 0x70, 0x5c, 0xe4,
    0x1c, 0xaf, 0xe4, 0x86, 0x94, 0xd1, 0x1d, 0xde, 0x5e, 0x3a, 0xd5, 0x52,
    0xd1, 0xdb, 0x9d, 0x81, 0x4d, 0x63, 0x2f, 0x51, 0xe2, 0x19, 0xc7, 0xb7,
    0xd7, 0xd7, 0x4d, 0xa9, 0x4b, 0x60, 0xf4, 0x64, 0xdf, 0x37, 0x99, 0x07,
    0xce, 0x1c, 0x2d, 0x17, 0x7c, 0x48, 0xf5, 0xd8, 0xcf, 0xc3, 0x30, 0x52,
    0x84, 0xba, 0xe5, 0x34, 0x2d, 0xba, 0xe9, 0x74, 0x6d, 0x8b, 0x93, 0x67,
    0x14, 0x1a, 0x19, 0x0b, 0x53, 0x36, 0x5f, 0x6d, 0x28, 0x3a, 0xfc, 0x60,
    0x8a, 0x9b, 0x63, 0x9a, 0x27, 0x3c, 0x93, 0x84, 0xb0, 0x60, 0x9a, 0xfb,
    0x20, 0xc5, 0x6b, 0xa4, 0x4b, 0x90, 0xbe, 0xd5, 0x23, 0x4c, 0x29, 0x57,
    0xea, 0xd5, 0xf5, 0xcb, 0xf0, 0xbf, 0x5a, 0x7f, 0xf3, 0xf7, 0x6c, 0x9b,
    0x51, 0x55, 0x47, 0xcd, 0x3d, 0x65, 0x2e, 0x45, 0xe7, 0x08, 0xb4, 0x28,
    0x46, 0x89, 0x45, 0x1c, 0x43, 0xdb, 0x19, 0x9c, 0x2f, 0x32, 0x19, 0x74,
    0x3f, 0x9f, 0xcd, 0x9a, 0x23, 0x50, 0xe3, 0xbc, 0x3f, 0x9d, 0x3a, 0xdd,
    0x53, 0xc0, 0x0a, 0x50, 0x7d, 0xd6, 0xfc, 0xeb, 0x7c, 0xdf, 0xdd, 0xdd,
    0x1f, 0xe0, 0x99, 0x9c, 0xfe, 0x76, 0xa5, 0x7c, 0x0d, 0x34, 0x73, 0x60,
    0x2a, 0x9a, 0xbc, 0xb9, 0x7f, 0x42, 0x2c, 0x13, 0x5b, 0x19, 0xc4, 0xae,
    0x29, 0x3f, 0xbb, 0xd9, 0xb0, 0x18, 0x7c, 0x78, 0x51, 0x00, 0xa2, 0xdc,
    0xfc, 0xf2, 0x6f, 0x76, 0x05, 0x98, 0xf4, 0x88, 0x28, 0x55, 0x8d, 0x29,
    0x3b, 0x64, 0x52, 0xc1, 0xac, 0xaa, 0x3a, 0x51, 0x18, 0x8a, 0x36, 0x0b,
    0x57, 0xca, 0x4b, 0xbb, 0xd6, 0x48, 0xb5, 0x25, 0x1e, 0xa4, 0x5d, 0xb1,
    0x26, 0xa4, 0xc8, 0xf8, 0xad, 0x4f, 0x40, 0xef, 0x58, 0x53, 0x0a, 0xb0,
    0x9e, 0x24, 0x1a, 0xcc, 0x31, 0xc8, 0x3b, 0x63, 0x6c, 0x4f, 0x62, 0x2d,
    0x38, 0x2a, 0x54, 0x52, 0x9d, 0xb2, 0xc5, 0x1f,
};

int main() {
    // ── 1. Empty input (spec test vector 1) ─────────────────────────────
    {
        Bytes want = {0x03, 0x00};
        ISO_CHECK(deflate::compress({}) == want);
        ISO_CHECK(deflate::inflate(want).empty());
        ISO_CHECK(deflate::decompress(deflate::compress({})).empty());
    }

    // ── 2. Literals only — "AAABBC" (spec test vector 2) ─────────────────
    {
        Bytes want = {0x73, 0x74, 0x74, 0x74, 0x72, 0x72, 0x06, 0x00};
        Bytes got = deflate::compress(bytes("AAABBC"));
        ISO_CHECK(got == want);
        ISO_CHECK(deflate::inflate(got) == bytes("AAABBC"));
    }

    // ── 3. With matches — exercise length/distance extra bits and the
    //    overlapping-copy back-reference case (spec test vector 3) ────────
    {
        Bytes a = bytes("AABCBBABC");
        ISO_CHECK(deflate::decompress(deflate::compress(a)) == a);
        Bytes b = bytes("AAAAAAA");  // offset=1, length=6: overlapping copy
        ISO_CHECK(deflate::decompress(deflate::compress(b)) == b);
    }

    // ── 4. Round-trip invariant across a range of inputs, including a
    //    single byte and every byte value 0..255 (spec test vector 4) ─────
    {
        const char* texts[] = {"",
                               "A",
                               "ABCDE",
                               "AAAAAAA",
                               "ABABABAB",
                               "AABCBBABC",
                               "hello world hello world",
                               "the quick brown fox"};
        for (const char* s : texts) {
            Bytes d = bytes(s);
            Bytes c = deflate::compress(d);
            ISO_CHECK(deflate::decompress(c) == d);
            ISO_CHECK(deflate::inflate(c) == d);  // decompress is an alias for inflate
        }

        Bytes all256;
        for (int i = 0; i < 256; ++i) {
            all256.push_back(static_cast<std::uint8_t>(i));
        }
        ISO_CHECK(deflate::decompress(deflate::compress(all256)) == all256);

        Bytes reps;
        for (int i = 0; i < 3000; ++i) {
            reps.push_back(static_cast<std::uint8_t>("ABC"[i % 3]));
        }
        ISO_CHECK(deflate::decompress(deflate::compress(reps)) == reps);
    }

    // ── 5. Repetitive input compresses smaller (spec test vector 5) ──────
    {
        Bytes d(10000, 'A');
        ISO_CHECK(deflate::compress(d).size() < d.size());
    }

    // ── 6. Every produced stream is a single BFINAL=1 block, fixed or
    //    dynamic (never stored, never non-final) ─────────────────────────
    {
        const char* texts[] = {"", "A", "AAABBC", "hello world hello world"};
        for (const char* s : texts) {
            Bytes c = deflate::compress(bytes(s));
            ISO_CHECK(!c.empty());
            if (!c.empty()) {
                std::uint8_t header = static_cast<std::uint8_t>(c[0] & 0b111u);
                ISO_CHECK(header == 0b011u || header == 0b101u);
            }
        }
    }

    // ── 7. Skewed/large varied-vocabulary text picks dynamic Huffman
    //    (BTYPE=10) because it beats fixed on exact bit count, and still
    //    round-trips. (A single perfectly-repeating pattern is a
    //    COUNTEREXAMPLE to "dynamic always wins": it collapses to a couple
    //    of LZSS tokens, so the dynamic header overhead loses to fixed —
    //    exactly why `compress` compares exact bit counts instead of always
    //    picking dynamic. `kRealText` below has a rich, skewed vocabulary
    //    without a single dominating repeat, so dynamic wins here.) ───────
    {
        Bytes d = bytes(kRealText);
        Bytes c = deflate::compress(d);
        ISO_CHECK(!c.empty());
        if (!c.empty()) {
            std::uint8_t btype = static_cast<std::uint8_t>((c[0] >> 1) & 0b11u);
            ISO_CHECK_EQ_UINT(btype, 0b10u);
        }
        ISO_CHECK(deflate::decompress(c) == d);
    }

    // ── 8. Decode a REAL zlib-produced (CPython `zlib.compressobj(9, ...,
    //    -15)`) raw DEFLATE dynamic-Huffman stream — proves `inflate` reads
    //    dynamic Huffman it never produced itself, including the full
    //    32768-byte-window distance/length alphabet real encoders use. ────
    {
        Bytes want = bytes(kRealText);
        ISO_CHECK_EQ_UINT(want.size(), 624u);
        ISO_CHECK_EQ_UINT(kRealZlibDynamicBlob.size(), 380u);
        // Confirm it really is a dynamic-Huffman block (BFINAL=1, BTYPE=10),
        // so this test cannot silently degrade to exercising fixed/stored.
        std::uint8_t header = static_cast<std::uint8_t>(kRealZlibDynamicBlob[0] & 0b111u);
        ISO_CHECK_EQ_UINT(header, 0b101u);
        Bytes got = deflate::inflate(kRealZlibDynamicBlob);
        ISO_CHECK(got == want);
        ISO_CHECK(deflate::decompress(kRealZlibDynamicBlob) == want);
    }

    // ── 9. Malformed / adversarial input must throw DeflateException, never
    //    crash, and never allocate unbounded memory ──────────────────────
    {
        // Truncated stream (claims 3-bit header but has 0 bytes).
        bool threw = false;
        try {
            deflate::inflate({});
        } catch (const deflate::DeflateException&) {
            threw = true;
        }
        ISO_CHECK(threw);

        // Reserved BTYPE=11: header byte 0b111 (BFINAL=1, BTYPE=11).
        threw = false;
        try {
            deflate::inflate(Bytes{0b111});
        } catch (const deflate::DeflateException& e) {
            threw = true;
            ISO_CHECK(e.error() == deflate::DeflateError::ReservedBlockType);
        }
        ISO_CHECK(threw);

        // Stored block with LEN/NLEN mismatch: BFINAL=1,BTYPE=00 (byte 0x01),
        // LEN=0x0005, NLEN should be 0xFFFA but we give a wrong value.
        threw = false;
        try {
            deflate::inflate(Bytes{0x01, 0x05, 0x00, 0x00, 0x00});
        } catch (const deflate::DeflateException& e) {
            threw = true;
            ISO_CHECK(e.error() == deflate::DeflateError::StoredBlockLenMismatch);
        }
        ISO_CHECK(threw);

        // A well-formed stored-block header claiming more literal bytes than
        // are actually present must throw UnexpectedEof, not read OOB.
        threw = false;
        try {
            // BFINAL=1,BTYPE=00, LEN=0xFFFF, NLEN=0x0000 (correct complement),
            // but zero payload bytes follow.
            deflate::inflate(Bytes{0x01, 0xFF, 0xFF, 0x00, 0x00});
        } catch (const deflate::DeflateException& e) {
            threw = true;
            ISO_CHECK(e.error() == deflate::DeflateError::UnexpectedEof);
        }
        ISO_CHECK(threw);

        // A fixed-Huffman block (BFINAL=1,BTYPE=01 -> byte 0x03) with no
        // token data at all must throw (truncated), not hang or crash.
        threw = false;
        try {
            deflate::inflate(Bytes{0x03});
        } catch (const deflate::DeflateException&) {
            threw = true;
        }
        ISO_CHECK(threw);

        // Random noise must never crash the decoder, only throw or (rarely)
        // decode to something and terminate; either way this must return.
        Bytes noise = {0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78,
                       0x9A, 0xBC, 0xDE, 0xF0, 0x01, 0x02, 0x03, 0x04};
        try {
            Bytes out = deflate::inflate(noise);
            (void)out;
        } catch (const deflate::DeflateException&) {
            // also acceptable
        }
        ISO_CHECK(true);  // reaching here means no crash / no infinite loop
    }

    // ── 10. Back-reference distance beyond the decoded-so-far output must
    //     be rejected, not read out of bounds. Hand-built fixed-Huffman
    //     block (BFINAL=1,BTYPE=01): literal 'A', then a length-3/distance-2
    //     match — invalid, because only 1 byte has been decoded so far —
    //     then EOB. Bits assembled MSB-first per Huffman code / LSB-first
    //     per raw field exactly as `compress` would, then LSB-packed into
    //     bytes; independently verified against RFC 1951 by hand. ─────────
    {
        Bytes bad_backref = {0x73, 0x04, 0x42, 0x00};
        bool threw = false;
        try {
            deflate::inflate(bad_backref);
        } catch (const deflate::DeflateException& e) {
            threw = true;
            ISO_CHECK(e.error() == deflate::DeflateError::BackReferenceOutOfRange);
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
