// Tests for cfb-writer, using the header-only iso_test.h harness (pure ISO).
// The sibling `cfb` reader is not ported, so this file carries a compact CFB
// reader (`extract_stream`) to prove the writer's output round-trips.
#include "iso_test.h"

#include <cstddef>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

#include "cfb_writer.hpp"

namespace cw = ca::cfb_writer;

namespace {

constexpr std::size_t kSector = 512;
constexpr std::size_t kHeader = 512;
constexpr std::size_t kMini = 64;
constexpr std::uint32_t kEndOfChain = 0xFFFFFFFEu;

std::uint16_t rd_u16(const std::vector<std::uint8_t> &b, std::size_t o) {
    return static_cast<std::uint16_t>(b[o] | (b[o + 1] << 8));
}
std::uint32_t rd_u32(const std::vector<std::uint8_t> &b, std::size_t o) {
    return static_cast<std::uint32_t>(b[o]) |
           (static_cast<std::uint32_t>(b[o + 1]) << 8) |
           (static_cast<std::uint32_t>(b[o + 2]) << 16) |
           (static_cast<std::uint32_t>(b[o + 3]) << 24);
}
std::uint64_t rd_u64(const std::vector<std::uint8_t> &b, std::size_t o) {
    std::uint64_t v = 0;
    for (int i = 0; i < 8; ++i)
        v |= static_cast<std::uint64_t>(b[o + static_cast<std::size_t>(i)])
             << (i * 8);
    return v;
}
std::size_t sector_off(std::uint32_t sid) {
    return kHeader + static_cast<std::size_t>(sid) * kSector;
}
std::uint32_t fat_at(const std::vector<std::uint8_t> &b, std::uint32_t g) {
    std::uint32_t fat_sid = rd_u32(b, 76 + static_cast<std::size_t>(g / 128) * 4);
    return rd_u32(b, sector_off(fat_sid) + static_cast<std::size_t>(g % 128) * 4);
}
std::vector<std::uint8_t> collect_chain(const std::vector<std::uint8_t> &b,
                                        std::uint32_t start) {
    std::vector<std::uint8_t> acc;
    std::uint32_t s = start;
    while (s != kEndOfChain && s < 0xFFFFFFF0u) {
        std::size_t off = sector_off(s);
        acc.insert(acc.end(), b.begin() + static_cast<std::ptrdiff_t>(off),
                   b.begin() + static_cast<std::ptrdiff_t>(off + kSector));
        s = fat_at(b, s);
    }
    return acc;
}
// Reconstruct the stream at directory index `idx`.
std::vector<std::uint8_t> extract_stream(const std::vector<std::uint8_t> &b,
                                         std::size_t idx) {
    std::uint32_t first_dir = rd_u32(b, 48);
    std::uint32_t mini_cutoff = rd_u32(b, 56);
    std::uint32_t first_minifat = rd_u32(b, 60);
    std::vector<std::uint8_t> dir = collect_chain(b, first_dir);
    std::size_t base = idx * 128;
    std::uint32_t start = rd_u32(dir, base + 116);
    std::uint64_t size = rd_u64(dir, base + 120);
    std::vector<std::uint8_t> result;
    if (size == 0) return result;
    if (size >= mini_cutoff) {
        std::vector<std::uint8_t> chain = collect_chain(b, start);
        result.assign(chain.begin(),
                      chain.begin() + static_cast<std::ptrdiff_t>(size));
    } else {
        std::uint32_t root_start = rd_u32(dir, 116);  // entry 0 mini-stream
        std::vector<std::uint8_t> mini_stream = collect_chain(b, root_start);
        std::vector<std::uint8_t> minifat = collect_chain(b, first_minifat);
        std::uint32_t mi = start;
        while (mi != kEndOfChain && result.size() < size) {
            std::size_t take = static_cast<std::size_t>(size) - result.size();
            if (take > kMini) take = kMini;
            std::size_t off = static_cast<std::size_t>(mi) * kMini;
            result.insert(result.end(),
                          mini_stream.begin() + static_cast<std::ptrdiff_t>(off),
                          mini_stream.begin() +
                              static_cast<std::ptrdiff_t>(off + take));
            mi = rd_u32(minifat, static_cast<std::size_t>(mi) * 4);
        }
    }
    return result;
}

std::vector<std::uint8_t> bytes(std::initializer_list<int> il) {
    std::vector<std::uint8_t> v;
    for (int x : il) v.push_back(static_cast<std::uint8_t>(x));
    return v;
}

}  // namespace

int main() {
    using Pair = std::pair<std::string, std::vector<std::uint8_t>>;
    const std::vector<std::uint8_t> SIG = {0xD0, 0xCF, 0x11, 0xE0,
                                           0xA1, 0xB1, 0x1A, 0xE1};

    // ── mixed small + large round-trip ─────────────────────────────────────
    {
        std::vector<std::uint8_t> workbook(5000, 0xAB);
        std::vector<std::uint8_t> another(100, 0x01);
        std::vector<std::uint8_t> mini(
            reinterpret_cast<const std::uint8_t *>("hello mini-stream"),
            reinterpret_cast<const std::uint8_t *>("hello mini-stream") + 17);
        auto cfb = cw::write_cfb(
            {{"Workbook", workbook}, {"SmallStream", mini}, {"Another", another}});
        ISO_CHECK(std::vector<std::uint8_t>(cfb.begin(), cfb.begin() + 8) == SIG);
        ISO_CHECK((cfb.size() - kHeader) % kSector == 0);
        ISO_CHECK(extract_stream(cfb, 1) == workbook);
        ISO_CHECK(extract_stream(cfb, 2) == mini);
        ISO_CHECK(extract_stream(cfb, 3) == another);
    }

    // ── header fields ──────────────────────────────────────────────────────
    {
        auto cfb = cw::write_cfb({{"Only", bytes({'x'})}});
        ISO_CHECK_EQ_UINT(rd_u16(cfb, 26), 0x0003u);
        ISO_CHECK_EQ_UINT(rd_u16(cfb, 30), 0x0009u);
        ISO_CHECK_EQ_UINT(rd_u16(cfb, 32), 0x0006u);
        ISO_CHECK_EQ_UINT(rd_u16(cfb, 28), 0xFFFEu);
        ISO_CHECK_EQ_UINT(rd_u32(cfb, 56), 4096u);
        ISO_CHECK((cfb.size() - kHeader) % kSector == 0);
    }

    // ── empty + real stream ────────────────────────────────────────────────
    {
        auto cfb = cw::write_cfb(
            {{"Nothing", {}}, {"Something", bytes({'d', 'a', 't', 'a'})}});
        ISO_CHECK(extract_stream(cfb, 1).empty());
        ISO_CHECK(extract_stream(cfb, 2) == bytes({'d', 'a', 't', 'a'}));
    }

    // ── no streams: valid minimal CFB ──────────────────────────────────────
    {
        cw::CfbWriter w;
        auto cfb = w.finish();
        ISO_CHECK(std::vector<std::uint8_t>(cfb.begin(), cfb.begin() + 8) == SIG);
        std::uint32_t first_dir = rd_u32(cfb, 48);
        ISO_CHECK(cfb[sector_off(first_dir) + 66] == 0x05);  // OBJ_ROOT
    }

    // ── exactly-cutoff large, one-under mini ───────────────────────────────
    {
        std::vector<std::uint8_t> at(4096, 0x7E);
        std::vector<std::uint8_t> under(4095, 0x7E);
        auto cfb1 = cw::write_cfb({{"AtCutoff", at}});
        ISO_CHECK(extract_stream(cfb1, 1) == at);
        auto cfb2 = cw::write_cfb({{"JustUnder", under}});
        ISO_CHECK(extract_stream(cfb2, 1) == under);
    }

    // ── many small streams over many mini-sectors ──────────────────────────
    {
        cw::CfbWriter w;
        std::vector<std::vector<std::uint8_t>> payloads;
        for (std::uint32_t i = 0; i < 50; ++i) {
            std::size_t len = (i % 200) + 1;
            std::vector<std::uint8_t> p(len, static_cast<std::uint8_t>(i & 0xFF));
            payloads.push_back(p);
            w.add_stream("s" + std::to_string(i), p);
        }
        auto cfb = w.finish();
        for (std::size_t i = 0; i < 50; ++i)
            ISO_CHECK(extract_stream(cfb, i + 1) == payloads[i]);
    }

    // ── huge stream → > 1 FAT sector ───────────────────────────────────────
    {
        std::vector<std::uint8_t> big(300u * 1024u);
        for (std::size_t i = 0; i < big.size(); ++i)
            big[i] = static_cast<std::uint8_t>(i & 0xFF);
        auto cfb = cw::write_cfb({{"Huge", big}});
        ISO_CHECK(rd_u32(cfb, 44) > 1);
        ISO_CHECK(extract_stream(cfb, 1) == big);
    }

    // ── overlong name truncated to 31 units ────────────────────────────────
    {
        auto cfb = cw::write_cfb(
            {{std::string(100, 'A'), bytes({'p', 'a', 'y'})}});
        std::uint32_t first_dir = rd_u32(cfb, 48);
        // name length incl NUL = (31 + 1) * 2 = 64
        ISO_CHECK_EQ_UINT(rd_u16(cfb, sector_off(first_dir) + 128 + 64), 64u);
        ISO_CHECK(extract_stream(cfb, 1) == bytes({'p', 'a', 'y'}));
    }

    // ── UTF-8 name → UTF-16LE (café-Ω) ─────────────────────────────────────
    {
        auto cfb = cw::write_cfb(
            {{std::string("caf\xC3\xA9-\xCE\xA9"), bytes({'u'})}});
        std::uint32_t first_dir = rd_u32(cfb, 48);
        std::size_t e = sector_off(first_dir) + 128;
        ISO_CHECK_EQ_UINT(rd_u16(cfb, e + 64), 14u);  // (6+1)*2
        std::vector<std::uint8_t> got(cfb.begin() + static_cast<std::ptrdiff_t>(e),
                                      cfb.begin() +
                                          static_cast<std::ptrdiff_t>(e + 14));
        std::vector<std::uint8_t> expect = {'c',  0, 'a',  0,    'f', 0, 0xE9,
                                            0,    '-', 0,   0xA9, 0x03, 0, 0};
        ISO_CHECK(got == expect);
    }

    // ── determinism ────────────────────────────────────────────────────────
    {
        std::vector<Pair> in = {{"A", std::vector<std::uint8_t>(5000, 9)},
                                {"B", bytes({'t', 'i', 'n', 'y'})}};
        ISO_CHECK(cw::write_cfb(in) == cw::write_cfb(in));
    }

    // ── mini-stream spanning multiple 512-byte sectors ─────────────────────
    {
        cw::CfbWriter w;
        std::vector<std::vector<std::uint8_t>> payloads;
        for (std::uint32_t i = 0; i < 20; ++i) {
            std::vector<std::uint8_t> p(200,
                                        static_cast<std::uint8_t>((i + 1) & 0xFF));
            payloads.push_back(p);
            w.add_stream("m" + std::to_string(i), p);
        }
        auto cfb = w.finish();
        for (std::size_t i = 0; i < 20; ++i)
            ISO_CHECK(extract_stream(cfb, i + 1) == payloads[i]);
    }

    return ISO_TEST_RESULT();
}
