// Tests for ge225-simulator, using the header-only iso_test.h harness (pure ISO).
// Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "ge225_simulator.hpp"

namespace ge = ca::ge225_simulator;

// Rust helper: ins(opcode, address, modifier).
static std::int32_t ins(std::int32_t opcode, std::int32_t address,
                        std::int32_t modifier) {
    return ge::encode_instruction(opcode, modifier, address);
}

int main() {
    using Words = std::vector<std::int32_t>;

    // ── encode / decode / pack round-trip ────────────────────────────────────
    {
        std::int32_t word = ins(001, 0x1234 & 0x1fff, 002);
        auto f = ge::decode_instruction(word);
        ISO_CHECK(f.opcode == 001 && f.modifier == 002 &&
                  f.address == (0x1234 & 0x1fff));
        Words words = {word, ge::assemble_fixed("NOP")};
        ISO_CHECK(ge::unpack_words(ge::pack_words(words)) == words);

        bool threw = false;
        try {
            ge::encode_instruction(0100, 0, 0);
        } catch (const ge::Error &e) {
            threw = e.kind() == ge::ErrorKind::Range;
        }
        ISO_CHECK(threw);
        threw = false;
        try {
            ge::unpack_words(std::vector<std::uint8_t>{0, 0, 0, 0});
        } catch (const ge::Error &e) {
            threw = e.kind() == ge::ErrorKind::OddByteLength;
        }
        ISO_CHECK(threw);
    }

    // ── LDA / ADD / STA program ──────────────────────────────────────────────
    {
        ge::Simulator s(4096);
        s.load_words({ins(000, 10, 0), ins(001, 11, 0), ins(003, 12, 0),
                      ge::assemble_fixed("NOP"), 0, 0, 0, 0, 0, 0, 1, 2, 0},
                     0);
        s.run(4);
        ISO_CHECK(s.a() == 3);
        ISO_CHECK(s.read_word(12) == 3);
    }

    // ── SPB stores P ─────────────────────────────────────────────────────────
    {
        ge::Simulator s(4096);
        s.load_words({ins(007, 4, 2), ge::assemble_fixed("NOP"),
                      ge::assemble_fixed("NOP"), ge::assemble_fixed("NOP"),
                      ins(000, 10, 0), ge::assemble_fixed("NOP"), 0, 0, 0, 0,
                      0x12345},
                     0);
        s.run(3);
        ISO_CHECK(s.x_word(2) == 0);
        ISO_CHECK(s.a() == 0x12345);
    }

    // ── odd-address double ops (DLD / DST) ───────────────────────────────────
    {
        ge::Simulator s(4096);
        s.write_word(11, 0x13579);
        s.load_words({ins(010, 11, 0), ins(013, 13, 0),
                      ge::assemble_fixed("NOP")},
                     0);
        s.run(3);
        ISO_CHECK(s.a() == 0x13579);
        ISO_CHECK(s.q() == 0x13579);
        ISO_CHECK(s.read_word(13) == 0x13579);
    }

    // ── MOY moves blocks ─────────────────────────────────────────────────────
    {
        ge::Simulator s(4096);
        s.write_word(20, 0x11111);
        s.write_word(21, 0x22222);
        s.write_word(30, 40);
        s.write_word(31, (1 << 20) - 2);
        s.load_words({ins(000, 30, 0), ge::assemble_fixed("LQA"),
                      ins(000, 31, 0), ge::assemble_fixed("XAQ"),
                      ins(024, 20, 0), ge::assemble_fixed("NOP")},
                     0);
        s.run(6);
        ISO_CHECK(s.a() == 0);
        ISO_CHECK(s.read_word(40) == 0x11111);
        ISO_CHECK(s.read_word(41) == 0x22222);
    }

    // ── console typewriter path ──────────────────────────────────────────────
    {
        ge::Simulator s(4096);
        s.set_control_switches(01633);
        s.load_words({ge::assemble_fixed("RCS"), ge::assemble_fixed("TON"),
                      ge::assemble_shift("SAN", 6), ge::assemble_fixed("TYP"),
                      ge::assemble_fixed("NOP")},
                     0);
        s.run(5);
        ISO_CHECK_STR_EQ(s.typewriter_output().c_str(), "-");
        ISO_CHECK(s.typewriter_power());
    }

    // ── RCD loads a queued card record ───────────────────────────────────────
    {
        ge::Simulator s(4096);
        s.queue_card_reader_record({0x11111, 0x22222});
        s.load_words({ins(025, 10, 0), ge::assemble_fixed("NOP")}, 0);
        s.run(2);
        ISO_CHECK(s.read_word(10) == 0x11111);
        ISO_CHECK(s.read_word(11) == 0x22222);
    }

    // ── disassembly + divide-by-zero ─────────────────────────────────────────
    {
        ge::Simulator s(256);
        ISO_CHECK_STR_EQ(s.disassemble_word(ge::assemble_fixed("NOP")).c_str(),
                         "NOP");
        ISO_CHECK_STR_EQ(s.disassemble_word(ins(001, 0x123, 2)).c_str(),
                         "ADD 0x123,X2");
        s.load_words({ins(016, 5, 0)}, 0);  // DVD 5, mem[5]==0
        bool threw = false;
        try {
            s.step();
        } catch (const ge::Error &e) {
            threw = e.kind() == ge::ErrorKind::DivideByZero;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
