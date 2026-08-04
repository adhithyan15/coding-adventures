// Tests for intel8008-simulator, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "intel8008_simulator.hpp"

namespace i8 = ca::intel8008_simulator;
using Prog = std::vector<std::uint8_t>;

int main() {
    // ── basic arithmetic ───────────────────────────────────────────────────
    {
        i8::Simulator s;
        auto tr = s.run({0x06, 0x01, 0x3E, 0x02, 0x80, 0x76}, 100);
        ISO_CHECK_EQ_UINT(tr.size(), 4u);
        ISO_CHECK_EQ_UINT(s.a(), 3u);
        ISO_CHECK(!s.flags().carry && !s.flags().zero && !s.flags().sign);
        ISO_CHECK(s.flags().parity);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0xFF, 0xC4, 0x01, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x00u);
        ISO_CHECK(s.flags().carry && s.flags().zero && !s.flags().sign &&
                  s.flags().parity);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x00, 0xD4, 0x01, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0xFFu);
        ISO_CHECK(s.flags().carry && s.flags().sign && !s.flags().zero &&
                  s.flags().parity);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0xFE, 0xC4, 0x01, 0xCC, 0x01, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x00u);
        ISO_CHECK(s.flags().carry && s.flags().zero);
    }

    // ── logical ────────────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.run({0x3E, 0xFF, 0xC4, 0x01, 0xA7, 0x76}, 100);
        ISO_CHECK(!s.flags().carry);
        ISO_CHECK_EQ_UINT(s.a(), 0x00u);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0xAB, 0xAF, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x00u);
        ISO_CHECK(s.flags().zero && !s.flags().carry && s.flags().parity);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x0F, 0x06, 0xF0, 0xB0, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0xFFu);
        ISO_CHECK(!s.flags().carry && s.flags().parity);
    }

    // ── INR / DCR ──────────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.run({0x3E, 0xFF, 0xC4, 0x01, 0x06, 0xFF, 0x00, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.b(), 0x00u);
        ISO_CHECK(s.flags().zero && s.flags().carry);  // INR preserves carry
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x00, 0x39, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0xFFu);
        ISO_CHECK(s.flags().sign && !s.flags().zero && s.flags().parity);
    }

    // ── rotates ────────────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.run({0x3E, 0x80, 0x02, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x01u);
        ISO_CHECK(s.flags().carry);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x01, 0x0A, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x80u);
        ISO_CHECK(s.flags().carry);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0xFF, 0x12, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0xFEu);
        ISO_CHECK(s.flags().carry);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0xFF, 0xC4, 0x01, 0x3E, 0x01, 0x1A, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x80u);
        ISO_CHECK(s.flags().carry);
    }

    // ── stack: call / return, nesting, RST ─────────────────────────────────
    {
        i8::Simulator s;
        Prog p(0x14, 0);
        p[0] = 0x3E; p[1] = 0x00;
        p[2] = 0x7E; p[3] = 0x10; p[4] = 0x00;
        p[5] = 0x76;
        p[0x10] = 0x3E; p[0x11] = 0x2A; p[0x12] = 0x3F;
        s.run(p, 100);
        ISO_CHECK_EQ_UINT(s.a(), 42u);
    }
    {
        i8::Simulator s;
        Prog p(0x50, 0);
        p[0] = 0x3E; p[1] = 0x00;
        p[2] = 0x7E; p[3] = 0x20; p[4] = 0x00; p[5] = 0x76;
        p[0x20] = 0x7E; p[0x21] = 0x40; p[0x22] = 0x00; p[0x23] = 0x3F;
        p[0x40] = 0x3E; p[0x41] = 99; p[0x42] = 0x3F;
        s.run(p, 200);
        ISO_CHECK_EQ_UINT(s.a(), 99u);
    }
    {
        i8::Simulator s;
        Prog p(0x20, 0);
        p[0] = 0x1D; p[1] = 0x76;
        p[0x18] = 0x3E; p[0x19] = 77; p[0x1A] = 0x3F;
        s.run(p, 100);
        ISO_CHECK_EQ_UINT(s.a(), 77u);
    }

    // ── memory via M ───────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.run({0x26, 0x00, 0x2E, 0x20, 0x36, 0x55, 0x6E, 0x7D, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x55u);
    }

    // ── conditional jumps ──────────────────────────────────────────────────
    {
        i8::Simulator s;
        Prog p(0x14, 0);
        p[0] = 0x3E; p[1] = 0x00; p[2] = 0xC4; p[3] = 0x00;
        p[4] = 0x4C; p[5] = 0x10; p[6] = 0x00;
        p[7] = 0x3E; p[8] = 99; p[9] = 0x76;
        p[0x10] = 0x3E; p[0x11] = 42; p[0x12] = 0x76;
        s.run(p, 100);
        ISO_CHECK_EQ_UINT(s.a(), 42u);
    }
    {
        i8::Simulator s;
        Prog p(0x14, 0);
        p[0] = 0x3E; p[1] = 0x01; p[2] = 0x4C; p[3] = 0x10; p[4] = 0x00;
        p[5] = 0x3E; p[6] = 99; p[7] = 0x76;
        p[0x10] = 0x3E; p[0x11] = 42; p[0x12] = 0x76;
        s.run(p, 100);
        ISO_CHECK_EQ_UINT(s.a(), 99u);
    }

    // ── parity ─────────────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.run({0x3E, 0x03, 0xF4, 0x00, 0x76}, 100);
        ISO_CHECK(s.flags().parity);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x01, 0xF4, 0x00, 0x76}, 100);
        ISO_CHECK(!s.flags().parity);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0xFF, 0xF4, 0x00, 0x76}, 100);
        ISO_CHECK(s.flags().parity);
    }

    // ── CMP ────────────────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.run({0x3E, 0x05, 0xFC, 0x05, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 5u);
        ISO_CHECK(s.flags().zero && !s.flags().carry);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x03, 0xFC, 0x05, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 3u);
        ISO_CHECK(s.flags().carry && !s.flags().zero);
    }

    // ── I/O ports ──────────────────────────────────────────────────────────
    {
        i8::Simulator s;
        s.set_input_port(3, 0xAB);
        s.run({0x59, 0x76}, 10);
        ISO_CHECK_EQ_UINT(s.a(), 0xABu);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x77, 0x22, 0x76}, 10);
        ISO_CHECK_EQ_UINT(s.get_output_port(17), 0x77u);
    }

    // ── abs-value subroutine / multiply loop / sbb / mov ───────────────────
    {
        i8::Simulator s;
        Prog p(0x40, 0);
        p[0] = 0x3E; p[1] = 0xF6; p[2] = 0xF4; p[3] = 0x00;
        p[4] = 0x7E; p[5] = 0x20; p[6] = 0x00; p[7] = 0x76;
        p[0x20] = 0x50; p[0x21] = 0x30; p[0x22] = 0x00;
        p[0x23] = 0xEC; p[0x24] = 0xFF; p[0x25] = 0xC4; p[0x26] = 0x01;
        p[0x30] = 0x3F;
        s.run(p, 200);
        ISO_CHECK_EQ_UINT(s.a(), 10u);
    }
    {
        i8::Simulator s;
        Prog p(20, 0);
        p[0] = 0x06; p[1] = 0x05; p[2] = 0x0E; p[3] = 0x04;
        p[4] = 0x3E; p[5] = 0x00; p[6] = 0x80; p[7] = 0x09;
        p[8] = 0x48; p[9] = 0x06; p[10] = 0x00; p[11] = 0x76;
        s.run(p, 200);
        ISO_CHECK_EQ_UINT(s.a(), 20u);
    }
    {
        i8::Simulator s;
        s.run({0x3E, 0x05, 0xC4, 0xFF, 0xDC, 0x01, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 2u);
    }
    {
        i8::Simulator s;
        s.run({0x06, 0x42, 0x78, 0x76}, 100);
        ISO_CHECK_EQ_UINT(s.a(), 0x42u);
    }

    // ── trace contents ─────────────────────────────────────────────────────
    {
        i8::Simulator s;
        auto tr = s.run({0x3E, 0x05, 0x76}, 100);
        ISO_CHECK_EQ_UINT(tr.size(), 2u);
        ISO_CHECK_EQ_UINT(tr[0].address, 0u);
        ISO_CHECK_EQ_UINT(tr[0].a_before, 0u);
        ISO_CHECK_EQ_UINT(tr[0].a_after, 5u);
        ISO_CHECK((tr[0].raw == Prog{0x3E, 0x05}));
        ISO_CHECK_EQ_UINT(tr[1].address, 2u);
        ISO_CHECK(tr[1].mnemonic.find("HLT") != std::string::npos);
    }

    return ISO_TEST_RESULT();
}
