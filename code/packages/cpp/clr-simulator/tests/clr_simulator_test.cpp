// Tests for clr-simulator, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's unit tests, plus extra bounds-safety cases.
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "clr_simulator.hpp"

namespace clr = ca::clr_simulator;

// The integer at the top of the stack (or a sentinel if it is not an int).
static std::int32_t top_int(const clr::Simulator& sim) {
    auto s = sim.stack_top();
    if (!s.has_value() || !s->is_int()) {
        return -999999;
    }
    return s->as_int();
}

int main() {
    using Bytes = std::vector<std::uint8_t>;

    // ── clr_simulator_math: 1 + 2 -> local 0, reload, ret ──────────────────
    {
        Bytes prog = {0x17, 0x18, 0x58, 0x0A, 0x06, 0x2A};
        clr::Simulator sim;
        sim.load(prog, 16);
        std::size_t steps = sim.run(100);
        ISO_CHECK_EQ_UINT(steps, 6u);
        ISO_CHECK(sim.halted());
        auto loc = sim.local_at(0);
        ISO_CHECK(loc.has_value() && loc->is_int());
        ISO_CHECK_EQ_INT(loc->as_int(), 3);
    }

    // ── clr_div_by_zero: throws Error(DivideByZero) ────────────────────────
    {
        Bytes prog = {0x17, 0x16, 0x5B};
        clr::Simulator sim;
        sim.load(prog, 4);
        bool threw = false;
        try {
            sim.run(100);
        } catch (const clr::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == clr::ErrorKind::DivideByZero);
        }
        ISO_CHECK(threw);
    }

    // ── clr_extended_opcodes: 10 cgt 5 == 1 ────────────────────────────────
    {
        Bytes prog = {0x1F, 10, 0x1B, clr::kOpPrefixFe, clr::kCgtByte};
        clr::Simulator sim;
        sim.load(prog, 4);
        // Runs 3 steps, then falls off the end (no ret).
        try {
            sim.run(100);
        } catch (const clr::Error& e) {
            ISO_CHECK(e.kind() == clr::ErrorKind::PcOutOfRange);
        }
        ISO_CHECK_EQ_INT(top_int(sim), 1);
    }

    // ── clr_branching_zero: brfalse.s skips the ldc.i4 1000 ────────────────
    {
        Bytes prog = {0x16, 0x2C, 5,    0x20, 0xE8,
                      0x03, 0x00, 0x00, 0x1F, 10};
        clr::Simulator sim;
        sim.load(prog, 4);
        try {
            sim.run(100);
        } catch (const clr::Error&) {
        }
        ISO_CHECK_EQ_INT(top_int(sim), 10);
    }

    // ── clr_object_array_cons_roundtrip ────────────────────────────────────
    {
        Bytes prog = {
            0x18,                    // ldc.i4 2
            0x8D, 0,    0, 0, 0,     // newarr
            0x25,                    // dup
            0x16,                    // ldc.i4 0
            0x1D,                    // ldc.i4 7
            0x8C, 0,    0, 0, 0,     // box
            0xA4,                    // stelem.ref arr[0] = 7
            0x25,                    // dup
            0x17,                    // ldc.i4 1
            0x1F, 9,                 // ldc.i4.s 9
            0x8C, 0,    0, 0, 0,     // box
            0xA4,                    // stelem.ref arr[1] = 9
            0x16,                    // ldc.i4 0
            0xA2,                    // ldelem.ref -> arr[0]
            0xA5, 0,    0, 0, 0      // unbox.any
        };
        clr::Simulator sim;
        sim.load(prog, 4);
        try {
            sim.run(100);
        } catch (const clr::Error&) {
        }
        ISO_CHECK_EQ_INT(top_int(sim), 7);
    }

    // ── clr_null_is_falsy: ldnull pushes a null reference ──────────────────
    {
        Bytes prog = {0x14};
        clr::Simulator sim;
        sim.load(prog, 4);
        sim.step();
        ISO_CHECK_EQ_UINT(sim.stack().size(), 1u);
        const clr::Slot& s = sim.stack()[0];
        ISO_CHECK(s.has_value() && s->is_ref() && !s->ref().has_value());
    }

    // ── clr_halted: stepping a halted machine throws ───────────────────────
    {
        Bytes prog = {0x2A};  // ret with no frame -> halt
        clr::Simulator sim;
        sim.load(prog, 4);
        sim.step();
        ISO_CHECK(sim.halted());
        bool threw = false;
        try {
            sim.step();
        } catch (const clr::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == clr::ErrorKind::Halted);
        }
        ISO_CHECK(threw);
    }

    // ── method call: entry calls a 2-arg adder ─────────────────────────────
    {
        clr::Method entry;
        entry.body = {0x1F, 20,   0x1F, 22,  0x28,
                      0x02, 0x00, 0x00, 0x06, 0x2A};
        entry.num_locals = 0;
        entry.num_args = 0;
        clr::Method adder;
        adder.body = {0x02, 0x03, 0x58, 0x2A};
        adder.num_locals = 0;
        adder.num_args = 2;
        clr::Simulator sim;
        sim.load_program({entry, adder}, 0);
        std::size_t steps = sim.run(100);
        (void)steps;
        ISO_CHECK(sim.halted());
        ISO_CHECK_EQ_INT(top_int(sim), 42);
    }

    // ── bounds safety: a truncated ldc.i4 operand throws, no OOB read ──────
    {
        Bytes prog = {0x20};  // ldc.i4 with no 4-byte operand
        clr::Simulator sim;
        sim.load(prog, 4);
        bool threw = false;
        try {
            sim.step();
        } catch (const clr::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == clr::ErrorKind::BytecodeOverrun);
        }
        ISO_CHECK(threw);
    }

    // ── unknown opcode is rejected ─────────────────────────────────────────
    {
        Bytes prog = {0x99};
        clr::Simulator sim;
        sim.load(prog, 4);
        bool threw = false;
        try {
            sim.step();
        } catch (const clr::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == clr::ErrorKind::UnknownOpcode);
        }
        ISO_CHECK(threw);
    }

    // ── stelem.ref index out of range throws, not crashes ──────────────────
    {
        Bytes prog = {0x17, 0x8D, 0, 0, 0, 0, 0x25, 0x1B, 0x1D, 0xA4};
        clr::Simulator sim;
        sim.load(prog, 4);
        bool threw = false;
        try {
            sim.run(100);
        } catch (const clr::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == clr::ErrorKind::IndexOutOfRange);
        }
        ISO_CHECK(threw);
    }

    // ── encoding helpers + assemble ────────────────────────────────────────
    {
        ISO_CHECK((clr::Simulator::encode_ldc_i4(5) == Bytes{0x1B}));
        ISO_CHECK((clr::Simulator::encode_ldc_i4(100) == Bytes{0x1F, 100}));
        ISO_CHECK((clr::Simulator::encode_ldc_i4(1000) ==
                   Bytes{0x20, 0xE8, 0x03, 0x00, 0x00}));
        ISO_CHECK((clr::Simulator::encode_ldc_i4(-1) == Bytes{0x1F, 0xFF}));
        ISO_CHECK((clr::Simulator::encode_stloc(0) == Bytes{0x0A}));
        ISO_CHECK((clr::Simulator::encode_stloc(5) == Bytes{0x13, 5}));
        ISO_CHECK((clr::Simulator::encode_ldloc(2) == Bytes{0x08}));
        ISO_CHECK((clr::Simulator::encode_ldloc(9) == Bytes{0x11, 9}));

        // assemble: 1 + 2; add via the helpers, then run it.
        auto blob = clr::Simulator::assemble({clr::Simulator::encode_ldc_i4(1),
                                              clr::Simulator::encode_ldc_i4(2),
                                              {clr::kOpAdd},
                                              {clr::kOpRet}});
        clr::Simulator sim;
        sim.load(blob, 4);
        sim.run(100);
        ISO_CHECK_EQ_INT(top_int(sim), 3);
    }

    return ISO_TEST_RESULT();
}
