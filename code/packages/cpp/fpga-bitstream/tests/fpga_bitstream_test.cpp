// Tests for the C++ fpga-bitstream emitter, using the header-only iso_test.h
// harness (pure ISO). Expected byte streams are the AUTHORITATIVE output of the
// real Rust crate (empty Hx1k → ff00020703050004800000ffff, 13 bytes).
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <vector>

#include "fpga_bitstream.hpp"

namespace fpga = ca::fpga;

static bool bytes_eq(const std::vector<std::uint8_t>& a, const std::uint8_t* b,
                     std::size_t n) {
    if (a.size() < n) return false;
    for (std::size_t i = 0; i < n; i++) {
        if (a[i] != b[i]) return false;
    }
    return true;
}

int main() {
    // ── part specs ───────────────────────────────────────────────────────
    {
        fpga::PartSpec s = fpga::part_specs(fpga::Ice40Part::Hx1k);
        ISO_CHECK(s.rows == 33 && s.cols == 17 && s.cram_bits == 1024);
        s = fpga::part_specs(fpga::Ice40Part::Hx8k);
        ISO_CHECK(s.rows == 33 && s.cols == 33 && s.cram_bits == 1024);
        s = fpga::part_specs(fpga::Ice40Part::Lp1k);
        ISO_CHECK(s.rows == 33 && s.cols == 17 && s.cram_bits == 1024);
    }

    // ── the empty Hx1k stream (exact oracle bytes) ───────────────────────
    {
        fpga::FpgaConfig cfg(fpga::Ice40Part::Hx1k);
        auto [bytes, rep] = fpga::emit_bitstream(cfg);
        ISO_CHECK_EQ_UINT(bytes.size(), 13u);
        static const std::uint8_t expected[13] = {0xFF, 0x00, 0x02, 0x07, 0x03,
                                                  0x05, 0x00, 0x04, 0x80, 0x00,
                                                  0x00, 0xFF, 0xFF};
        ISO_CHECK(bytes_eq(bytes, expected, 13));
        ISO_CHECK(rep.clb_count == 0u && rep.cram_size == 128u &&
                  rep.bytes_written == 13u);
        ISO_CHECK(rep.part == fpga::Ice40Part::Hx1k);
    }

    // ── one CLB at (1, 2): 149 bytes with the right framing ──────────────
    {
        fpga::FpgaConfig cfg(fpga::Ice40Part::Hx1k);
        cfg.clbs[{1, 2}] = fpga::ClbConfig{};
        auto [b, rep] = fpga::emit_bitstream(cfg);
        ISO_CHECK_EQ_UINT(b.size(), 149u);
        ISO_CHECK(rep.clb_count == 1u);
        static const std::uint8_t head[13] = {0xFF, 0x00, 0x02, 0x07, 0x03, 0x05,
                                              0x00, 0x06, 0x06, 0x00, 0x01, 0x00,
                                              0x02};
        ISO_CHECK(bytes_eq(b, head, 13));
        if (b.size() == 149) {
            ISO_CHECK(b[13] == 0x82 && b[14] == 0x08);
            ISO_CHECK(b[15] == 0x00 && b[142] == 0x00);
            static const std::uint8_t tail[6] = {0x04, 0x80, 0x00,
                                                 0x00, 0xFF, 0xFF};
            for (std::size_t i = 0; i < 6; i++) ISO_CHECK(b[143 + i] == tail[i]);
        }
    }

    // ── insertion order does not change the stream (std::map is sorted) ──
    {
        fpga::FpgaConfig a(fpga::Ice40Part::Hx8k), d(fpga::Ice40Part::Hx8k);
        fpga::ClbConfig clb{};
        a.clbs[{0, 0}] = clb;
        a.clbs[{2, 5}] = clb;
        a.clbs[{1, 3}] = clb;
        d.clbs[{1, 3}] = clb;
        d.clbs[{2, 5}] = clb;
        d.clbs[{0, 0}] = clb;
        ISO_CHECK(fpga::emit_bitstream(a).first == fpga::emit_bitstream(d).first);
    }

    // ── inserting the same key overwrites (map semantics) ────────────────
    {
        fpga::FpgaConfig cfg(fpga::Ice40Part::Hx1k);
        cfg.clbs[{4, 4}] = fpga::ClbConfig{};
        cfg.clbs[{4, 4}] = fpga::ClbConfig{};
        ISO_CHECK_EQ_UINT(cfg.clbs.size(), 1u);
    }

    // ── cmd builds a record; overlong payloads throw ─────────────────────
    {
        std::vector<std::uint8_t> rec = fpga::cmd(0x06, {0x00, 0x01, 0x00, 0x02});
        static const std::uint8_t want[6] = {0x06, 0x06, 0x00, 0x01, 0x00, 0x02};
        ISO_CHECK(rec.size() == 6 && bytes_eq(rec, want, 6));
        std::vector<std::uint8_t> empty = fpga::cmd(0x07, {});
        ISO_CHECK(empty.size() == 2 && empty[0] == 0x02 && empty[1] == 0x07);

        bool threw = false;
        try {
            (void)fpga::cmd(0x08, std::vector<std::uint8_t>(254, 0));
        } catch (const std::length_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
