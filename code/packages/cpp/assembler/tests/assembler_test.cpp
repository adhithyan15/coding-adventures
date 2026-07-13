// Tests for the C++ assembler library, using the header-only iso_test.h harness
// (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>
#include <variant>
#include <vector>

#include "assembler.hpp"

namespace asmr = ca::assembler;
using asmr::Assembler;
using asmr::ArmOpcode;

// True iff parsing `src` throws an AssemblerError whose message contains `kw`.
static bool throws_with(const std::string& src, const char* kw) {
    Assembler a;
    try {
        a.parse(src);
        return false;
    } catch (const asmr::AssemblerError& e) {
        return std::string(e.what()).find(kw) != std::string::npos;
    }
}

// Encode `src` (one instruction word) and return that word.
static std::uint32_t encode_one(const std::string& src) {
    Assembler a;
    auto b = a.encode(a.parse(src));
    return b.empty() ? 0 : b[0];
}

int main() {
    // ── register / immediate helpers ───────────────────────────────────────
    ISO_CHECK(asmr::detail::parse_register("R0") == 0u);
    ISO_CHECK(asmr::detail::parse_register("R15") == 15u);
    ISO_CHECK(asmr::detail::parse_register("SP") == 13u);
    ISO_CHECK(asmr::detail::parse_register("LR") == 14u);
    ISO_CHECK(asmr::detail::parse_register("PC") == 15u);
    ISO_CHECK(!asmr::detail::parse_register("R16").has_value());
    ISO_CHECK(!asmr::detail::parse_register("X0").has_value());
    ISO_CHECK(asmr::detail::parse_register("r0") == 0u);
    ISO_CHECK(asmr::detail::parse_register("sp") == 13u);

    ISO_CHECK(asmr::detail::parse_immediate("#42") == 42u);
    ISO_CHECK(asmr::detail::parse_immediate("#0") == 0u);
    ISO_CHECK(asmr::detail::parse_immediate("#255") == 255u);
    ISO_CHECK(asmr::detail::parse_immediate("#0xFF") == 255u);
    ISO_CHECK(asmr::detail::parse_immediate("#0x10") == 16u);
    ISO_CHECK(asmr::detail::parse_immediate("42") == 42u);  // bare number allowed here

    // ── instruction parsing ────────────────────────────────────────────────
    {
        Assembler a;
        auto ins = a.parse("MOV R0, #42");
        ISO_CHECK(ins.size() == 1);
        const auto& dp = std::get<asmr::DataProcessing>(ins[0]);
        ISO_CHECK(dp.opcode == ArmOpcode::Mov && dp.rd == 0u);
        ISO_CHECK((dp.operand2 == asmr::Operand2::imm(42)));
    }
    {
        Assembler a;
        auto ins = a.parse("ADD R2, R0, R1");
        const auto& dp = std::get<asmr::DataProcessing>(ins[0]);
        ISO_CHECK(dp.opcode == ArmOpcode::Add && dp.rd == 2u && dp.rn == 0u);
        ISO_CHECK((dp.operand2 == asmr::Operand2::reg(1)));
    }
    {
        Assembler a;
        auto ins = a.parse("SUB R3, R1, R2");
        ISO_CHECK(std::get<asmr::DataProcessing>(ins[0]).opcode == ArmOpcode::Sub);
    }
    {
        Assembler a;
        auto ins = a.parse("CMP R0, R1");
        const auto& dp = std::get<asmr::DataProcessing>(ins[0]);
        ISO_CHECK(dp.opcode == ArmOpcode::Cmp && !dp.rd.has_value() && dp.set_flags);
    }
    {
        Assembler a;
        auto li = a.parse("LDR R0, [R1]");
        const auto& ld = std::get<asmr::Load>(li[0]);
        ISO_CHECK(ld.rd == 0u && ld.rn == 1u);
        auto si = a.parse("STR R0, [R1]");
        const auto& st = std::get<asmr::Store>(si[0]);
        ISO_CHECK(st.rd == 0u && st.rn == 1u);
    }
    {
        Assembler a;
        ISO_CHECK(std::holds_alternative<asmr::Nop>(a.parse("NOP")[0]));
    }
    {
        Assembler a;
        auto ins = a.parse("loop:");
        ISO_CHECK(std::get<asmr::Label>(ins[0]).name == "loop");
        ISO_CHECK(a.labels.at("loop") == 0u);
    }
    {
        Assembler a;
        ISO_CHECK(a.parse("MOV R0, #1 ; load one").size() == 1);
        ISO_CHECK(a.parse("\n\nMOV R0, #1\n\n").size() == 1);
        ISO_CHECK(a.parse("MOV R0, #10\nMOV R1, #20\nADD R2, R0, R1").size() == 3);
    }

    // ── binary encoding (exact words) ──────────────────────────────────────
    ISO_CHECK(encode_one("MOV R0, #42") == 0xE3A0002Au);
    ISO_CHECK(encode_one("ADD R2, R0, R1") == 0xE0802001u);
    ISO_CHECK(encode_one("NOP") == 0xE1A00000u);
    ISO_CHECK(encode_one("LDR R0, [R1]") == 0xE5910000u);
    ISO_CHECK(encode_one("STR R0, [R1]") == 0xE5810000u);
    ISO_CHECK(((encode_one("LDR R0, [R1]") >> 20) & 0x1) == 1);
    ISO_CHECK(((encode_one("STR R0, [R1]") >> 20) & 0x1) == 0);

    // labels produce no binary; full program length
    {
        Assembler a;
        ISO_CHECK(a.encode(a.parse("start:\nMOV R0, #1")).size() == 1);
        ISO_CHECK(a.encode(a.parse("MOV R0, #10\nMOV R1, #20\nADD R2, R0, R1\nSTR R2, [R3]")).size() == 4);
    }

    // ── errors ─────────────────────────────────────────────────────────────
    ISO_CHECK(throws_with("BLAH R0, R1", "Unknown mnemonic: BLAH"));
    ISO_CHECK(throws_with("MOV X0, #1", "Invalid register"));
    ISO_CHECK(throws_with("ADD R0, R1", "expected 3 operands, got 2"));
    ISO_CHECK((asmr::AssemblerError::unknown_mnemonic("BLAH").what() ==
               std::string("Unknown mnemonic: BLAH")));

    return ISO_TEST_RESULT();
}
