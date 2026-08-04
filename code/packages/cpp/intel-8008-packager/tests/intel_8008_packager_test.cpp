// Tests for the C++ intel-8008-packager library, using the header-only
// iso_test.h harness (pure ISO). Byte vectors and error expectations mirror the
// Rust crate's own unit tests one-for-one.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "intel_8008_packager.hpp"

namespace pak = ca::intel_8008_packager;
using Bytes = std::vector<std::uint8_t>;

// True iff decode(text) throws a PackagerError whose message contains keyword.
static bool dec_throws(const std::string& text, const char* keyword) {
    try {
        pak::decode_hex(text);
        return false;
    } catch (const pak::PackagerError& e) {
        return std::string(e.what()).find(keyword) != std::string::npos;
    }
}

static std::size_t count_lines(const std::string& s) {
    std::size_t c = 0;
    for (char ch : s)
        if (ch == '\n') c++;
    return c;
}

int main() {
    // ── encode: exact small vectors ────────────────────────────────────────
    ISO_CHECK(pak::encode_hex(Bytes{0xFF}, 0) == ":01000000FF00\n:00000001FF\n");
    ISO_CHECK(pak::encode_hex(Bytes{0x06, 0x00, 0xFF}, 0) ==
              ":030000000600FFF8\n:00000001FF\n");

    // ── encode: structural checks ──────────────────────────────────────────
    {
        auto s = pak::encode_hex(Bytes{0x01, 0x02, 0x03}, 0);
        ISO_CHECK(s[0] == ':');
        ISO_CHECK(count_lines(s) == 2);
        ISO_CHECK(s.find(":00000001FF\n") != std::string::npos);

        auto p = pak::encode_hex(Bytes{0x06, 0x00, 0xFF}, 0);
        ISO_CHECK(p.substr(1, 2) == "03");
        ISO_CHECK(p.substr(3, 4) == "0000");
        ISO_CHECK(p.substr(7, 2) == "00");
        ISO_CHECK(p.substr(9, 6) == "0600FF");

        Bytes b16;
        for (int i = 0; i < 16; i++) b16.push_back(static_cast<std::uint8_t>(i));
        auto s16 = pak::encode_hex(b16, 0);
        ISO_CHECK(count_lines(s16) == 2 && s16.substr(1, 2) == "10");

        Bytes b17 = b16;
        b17.push_back(16);
        auto s17 = pak::encode_hex(b17, 0);
        ISO_CHECK(count_lines(s17) == 3 && s17.substr(1, 2) == "10");
        auto l2 = s17.substr(s17.find('\n') + 1);
        ISO_CHECK(l2.substr(1, 2) == "01");

        auto z32 = pak::encode_hex(Bytes(32, 0), 0);
        ISO_CHECK(z32.substr(3, 4) == "0000");
        ISO_CHECK(z32.substr(z32.find('\n') + 1 + 3, 4) == "0010");

        ISO_CHECK(pak::encode_hex(Bytes{0x7C, 0x03, 0x00, 0xFF}, 0x0100).substr(3, 4) == "0100");
        ISO_CHECK(pak::encode_hex(Bytes(4, 0), 0x2000).substr(3, 4) == "2000");
    }

    // ── encode: error cases ────────────────────────────────────────────────
    {
        bool threw = false;
        try { pak::encode_hex(Bytes{}, 0); } catch (const pak::PackagerError& e) {
            threw = std::string(e.what()).find("non-empty") != std::string::npos;
        }
        ISO_CHECK(threw);

        threw = false;
        try { pak::encode_hex(Bytes{1, 2}, 0xFFFF); } catch (const pak::PackagerError&) { threw = true; }
        ISO_CHECK(threw);

        threw = false;
        try { pak::encode_hex(Bytes{1}, 0x10000); } catch (const pak::PackagerError&) { threw = true; }
        ISO_CHECK(threw);
    }

    // ── decode: round trips ────────────────────────────────────────────────
    {
        struct Case { std::size_t n, origin; };
        for (Case c : {Case{1, 0}, Case{3, 0}, Case{17, 0}, Case{4, 0x0100}, Case{16, 0x3FF0}}) {
            Bytes bin;
            for (std::size_t i = 0; i < c.n; i++)
                bin.push_back(static_cast<std::uint8_t>(i * 7 + 1));
            auto d = pak::decode_hex(pak::encode_hex(bin, c.origin));
            ISO_CHECK(d.origin == c.origin && d.binary == bin);
        }
        Bytes big(pak::MAX_IMAGE_SIZE, 0xFF);  // full 16 KB, span at the limit
        auto d = pak::decode_hex(pak::encode_hex(big, 0));
        ISO_CHECK(d.binary.size() == pak::MAX_IMAGE_SIZE);
    }

    // ── decode: error cases ────────────────────────────────────────────────
    ISO_CHECK(dec_throws("03000000060000F7\n:00000001FF\n", "':'"));
    ISO_CHECK(dec_throws(":0ZZZZ000060000F7\n:00000001FF\n", "hex"));
    ISO_CHECK(dec_throws(":100\n:00000001FF\n", "hex"));  // odd body
    ISO_CHECK(dec_throws(":020000020000FC\n:00000001FF\n", "unsupported"));
    ISO_CHECK(dec_throws(":050000000102\n:00000001FF\n", "too short"));
    ISO_CHECK(dec_throws(":0100000000FF\n", "EOF"));
    ISO_CHECK(dec_throws(":0100000000FF\n:01400100FFBF\n:00000001FF\n", "large"));
    ISO_CHECK(dec_throws(":0100000042BD\n:0100000042BD\n:00000001FF\n", "overlap"));

    // bad checksum: encode [1,2,3], corrupt the first record's checksum
    {
        auto s = pak::encode_hex(Bytes{1, 2, 3}, 0);
        std::size_t nl = s.find('\n');
        s[nl - 1] = '0';
        s[nl - 2] = '0';
        ISO_CHECK(dec_throws(s, "checksum"));
    }

    // overlapping: 16-byte record at 0 + 1-byte record inside it
    {
        auto hexa = pak::encode_hex(Bytes(16, 0), 0);
        std::string rec_a = hexa.substr(0, hexa.find('\n'));
        auto combined = rec_a + "\n:0100050000FA\n:00000001FF\n";
        ISO_CHECK(dec_throws(combined, "overlap"));
        // out-of-order: same records, B before A
        auto combined2 = std::string(":0100050000FA\n") + rec_a + "\n:00000001FF\n";
        ISO_CHECK(dec_throws(combined2, "overlap"));
    }

    // empty (EOF-only) file decodes to empty binary
    {
        auto d = pak::decode_hex(":00000001FF\n");
        ISO_CHECK(d.origin == 0 && d.binary.empty());
    }

    // line too long
    {
        std::string ll = ":" + std::string(1200, 'A') + "\n:00000001FF\n";
        ISO_CHECK(dec_throws(ll, "long"));
    }

    // ── 8008 top-of-space round trip (0x3FF0 + 16 = 0x4000) ────────────────
    {
        Bytes bin(16, 0xFF);
        auto d = pak::decode_hex(pak::encode_hex(bin, 0x3FF0));
        ISO_CHECK(d.origin == 0x3FF0 && d.binary == bin);
    }

    return ISO_TEST_RESULT();
}
