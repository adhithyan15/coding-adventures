// Tests for cfb. Like the C port, we craft in-memory CFBs (the Rust tests use
// an embedded .xls fixture plus crafted builders) so the full read path is
// exercised without an external file. Uses the header-only iso_test.h harness.
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "cfb.hpp"

namespace cfb = ca::cfb;
using Bytes = std::vector<std::uint8_t>;

static constexpr std::size_t HEADER_LEN = 512, SECTOR = 512,
                             DIR_ENTRY_SIZE = 128, HEADER_DIFAT_OFFSET = 76,
                             HEADER_DIFAT_COUNT = 109;
static constexpr std::uint32_t FREESECT = 0xFFFFFFFFu, ENDOFCHAIN = 0xFFFFFFFEu,
                               FATSECT = 0xFFFFFFFDu, NOSTREAM = 0xFFFFFFFFu;
static const std::uint8_t SIG[8] = {0xD0, 0xCF, 0x11, 0xE0,
                                    0xA1, 0xB1, 0x1A, 0xE1};

static void wu16(Bytes& b, std::size_t o, std::uint16_t v) {
    b[o] = static_cast<std::uint8_t>(v);
    b[o + 1] = static_cast<std::uint8_t>(v >> 8);
}
static void wu32(Bytes& b, std::size_t o, std::uint32_t v) {
    b[o] = static_cast<std::uint8_t>(v);
    b[o + 1] = static_cast<std::uint8_t>(v >> 8);
    b[o + 2] = static_cast<std::uint8_t>(v >> 16);
    b[o + 3] = static_cast<std::uint8_t>(v >> 24);
}
static void wu64(Bytes& b, std::size_t o, std::uint64_t v) {
    wu32(b, o, static_cast<std::uint32_t>(v));
    wu32(b, o + 4, static_cast<std::uint32_t>(v >> 32));
}
static void wname(Bytes& b, std::size_t off, const std::string& name) {
    std::size_t n = 0;
    for (std::size_t i = 0; i < name.size(); ++i) {
        wu16(b, off + i * 2,
             static_cast<std::uint16_t>(static_cast<unsigned char>(name[i])));
        ++n;
    }
    wu16(b, off + 64, static_cast<std::uint16_t>((n + 1) * 2));
}

static Bytes craft_mini(bool second, bool cycle) {
    std::size_t fat_off = HEADER_LEN, dir_off = HEADER_LEN + SECTOR;
    std::size_t mf_off = HEADER_LEN + 2 * SECTOR, ms_off = HEADER_LEN + 3 * SECTOR;
    Bytes b(HEADER_LEN + 4 * SECTOR, 0);
    static const std::uint8_t payload[8] = {0xDE, 0xAD, 0xBE, 0xEF,
                                            0x01, 0x02, 0x03, 0x04};
    std::size_t i;
    for (i = 0; i < 8; ++i) b[i] = SIG[i];
    b[30] = 0x09;
    b[32] = 0x06;
    wu32(b, 44, 1);
    wu32(b, 48, 1);
    wu32(b, 56, 4096);
    wu32(b, 60, 2);
    wu32(b, 64, 1);
    wu32(b, 68, ENDOFCHAIN);
    wu32(b, 72, 0);
    wu32(b, HEADER_DIFAT_OFFSET, 0);
    for (i = 1; i < HEADER_DIFAT_COUNT; ++i) {
        wu32(b, HEADER_DIFAT_OFFSET + i * 4, FREESECT);
    }
    for (i = 0; i < SECTOR / 4; ++i) wu32(b, fat_off + i * 4, FREESECT);
    wu32(b, fat_off + 0, FATSECT);
    wu32(b, fat_off + 4, ENDOFCHAIN);
    wu32(b, fat_off + 8, ENDOFCHAIN);
    wu32(b, fat_off + 12, ENDOFCHAIN);

    std::size_t root = dir_off, st = dir_off + DIR_ENTRY_SIZE;
    wname(b, root, "Root Entry");
    b[root + 66] = 5;
    wu32(b, root + 68, NOSTREAM);
    wu32(b, root + 72, NOSTREAM);
    wu32(b, root + 76, 1);
    wu32(b, root + 116, 3);
    wu64(b, root + 120, 64);

    wname(b, st, "Tiny");
    b[st + 66] = 2;
    wu32(b, st + 68, second ? 2u : NOSTREAM);
    wu32(b, st + 72, NOSTREAM);
    wu32(b, st + 76, NOSTREAM);
    wu32(b, st + 116, 0);
    wu64(b, st + 120, 8);

    if (second) {
        std::size_t st2 = dir_off + 2 * DIR_ENTRY_SIZE;
        wname(b, st2, "Two");
        b[st2 + 66] = 2;
        wu32(b, st2 + 68, cycle ? 1u : NOSTREAM);
        wu32(b, st2 + 72, NOSTREAM);
        wu32(b, st2 + 76, NOSTREAM);
        wu32(b, st2 + 116, 0);
        wu64(b, st2 + 120, 0);
    }

    for (i = 0; i < SECTOR / 4; ++i) wu32(b, mf_off + i * 4, FREESECT);
    wu32(b, mf_off + 0, ENDOFCHAIN);
    for (i = 0; i < 8; ++i) b[ms_off + i] = payload[i];
    return b;
}

static Bytes craft_fat_cycle() {
    std::size_t fat_off = HEADER_LEN, i;
    Bytes b(HEADER_LEN + 2 * SECTOR, 0);
    for (i = 0; i < 8; ++i) b[i] = SIG[i];
    b[30] = 0x09;
    b[32] = 0x06;
    wu32(b, 44, 1);
    wu32(b, 48, 1);
    wu32(b, 56, 4096);
    wu32(b, 60, ENDOFCHAIN);
    wu32(b, 64, 0);
    wu32(b, 68, ENDOFCHAIN);
    wu32(b, 72, 0);
    wu32(b, HEADER_DIFAT_OFFSET, 0);
    for (i = 1; i < HEADER_DIFAT_COUNT; ++i) {
        wu32(b, HEADER_DIFAT_OFFSET + i * 4, FREESECT);
    }
    for (i = 0; i < SECTOR / 4; ++i) wu32(b, fat_off + i * 4, FREESECT);
    wu32(b, fat_off + 0, FATSECT);
    wu32(b, fat_off + 4, 1); // POISON
    return b;
}

static std::optional<cfb::CfbError> open_err(const Bytes& b) {
    try {
        cfb::CompoundFile::open(b);
        return std::nullopt;
    } catch (cfb::CfbError e) {
        return e;
    }
}

int main() {
    // ── mini-stream round-trip ────────────────────────────────────────────
    {
        auto cf = cfb::CompoundFile::open(craft_mini(false, false));
        ISO_CHECK_EQ_UINT(cf.sector_size(), 512u);
        std::size_t roots = 0, tinys = 0;
        for (const auto& e : cf.entries()) {
            if (e.kind == cfb::EntryKind::RootStorage) roots++;
            if (e.kind == cfb::EntryKind::Stream && e.name == "Tiny") tinys++;
        }
        ISO_CHECK(roots == 1);
        ISO_CHECK(tinys == 1);
        auto data = cf.read_stream("Tiny");
        ISO_CHECK(data.has_value());
        ISO_CHECK_EQ_UINT(data->size(), 8u);
        Bytes want = {0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04};
        ISO_CHECK(*data == want);
        ISO_CHECK(cf.read_stream("TINY").has_value());
        ISO_CHECK(cf.read_stream("tiny").has_value());
        ISO_CHECK(!cf.read_stream("does-not-exist").has_value());
        // read_stream_by_id on the root storage throws NotAStream
        bool threw = false;
        try {
            cf.read_stream_by_id(0);
        } catch (cfb::CfbError e) {
            threw = (e == cfb::CfbError::NotAStream);
        }
        ISO_CHECK(threw);
    }

    // ── multi-entry flatten ───────────────────────────────────────────────
    {
        auto cf = cfb::CompoundFile::open(craft_mini(true, false));
        std::size_t streams = 0;
        for (const auto& e : cf.entries()) {
            if (e.kind == cfb::EntryKind::Stream) streams++;
        }
        ISO_CHECK_EQ_UINT(streams, 2u);
        ISO_CHECK_EQ_UINT(cf.stream_names().size(), 2u);
    }

    // ── directory-tree cycle detected ─────────────────────────────────────
    ISO_CHECK(open_err(craft_mini(true, true)) == cfb::CfbError::CycleDetected);

    // ── FAT sector-chain cycle detected ───────────────────────────────────
    {
        auto e = open_err(craft_fat_cycle());
        ISO_CHECK(e == cfb::CfbError::CycleDetected ||
                  e == cfb::CfbError::BadSectorChain);
    }

    // ── error paths ───────────────────────────────────────────────────────
    {
        ISO_CHECK(open_err(Bytes{}) == cfb::CfbError::Truncated);
        Bytes sh(SIG, SIG + 8);
        sh.resize(18, 0);
        ISO_CHECK(open_err(sh) == cfb::CfbError::Truncated);

        Bytes badsig = craft_mini(false, false);
        badsig[0] = 0x00;
        ISO_CHECK(open_err(badsig) == cfb::CfbError::BadSignature);
        ISO_CHECK(open_err(Bytes(600, 0)) == cfb::CfbError::BadSignature);

        Bytes badsec = craft_mini(false, false);
        badsec[30] = 0x0A;
        badsec[31] = 0x00;
        ISO_CHECK(open_err(badsec) == cfb::CfbError::UnsupportedSectorSize);

        Bytes full = craft_mini(false, false);
        Bytes header_only(full.begin(), full.begin() + HEADER_LEN);
        ISO_CHECK(open_err(header_only).has_value()); // some error, no crash
    }

    // ── truncation fuzz ───────────────────────────────────────────────────
    {
        Bytes full = craft_mini(true, false);
        for (std::size_t n = 0; n <= full.size(); ++n) {
            Bytes prefix(full.begin(),
                         full.begin() + static_cast<std::ptrdiff_t>(n));
            try {
                auto cf = cfb::CompoundFile::open(prefix);
                (void)cf.read_stream("Tiny");
            } catch (cfb::CfbError&) {
                // expected for short/invalid prefixes
            }
        }
        ISO_CHECK(true);
    }

    return ISO_TEST_RESULT();
}
