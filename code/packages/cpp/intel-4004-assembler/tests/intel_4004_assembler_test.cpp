// Tests for the C++ intel-4004-assembler, using the header-only iso_test.h
// harness (pure ISO). Reference vector from the Rust crate; the rest are
// hand-computed from the encoding table.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "intel_4004_assembler.hpp"

namespace i4004 = ca::intel4004;
using Bytes = std::vector<std::uint8_t>;

static bool asm_fails(const std::string& text) {
    try {
        (void)i4004::assemble(text);
    } catch (const i4004::AssemblerError&) {
        return true;
    }
    return false;
}

int main() {
    // ── Rust reference vector ────────────────────────────────────────────
    ISO_CHECK(i4004::assemble("ORG 0x000\nLDM 5\nXCH R2\nHLT\n") ==
              Bytes({0xD5, 0xB2, 0x01}));

    // ── one-byte instructions ────────────────────────────────────────────
    ISO_CHECK(i4004::assemble("NOP\nHLT\nWRM\nINC R3\nADD R1\nSUB R4\nLD R5\n"
                              "XCH R2\nBBL 0\nLDM 5\n") ==
              Bytes({0x00, 0x01, 0xE0, 0x63, 0x81, 0x94, 0xA5, 0xB2, 0xC0,
                     0xD5}));

    // ── register pairs ───────────────────────────────────────────────────
    ISO_CHECK(i4004::assemble("SRC P1\nFIN P2\nJIN P0\n") ==
              Bytes({0x23, 0x34, 0x31}));

    // ── two-byte instructions ────────────────────────────────────────────
    ISO_CHECK(i4004::assemble("FIM P0, 0xAB\n") == Bytes({0x20, 0xAB}));
    ISO_CHECK(i4004::assemble("JCN 0x2, 0x10\n") == Bytes({0x12, 0x10}));
    ISO_CHECK(i4004::assemble("JUN 0x123\n") == Bytes({0x41, 0x23}));
    ISO_CHECK(i4004::assemble("JMS 0x234\n") == Bytes({0x52, 0x34}));
    ISO_CHECK(i4004::assemble("ISZ R3, 0x40\n") == Bytes({0x73, 0x40}));
    ISO_CHECK(i4004::assemble("ADD_IMM acc, R2, 5\n") == Bytes({0xD5, 0x82}));

    // ── labels ───────────────────────────────────────────────────────────
    ISO_CHECK(i4004::assemble("ORG 0x000\nstart: LDM 5\nJUN start\n") ==
              Bytes({0xD5, 0x40, 0x00}));

    // ── forward ORG pads with zeros ──────────────────────────────────────
    ISO_CHECK(i4004::assemble("ORG 3\nHLT\n") ==
              Bytes({0x00, 0x00, 0x00, 0x01}));

    // ── comments / blank lines ───────────────────────────────────────────
    ISO_CHECK(i4004::assemble("  ; a comment\n\nHLT   ; inline\n") ==
              Bytes({0x01}));

    // ── error cases ──────────────────────────────────────────────────────
    ISO_CHECK(asm_fails("FOO\n"));
    ISO_CHECK(asm_fails("LDM\n"));
    ISO_CHECK(asm_fails("LDM missing\n"));
    ISO_CHECK(asm_fails("INC Rx\n"));
    ISO_CHECK(asm_fails("JUN 0x10000\n"));
    ISO_CHECK(asm_fails("ORG\n"));

    return ISO_TEST_RESULT();
}
