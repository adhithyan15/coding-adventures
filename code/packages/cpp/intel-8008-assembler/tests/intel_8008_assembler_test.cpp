// Tests for intel-8008-assembler, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "intel_8008_assembler.hpp"

namespace a8 = ca::intel8008_assembler;
using Bytes = std::vector<std::uint8_t>;
using Ops = std::vector<std::string>;

static Bytes enc(const std::string& m, const Ops& ops, const a8::Symbols& s,
                 std::size_t pc) {
    return a8::encode_instruction(m, ops, s, pc);
}

int main() {
    a8::Symbols empty;

    // ── instruction sizes ──────────────────────────────────────────────────
    {
        const char* one[] = {"HLT", "RFC", "RET", "RLC", "RRC", "RAL",
                             "RAR", "RFZ", "RTC", "ADD", "ADC", "SUB",
                             "SBB", "ANA", "XRA", "ORA", "CMP"};
        for (const char* m : one) {
            ISO_CHECK_EQ_UINT(a8::instruction_size(m), 1u);
        }
        const char* two[] = {"MVI", "ADI", "ACI", "SUI", "SBI",
                             "ANI", "XRI", "ORI", "CPI"};
        for (const char* m : two) {
            ISO_CHECK_EQ_UINT(a8::instruction_size(m), 2u);
        }
        const char* three[] = {"JMP", "CAL", "JFC", "JTC", "JFZ", "JTZ",
                               "JFP", "JTP", "CFC", "CTC", "CFZ", "CTZ"};
        for (const char* m : three) {
            ISO_CHECK_EQ_UINT(a8::instruction_size(m), 3u);
        }
        ISO_CHECK_EQ_UINT(a8::instruction_size("ORG"), 0u);
        bool threw = false;
        try {
            a8::instruction_size("BOGUS");
        } catch (const a8::AssemblerError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── encoder vectors ────────────────────────────────────────────────────
    {
        ISO_CHECK((enc("HLT", {}, empty, 0) == Bytes{0xFF}));
        ISO_CHECK((enc("RFC", {}, empty, 0) == enc("RET", {}, empty, 0)));
        ISO_CHECK((enc("RFC", {}, empty, 0) == Bytes{0x03}));
        ISO_CHECK((enc("MVI", {"B", "42"}, empty, 0) == Bytes{0x06, 0x2A}));
        ISO_CHECK((enc("MVI", {"H", "0x20"}, empty, 0) == Bytes{0x26, 0x20}));
        ISO_CHECK((enc("MOV", {"A", "B"}, empty, 0) == Bytes{0x78}));
        ISO_CHECK((enc("ADD", {"C"}, empty, 0) == Bytes{0x81}));
        ISO_CHECK((enc("JMP", {"0x000A"}, empty, 0) == Bytes{0x7C, 0x0A, 0x00}));
        ISO_CHECK((enc("CAL", {"0x0100"}, empty, 0) == Bytes{0x7E, 0x00, 0x01}));
        ISO_CHECK((enc("ADI", {"5"}, empty, 0) == Bytes{0xC4, 0x05}));
        ISO_CHECK((enc("IN", {"2"}, empty, 0) == Bytes{0x51}));
        ISO_CHECK((enc("OUT", {"17"}, empty, 0) == Bytes{0x22}));
        ISO_CHECK((enc("INR", {"D"}, empty, 0) == Bytes{0x10}));
        ISO_CHECK((enc("DCR", {"C"}, empty, 0) == Bytes{0x09}));
        ISO_CHECK((enc("RST", {"3"}, empty, 0) == Bytes{0x1D}));
    }

    // ── label / hi / lo resolution ─────────────────────────────────────────
    {
        a8::Symbols s;
        s["loop_end"] = 0x0010;
        ISO_CHECK((enc("JTZ", {"loop_end"}, s, 0) == Bytes{0x4C, 0x10, 0x00}));

        a8::Symbols c;
        c["counter"] = 0x2000;
        ISO_CHECK((enc("MVI", {"H", "hi(counter)"}, c, 0) ==
                   Bytes{0x26, 0x20}));
        ISO_CHECK((enc("MVI", {"L", "lo(counter)"}, c, 0) ==
                   Bytes{0x2E, 0x00}));
    }

    // ── full two-pass assembly ─────────────────────────────────────────────
    {
        ISO_CHECK((a8::assemble("    ORG 0x0000\n_start:\n    HLT\n") ==
                   Bytes{0xFF}));
        ISO_CHECK(
            (a8::assemble("    ORG 0x0000\n_start:\n    MVI  B, 0\n    HLT\n") ==
             Bytes{0x06, 0x00, 0xFF}));

        // forward reference
        std::string fwd =
            "\n            ORG 0x0000\n        _start:\n            JMP "
            "loop_end\n        loop_end:\n            HLT\n        ";
        ISO_CHECK((a8::assemble(fwd) == Bytes{0x7C, 0x03, 0x00, 0xFF}));

        // $ resolves to PC
        ISO_CHECK((a8::assemble("    ORG 0x0000\n    JMP $\n") ==
                   Bytes{0x7C, 0x00, 0x00}));

        // ORG padding with 0xFF
        std::string padsrc =
            "\n            ORG 0x0000\n            MVI  B, 1\n            ORG "
            "0x0005\n            HLT\n        ";
        Bytes pad = a8::assemble(padsrc);
        ISO_CHECK_EQ_UINT(pad.size(), 6u);
        ISO_CHECK(pad[0] == 0x06 && pad[1] == 0x01);
        ISO_CHECK(pad[2] == 0xFF && pad[3] == 0xFF && pad[4] == 0xFF);
        ISO_CHECK(pad[5] == 0xFF);
    }

    // ── error paths ────────────────────────────────────────────────────────
    {
        bool threw;
        threw = false;
        try {
            a8::assemble("    BOGUS\n");
        } catch (const a8::AssemblerError& e) {
            threw = true;
            ISO_CHECK(std::string(e.what()).find("BOGUS") != std::string::npos);
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            a8::assemble("    JMP undefined_label\n");
        } catch (const a8::AssemblerError&) {
            threw = true;
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            a8::assemble("    MVI B, 256\n");
        } catch (const a8::AssemblerError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
