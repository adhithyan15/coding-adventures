// Tests for intel4004-simulator, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests (pure ISO C++17).
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "intel4004_simulator.hpp"

namespace i4 = ca::intel4004_simulator;
using Prog = std::vector<std::uint8_t>;

// Build a 4096-byte simulator, run `program` (cap 1000 steps), return it.
static i4::Simulator run_program(const Prog& program) {
    i4::Simulator s(4096);
    s.run(program, 1000);
    return s;
}

int main() {
    // ── NOP and HLT ──────────────────────────────────────────────────────────
    {
        i4::Simulator s;
        auto tr = s.run({i4::encode_nop(), i4::encode_nop(), i4::encode_hlt()},
                        10);
        ISO_CHECK_EQ_UINT(tr.size(), 3u);
        ISO_CHECK_STR_EQ(tr[0].mnemonic.c_str(), "NOP");
        ISO_CHECK_STR_EQ(tr[1].mnemonic.c_str(), "NOP");
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_hlt()});
        ISO_CHECK(s.halted());
    }
    {  // step() on a halted CPU throws
        i4::Simulator s;
        s.run({i4::encode_hlt()}, 10);
        bool threw = false;
        try {
            s.step();
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── LDM ──────────────────────────────────────────────────────────────────
    {
        auto s = run_program({i4::encode_ldm(7), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 7u);
    }
    for (std::uint8_t n = 0; n <= 15; ++n) {
        auto s = run_program({i4::encode_ldm(n), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), n);
    }

    // ── LD / XCH ─────────────────────────────────────────────────────────────
    {
        auto s = run_program({i4::encode_ldm(9), i4::encode_xch(0),
                              i4::encode_ld(0), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 9u);
    }
    {
        auto s = run_program({i4::encode_ldm(3), i4::encode_xch(0),
                              i4::encode_ldm(5), i4::encode_xch(0),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
        ISO_CHECK_EQ_UINT(s.register_at(0), 5u);
    }

    // ── ADD ──────────────────────────────────────────────────────────────────
    {
        auto s = run_program({i4::encode_ldm(1), i4::encode_xch(0),
                              i4::encode_ldm(2), i4::encode_add(0),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(8), i4::encode_xch(0),
                              i4::encode_ldm(9), i4::encode_add(0),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 1u);
        ISO_CHECK(s.carry());
    }
    {  // ADD includes carry: 4 + 4 + 1 = 9
        i4::Simulator s;
        s.load_program({i4::encode_ldm(4), i4::encode_xch(0), i4::encode_ldm(4),
                        i4::encode_stc(), i4::encode_add(0), i4::encode_hlt()});
        while (!s.halted()) {
            s.step();
        }
        ISO_CHECK_EQ_UINT(s.accumulator(), 9u);
    }

    // ── SUB (inverted-carry convention) ──────────────────────────────────────
    {
        auto s = run_program({i4::encode_ldm(3), i4::encode_xch(0),
                              i4::encode_ldm(5), i4::encode_sub(0),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 2u);
        ISO_CHECK(s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(5), i4::encode_xch(0),
                              i4::encode_ldm(3), i4::encode_sub(0),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 14u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(1), i4::encode_xch(0),
                              i4::encode_ldm(0), i4::encode_sub(0),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 15u);
        ISO_CHECK(!s.carry());
    }

    // ── INC ──────────────────────────────────────────────────────────────────
    {
        auto s = run_program({i4::encode_ldm(7), i4::encode_xch(2),
                              i4::encode_inc(2), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(2), 8u);
    }
    {
        auto s = run_program({i4::encode_ldm(15), i4::encode_xch(0),
                              i4::encode_inc(0), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(0), 0u);
    }
    {  // INC does not affect carry
        i4::Simulator s;
        s.load_program({i4::encode_ldm(15), i4::encode_xch(0), i4::encode_stc(),
                        i4::encode_inc(0), i4::encode_hlt()});
        while (!s.halted()) {
            s.step();
        }
        ISO_CHECK_EQ_UINT(s.register_at(0), 0u);
        ISO_CHECK(s.carry());
    }

    // ── JUN ──────────────────────────────────────────────────────────────────
    {
        auto jun = i4::encode_jun(0x004);
        auto s = run_program({jun.first, jun.second, i4::encode_ldm(15),
                              i4::encode_hlt(), i4::encode_ldm(7),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 7u);
    }

    // ── JMS / BBL ────────────────────────────────────────────────────────────
    {
        auto jms = i4::encode_jms(0x004);
        auto s = run_program({jms.first, jms.second, i4::encode_hlt(), 0x00,
                              i4::encode_ldm(5), i4::encode_bbl(3)});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
    }
    {  // two levels of nesting
        Prog prog(256, 0);
        auto jms1 = i4::encode_jms(0x010);
        prog[0] = jms1.first;
        prog[1] = jms1.second;
        prog[2] = i4::encode_hlt();
        auto jms2 = i4::encode_jms(0x020);
        prog[0x010] = jms2.first;
        prog[0x011] = jms2.second;
        prog[0x012] = i4::encode_bbl(9);
        prog[0x020] = i4::encode_bbl(7);
        auto s = run_program(prog);
        ISO_CHECK_EQ_UINT(s.accumulator(), 9u);
    }

    // ── JCN ──────────────────────────────────────────────────────────────────
    {
        auto jcn = i4::encode_jcn(0x4, 0x05);
        auto s = run_program({jcn.first, jcn.second, i4::encode_ldm(15),
                              i4::encode_hlt(), 0x00, i4::encode_ldm(1),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 1u);
    }
    {
        auto jcn = i4::encode_jcn(0x4, 0x06);
        auto s = run_program({i4::encode_ldm(5), jcn.first, jcn.second,
                              i4::encode_ldm(2), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 2u);
    }
    {
        auto jcn = i4::encode_jcn(0xC, 0x05);
        auto s = run_program({i4::encode_ldm(5), jcn.first, jcn.second,
                              i4::encode_ldm(15), i4::encode_hlt(),
                              i4::encode_ldm(1), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 1u);
    }
    {
        auto jcn = i4::encode_jcn(0x2, 0x05);
        auto s = run_program({i4::encode_stc(), jcn.first, jcn.second,
                              i4::encode_ldm(15), i4::encode_hlt(),
                              i4::encode_ldm(1), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 1u);
    }

    // ── ISZ ──────────────────────────────────────────────────────────────────
    {
        auto isz = i4::encode_isz(0, 0x02);
        auto s = run_program({i4::encode_ldm(14), i4::encode_xch(0), isz.first,
                              isz.second, i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(0), 0u);
    }
    {
        auto isz = i4::encode_isz(0, 0x10);
        auto s = run_program({i4::encode_ldm(15), i4::encode_xch(0), isz.first,
                              isz.second, i4::encode_ldm(7), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(0), 0u);
        ISO_CHECK_EQ_UINT(s.accumulator(), 7u);
    }

    // ── FIM / register pairs ─────────────────────────────────────────────────
    {
        auto fim = i4::encode_fim(0, 0xA3);
        auto s = run_program({fim.first, fim.second, i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(0), 0xAu);
        ISO_CHECK_EQ_UINT(s.register_at(1), 0x3u);
    }
    {
        Prog prog;
        for (std::uint8_t p = 0; p < 8; ++p) {
            std::uint8_t val = static_cast<std::uint8_t>((p << 4) | (15 - p));
            auto fim = i4::encode_fim(p, val);
            prog.push_back(fim.first);
            prog.push_back(fim.second);
        }
        prog.push_back(i4::encode_hlt());
        auto s = run_program(prog);
        for (std::uint8_t p = 0; p < 8; ++p) {
            std::uint8_t val = static_cast<std::uint8_t>((p << 4) | (15 - p));
            ISO_CHECK_EQ_UINT(s.register_at(static_cast<std::size_t>(p) * 2),
                              (val >> 4) & 0xF);
            ISO_CHECK_EQ_UINT(s.register_at(static_cast<std::size_t>(p) * 2 + 1),
                              val & 0xF);
        }
    }
    {  // FIM P3, 0xDE -> R6=D, R7=E
        auto fim = i4::encode_fim(3, 0xDE);
        auto s = run_program({fim.first, fim.second, i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(6), 0xDu);
        ISO_CHECK_EQ_UINT(s.register_at(7), 0xEu);
    }

    // ── SRC / FIN / JIN ──────────────────────────────────────────────────────
    {
        auto fim = i4::encode_fim(0, 0x25);
        auto s = run_program(
            {fim.first, fim.second, i4::encode_src(0), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.ram_register(), 2u);
        ISO_CHECK_EQ_UINT(s.ram_character(), 5u);
    }
    {
        auto fim = i4::encode_fim(0, 0x08);
        Prog prog(9, 0);
        prog[0] = fim.first;
        prog[1] = fim.second;
        prog[2] = i4::encode_fin(1);
        prog[3] = i4::encode_hlt();
        prog[8] = 0xBC;
        auto s = run_program(prog);
        ISO_CHECK_EQ_UINT(s.register_at(2), 0xBu);
        ISO_CHECK_EQ_UINT(s.register_at(3), 0xCu);
    }
    {
        auto fim = i4::encode_fim(0, 0x05);
        auto s = run_program({fim.first, fim.second, i4::encode_jin(0),
                              i4::encode_ldm(15), i4::encode_hlt(),
                              i4::encode_ldm(3), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
    }

    // ── RAM: WRM/RDM, DCL banks ──────────────────────────────────────────────
    {
        auto fim = i4::encode_fim(0, 0x00);
        auto s = run_program({fim.first, fim.second, i4::encode_src(0),
                              i4::encode_ldm(7), i4::encode_wrm(),
                              i4::encode_ldm(0), i4::encode_rdm(),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 7u);
    }
    {
        auto fim = i4::encode_fim(0, 0x00);
        auto s = run_program(
            {fim.first, fim.second, i4::encode_src(0), i4::encode_ldm(0),
             i4::encode_dcl(), i4::encode_ldm(5), i4::encode_wrm(),
             i4::encode_ldm(2), i4::encode_dcl(), i4::encode_ldm(9),
             i4::encode_wrm(), i4::encode_ldm(0), i4::encode_dcl(),
             i4::encode_rdm(), i4::encode_xch(2), i4::encode_ldm(2),
             i4::encode_dcl(), i4::encode_rdm(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 9u);
        ISO_CHECK_EQ_UINT(s.register_at(2), 5u);
    }

    // ── RAM status: WR0-WR3 / RD0-RD3 ────────────────────────────────────────
    {
        auto fim = i4::encode_fim(0, 0x00);
        auto s = run_program(
            {fim.first, fim.second, i4::encode_src(0), i4::encode_ldm(1),
             i4::encode_wr0(), i4::encode_ldm(2), i4::encode_wr1(),
             i4::encode_ldm(3), i4::encode_wr2(), i4::encode_ldm(4),
             i4::encode_wr3(), i4::encode_rd0(), i4::encode_xch(4),
             i4::encode_rd1(), i4::encode_xch(5), i4::encode_rd2(),
             i4::encode_xch(6), i4::encode_rd3(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(4), 1u);
        ISO_CHECK_EQ_UINT(s.register_at(5), 2u);
        ISO_CHECK_EQ_UINT(s.register_at(6), 3u);
        ISO_CHECK_EQ_UINT(s.accumulator(), 4u);
    }

    // ── ROM port / WMP ───────────────────────────────────────────────────────
    {
        auto s = run_program({i4::encode_ldm(11), i4::encode_wrr(),
                              i4::encode_ldm(0), i4::encode_rdr(),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 11u);
    }
    {
        auto s = run_program({i4::encode_ldm(0), i4::encode_dcl(),
                              i4::encode_ldm(13), i4::encode_wmp(),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.ram_output(0), 13u);
    }

    // ── ADM / SBM ────────────────────────────────────────────────────────────
    {
        auto fim = i4::encode_fim(0, 0x00);
        auto s = run_program({fim.first, fim.second, i4::encode_src(0),
                              i4::encode_ldm(6), i4::encode_wrm(),
                              i4::encode_ldm(3), i4::encode_adm(),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 9u);
        ISO_CHECK(!s.carry());
    }
    {
        auto fim = i4::encode_fim(0, 0x00);
        auto s = run_program({fim.first, fim.second, i4::encode_src(0),
                              i4::encode_ldm(3), i4::encode_wrm(),
                              i4::encode_ldm(7), i4::encode_sbm(),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 4u);
        ISO_CHECK(s.carry());
    }

    // ── Accumulator group: CLB, CLC, IAC, CMC, CMA ───────────────────────────
    {
        auto s = run_program({i4::encode_ldm(15), i4::encode_stc(),
                              i4::encode_clb(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(7), i4::encode_stc(),
                              i4::encode_clc(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 7u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(4), i4::encode_iac(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 5u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(15), i4::encode_iac(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(s.carry());
    }
    {
        auto s = run_program({i4::encode_cmc(), i4::encode_hlt()});
        ISO_CHECK(s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_stc(), i4::encode_cmc(), i4::encode_hlt()});
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(5), i4::encode_cma(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 10u);
    }
    {
        auto s = run_program(
            {i4::encode_ldm(0), i4::encode_cma(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 15u);
    }

    // ── RAL / RAR ────────────────────────────────────────────────────────────
    {
        auto s = run_program(
            {i4::encode_ldm(5), i4::encode_ral(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 10u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(5), i4::encode_stc(),
                              i4::encode_ral(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 11u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(8), i4::encode_ral(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(6), i4::encode_rar(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(6), i4::encode_stc(),
                              i4::encode_rar(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 11u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(1), i4::encode_rar(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(s.carry());
    }

    // ── TCC / DAC / TCS / STC ────────────────────────────────────────────────
    {
        auto s = run_program(
            {i4::encode_stc(), i4::encode_tcc(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 1u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_tcc(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(5), i4::encode_dac(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 4u);
        ISO_CHECK(s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_ldm(0), i4::encode_dac(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 15u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program(
            {i4::encode_stc(), i4::encode_tcs(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 10u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_tcs(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 9u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_stc(), i4::encode_hlt()});
        ISO_CHECK(s.carry());
    }

    // ── DAA ──────────────────────────────────────────────────────────────────
    {
        auto s = run_program(
            {i4::encode_ldm(5), i4::encode_daa(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 5u);
        ISO_CHECK(!s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(5), i4::encode_xch(0),
                              i4::encode_ldm(8), i4::encode_add(0),
                              i4::encode_daa(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
        ISO_CHECK(s.carry());
    }
    {
        auto s = run_program({i4::encode_ldm(2), i4::encode_stc(),
                              i4::encode_daa(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 8u);
        ISO_CHECK(s.carry());
    }

    // ── KBP (exhaustive one-hot decode) ──────────────────────────────────────
    {
        const std::uint8_t expected[16] = {0, 1, 2, 15, 3,  15, 15, 15,
                                           4, 15, 15, 15, 15, 15, 15, 15};
        for (std::uint8_t input = 0; input < 16; ++input) {
            auto s = run_program(
                {i4::encode_ldm(input), i4::encode_kbp(), i4::encode_hlt()});
            ISO_CHECK_EQ_UINT(s.accumulator(), expected[input]);
        }
    }

    // ── DCL bank selection ───────────────────────────────────────────────────
    for (std::uint8_t bank = 0; bank < 4; ++bank) {
        auto s = run_program(
            {i4::encode_ldm(bank), i4::encode_dcl(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.ram_bank(), bank);
    }
    {
        auto s = run_program(
            {i4::encode_ldm(7), i4::encode_dcl(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.ram_bank(), 3u);
    }

    // ── WPM is a no-op ───────────────────────────────────────────────────────
    {
        auto s = run_program(
            {i4::encode_ldm(5), i4::encode_wpm(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 5u);
    }

    // ── reset clears state ───────────────────────────────────────────────────
    {
        auto fim = i4::encode_fim(0, 0x25);
        i4::Simulator s(4096);
        s.run({i4::encode_ldm(15), i4::encode_xch(0), i4::encode_stc(),
               fim.first, fim.second, i4::encode_src(0), i4::encode_ldm(5),
               i4::encode_wrm(), i4::encode_hlt()},
              100);
        ISO_CHECK(s.register_at(0) != 0 || s.carry() || s.ram(0, 2, 5) != 0);
        s.reset();
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(!s.carry());
        ISO_CHECK_EQ_UINT(s.register_at(0), 0u);
        ISO_CHECK_EQ_UINT(s.pc(), 0u);
        ISO_CHECK(!s.halted());
        ISO_CHECK_EQ_UINT(s.hw_stack(0), 0u);
        ISO_CHECK_EQ_UINT(s.stack_pointer(), 0u);
        ISO_CHECK_EQ_UINT(s.ram(0, 2, 5), 0u);
        ISO_CHECK_EQ_UINT(s.ram_bank(), 0u);
        ISO_CHECK_EQ_UINT(s.ram_register(), 0u);
        ISO_CHECK_EQ_UINT(s.ram_character(), 0u);
        ISO_CHECK_EQ_UINT(s.rom_port(), 0u);
    }

    // ── trace raw2 / single-byte / unknown ───────────────────────────────────
    {
        auto jun = i4::encode_jun(0x004);
        i4::Simulator s;
        s.load_program({jun.first, jun.second, 0, 0, i4::encode_hlt()});
        auto t = s.step();
        ISO_CHECK_EQ_UINT(t.raw, jun.first);
        ISO_CHECK(t.raw2.has_value());
        ISO_CHECK_EQ_UINT(t.raw2.value(), jun.second);
    }
    {
        i4::Simulator s;
        s.load_program({i4::encode_ldm(5), i4::encode_hlt()});
        auto t = s.step();
        ISO_CHECK(!t.raw2.has_value());
    }
    {
        i4::Simulator s;
        s.load_program({0xFE, i4::encode_hlt()});
        auto t = s.step();
        ISO_CHECK(t.mnemonic.find("UNKNOWN") != std::string::npos);
    }
    {  // SRC (0x2 odd) is single-byte
        i4::Simulator s;
        s.load_program({i4::encode_src(0), i4::encode_hlt()});
        auto t = s.step();
        ISO_CHECK(!t.raw2.has_value());
    }

    // ── End-to-end programs ──────────────────────────────────────────────────
    {  // x = 1 + 2 stored in R1
        auto s = run_program({i4::encode_ldm(1), i4::encode_xch(0),
                              i4::encode_ldm(2), i4::encode_add(0),
                              i4::encode_xch(1), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.register_at(1), 3u);
    }
    {  // countdown 5 -> 0
        auto jcn = i4::encode_jcn(0xC, 0x01);
        auto s = run_program({i4::encode_ldm(5), i4::encode_dac(), jcn.first,
                              jcn.second, i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
    }
    {  // BCD add 8 + 5 = 13
        auto s = run_program({i4::encode_ldm(5), i4::encode_xch(0),
                              i4::encode_ldm(8), i4::encode_clc(),
                              i4::encode_add(0), i4::encode_daa(),
                              i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
        ISO_CHECK(s.carry());
    }
    {  // doubling subroutine
        Prog prog(256, 0);
        prog[0] = i4::encode_ldm(6);
        prog[1] = i4::encode_xch(0);
        auto jms = i4::encode_jms(0x010);
        prog[2] = jms.first;
        prog[3] = jms.second;
        prog[4] = i4::encode_ld(1);
        prog[5] = i4::encode_hlt();
        prog[0x010] = i4::encode_ld(0);
        prog[0x011] = i4::encode_add(0);
        prog[0x012] = i4::encode_xch(1);
        prog[0x013] = i4::encode_bbl(0);
        auto s = run_program(prog);
        ISO_CHECK_EQ_UINT(s.accumulator(), 12u);
    }
    {  // RAM array: store 1,3,5; read back char 1 == 3
        Prog prog;
        const std::uint8_t values[3] = {1, 3, 5};
        for (std::uint8_t i = 0; i < 3; ++i) {
            auto fim = i4::encode_fim(0, i);
            prog.push_back(fim.first);
            prog.push_back(fim.second);
            prog.push_back(i4::encode_src(0));
            prog.push_back(i4::encode_ldm(values[i]));
            prog.push_back(i4::encode_wrm());
        }
        auto fim = i4::encode_fim(0, 1);
        prog.push_back(fim.first);
        prog.push_back(fim.second);
        prog.push_back(i4::encode_src(0));
        prog.push_back(i4::encode_rdm());
        prog.push_back(i4::encode_hlt());
        auto s = run_program(prog);
        ISO_CHECK_EQ_UINT(s.accumulator(), 3u);
    }
    {  // ISZ loop summing 1+2+3 = 6
        auto fim = i4::encode_fim(0, static_cast<std::uint8_t>(13 << 4));
        auto isz = i4::encode_isz(0, 0x04);
        auto s = run_program({fim.first, fim.second, i4::encode_ldm(0),
                              i4::encode_xch(2), i4::encode_inc(1),
                              i4::encode_ld(2), i4::encode_add(1),
                              i4::encode_xch(2), isz.first, isz.second,
                              i4::encode_ld(2), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 6u);
    }
    {  // rotate left twice: 1 -> 2 -> 4
        auto s = run_program({i4::encode_ldm(1), i4::encode_ral(),
                              i4::encode_ral(), i4::encode_hlt()});
        ISO_CHECK_EQ_UINT(s.accumulator(), 4u);
        ISO_CHECK(!s.carry());
    }
    {  // stack wrapping: nest 4 deep in a 3-slot stack; must not crash
        Prog prog(256, 0);
        auto j1 = i4::encode_jms(0x20);
        prog[0x10] = j1.first;
        prog[0x11] = j1.second;
        prog[0x12] = i4::encode_bbl(0);
        auto j2 = i4::encode_jms(0x30);
        prog[0x20] = j2.first;
        prog[0x21] = j2.second;
        prog[0x22] = i4::encode_bbl(0);
        auto j3 = i4::encode_jms(0x40);
        prog[0x30] = j3.first;
        prog[0x31] = j3.second;
        prog[0x32] = i4::encode_bbl(0);
        prog[0x40] = i4::encode_bbl(0);
        auto j0 = i4::encode_jms(0x10);
        prog[0x00] = j0.first;
        prog[0x01] = j0.second;
        prog[0x02] = i4::encode_hlt();
        i4::Simulator s(4096);
        s.run(prog, 100);
        ISO_CHECK(true);  // survived without out-of-bounds access
    }

    // ── Encoding roundtrip ───────────────────────────────────────────────────
    {
        ISO_CHECK_EQ_UINT(i4::encode_nop(), 0x00u);
        ISO_CHECK_EQ_UINT(i4::encode_hlt(), 0x01u);
        ISO_CHECK_EQ_UINT(i4::encode_ldm(5), 0xD5u);
        ISO_CHECK_EQ_UINT(i4::encode_ld(3), 0xA3u);
        ISO_CHECK_EQ_UINT(i4::encode_xch(7), 0xB7u);
        ISO_CHECK_EQ_UINT(i4::encode_add(2), 0x82u);
        ISO_CHECK_EQ_UINT(i4::encode_sub(4), 0x94u);
        ISO_CHECK_EQ_UINT(i4::encode_inc(6), 0x66u);
        ISO_CHECK_EQ_UINT(i4::encode_bbl(1), 0xC1u);
        auto jcn = i4::encode_jcn(0x4, 0x10);
        ISO_CHECK_EQ_UINT(jcn.first, 0x14u);
        ISO_CHECK_EQ_UINT(jcn.second, 0x10u);
        auto fim = i4::encode_fim(2, 0xAB);
        ISO_CHECK_EQ_UINT(fim.first, 0x24u);
        ISO_CHECK_EQ_UINT(fim.second, 0xABu);
        ISO_CHECK_EQ_UINT(i4::encode_src(1), 0x23u);
        ISO_CHECK_EQ_UINT(i4::encode_fin(3), 0x36u);
        ISO_CHECK_EQ_UINT(i4::encode_jin(3), 0x37u);
        auto jun = i4::encode_jun(0x123);
        ISO_CHECK_EQ_UINT(jun.first, 0x41u);
        ISO_CHECK_EQ_UINT(jun.second, 0x23u);
        auto jms = i4::encode_jms(0x456);
        ISO_CHECK_EQ_UINT(jms.first, 0x54u);
        ISO_CHECK_EQ_UINT(jms.second, 0x56u);
        auto isz = i4::encode_isz(5, 0x20);
        ISO_CHECK_EQ_UINT(isz.first, 0x75u);
        ISO_CHECK_EQ_UINT(isz.second, 0x20u);
        ISO_CHECK_EQ_UINT(i4::encode_wrm(), 0xE0u);
        ISO_CHECK_EQ_UINT(i4::encode_wmp(), 0xE1u);
        ISO_CHECK_EQ_UINT(i4::encode_wrr(), 0xE2u);
        ISO_CHECK_EQ_UINT(i4::encode_wpm(), 0xE3u);
        ISO_CHECK_EQ_UINT(i4::encode_wr0(), 0xE4u);
        ISO_CHECK_EQ_UINT(i4::encode_wr1(), 0xE5u);
        ISO_CHECK_EQ_UINT(i4::encode_wr2(), 0xE6u);
        ISO_CHECK_EQ_UINT(i4::encode_wr3(), 0xE7u);
        ISO_CHECK_EQ_UINT(i4::encode_sbm(), 0xE8u);
        ISO_CHECK_EQ_UINT(i4::encode_rdm(), 0xE9u);
        ISO_CHECK_EQ_UINT(i4::encode_rdr(), 0xEAu);
        ISO_CHECK_EQ_UINT(i4::encode_adm(), 0xEBu);
        ISO_CHECK_EQ_UINT(i4::encode_rd0(), 0xECu);
        ISO_CHECK_EQ_UINT(i4::encode_rd1(), 0xEDu);
        ISO_CHECK_EQ_UINT(i4::encode_rd2(), 0xEEu);
        ISO_CHECK_EQ_UINT(i4::encode_rd3(), 0xEFu);
        ISO_CHECK_EQ_UINT(i4::encode_clb(), 0xF0u);
        ISO_CHECK_EQ_UINT(i4::encode_clc(), 0xF1u);
        ISO_CHECK_EQ_UINT(i4::encode_iac(), 0xF2u);
        ISO_CHECK_EQ_UINT(i4::encode_cmc(), 0xF3u);
        ISO_CHECK_EQ_UINT(i4::encode_cma(), 0xF4u);
        ISO_CHECK_EQ_UINT(i4::encode_ral(), 0xF5u);
        ISO_CHECK_EQ_UINT(i4::encode_rar(), 0xF6u);
        ISO_CHECK_EQ_UINT(i4::encode_tcc(), 0xF7u);
        ISO_CHECK_EQ_UINT(i4::encode_dac(), 0xF8u);
        ISO_CHECK_EQ_UINT(i4::encode_tcs(), 0xF9u);
        ISO_CHECK_EQ_UINT(i4::encode_stc(), 0xFAu);
        ISO_CHECK_EQ_UINT(i4::encode_daa(), 0xFBu);
        ISO_CHECK_EQ_UINT(i4::encode_kbp(), 0xFCu);
        ISO_CHECK_EQ_UINT(i4::encode_dcl(), 0xFDu);
    }

    // ── run() resets between programs ─────────────────────────────────────────
    {
        i4::Simulator s(4096);
        s.run({i4::encode_ldm(15), i4::encode_stc(), i4::encode_hlt()}, 10);
        ISO_CHECK_EQ_UINT(s.accumulator(), 15u);
        ISO_CHECK(s.carry());
        s.run({i4::encode_hlt()}, 10);
        ISO_CHECK_EQ_UINT(s.accumulator(), 0u);
        ISO_CHECK(!s.carry());
    }

    return ISO_TEST_RESULT();
}
