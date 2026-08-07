// assembler.hpp — ARM assembly parser and binary encoder, header-only in pure
// ISO C++17 (namespace ca::assembler). A faithful port of the Rust `assembler`
// crate.
// ===========================================================================
//
// Parses a subset of ARM assembly text into structured instructions, then
// encodes each into its 32-bit ARM machine-code word. Supported mnemonics:
// MOV(S), ADD(S), SUB(S), AND(S), ORR(S), EOR(S), RSB(S), CMP, LDR, STR, NOP,
// and labels (`name:`).
//
// DIVERGENCE FROM RUST. `Result<_, AssemblerError>` -> methods that throw
// `AssemblerError` (a std::runtime_error whose message reproduces the Rust
// `Display` text). `Vec` -> `std::vector`; `Option<u32>` -> `std::optional`;
// the Rust `ArmInstruction` enum -> a `std::variant`.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no <cmath>, no compiler extensions.
#ifndef CA_ASSEMBLER_HPP
#define CA_ASSEMBLER_HPP

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

namespace ca {
namespace assembler {

// ARM data-processing opcodes (bits 24-21).
enum class ArmOpcode : std::uint32_t {
    And = 0x0,
    Eor = 0x1,
    Sub = 0x2,
    Rsb = 0x3,
    Add = 0x4,
    Cmp = 0xA,
    Orr = 0xC,
    Mov = 0xD
};

// Second operand: a register index or an immediate value.
struct Operand2 {
    enum class Kind { Register, Immediate } kind;
    std::uint32_t value;
    static Operand2 reg(std::uint32_t r) { return {Kind::Register, r}; }
    static Operand2 imm(std::uint32_t v) { return {Kind::Immediate, v}; }
    bool operator==(const Operand2& o) const {
        return kind == o.kind && value == o.value;
    }
};

// The parsed-instruction variants (mirroring the Rust `ArmInstruction` enum).
struct DataProcessing {
    ArmOpcode opcode;
    std::optional<std::uint32_t> rd;  // None for CMP
    std::optional<std::uint32_t> rn;  // None for MOV
    Operand2 operand2;
    bool set_flags;
};
struct Load {
    std::uint32_t rd, rn;
};
struct Store {
    std::uint32_t rd, rn;
};
struct Nop {};
struct Label {
    std::string name;
};
using ArmInstruction = std::variant<DataProcessing, Load, Store, Nop, Label>;

// Thrown by parse/encode on any error; `what()` matches the Rust Display text.
class AssemblerError : public std::runtime_error {
public:
    explicit AssemblerError(const std::string& msg) : std::runtime_error(msg) {}
    static AssemblerError unknown_mnemonic(const std::string& m) {
        return AssemblerError("Unknown mnemonic: " + m);
    }
    static AssemblerError invalid_register(const std::string& r) {
        return AssemblerError("Invalid register: " + r);
    }
    static AssemblerError invalid_immediate(const std::string& v) {
        return AssemblerError("Invalid immediate: " + v);
    }
    static AssemblerError invalid_operand_count(const std::string& m, std::size_t expected,
                                               std::size_t got) {
        return AssemblerError(m + ": expected " + std::to_string(expected) +
                              " operands, got " + std::to_string(got));
    }
    static AssemblerError parse_error(const std::string& msg) {
        return AssemblerError("Parse error: " + msg);
    }
};

namespace detail {

inline bool is_ws(char c) {
    return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\v' || c == '\f';
}

inline std::string trim(const std::string& s) {
    std::size_t a = 0, b = s.size();
    while (a < b && is_ws(s[a])) a++;
    while (b > a && is_ws(s[b - 1])) b--;
    return s.substr(a, b - a);
}

inline std::string to_upper(const std::string& s) {
    std::string out = s;
    for (char& c : out)
        if (c >= 'a' && c <= 'z') c = static_cast<char>(c - 32);
    return out;
}

// Parse a register (already trimmed). Returns std::nullopt if invalid.
inline std::optional<std::uint32_t> parse_register(const std::string& raw) {
    std::string s = to_upper(trim(raw));
    if (s == "SP") return 13u;
    if (s == "LR") return 14u;
    if (s == "PC") return 15u;
    if (s.empty() || s[0] != 'R' || s.size() < 2) return std::nullopt;
    std::uint32_t n = 0;
    for (std::size_t i = 1; i < s.size(); i++) {
        if (s[i] < '0' || s[i] > '9') return std::nullopt;
        n = n * 10 + static_cast<std::uint32_t>(s[i] - '0');
        if (n > 15) return std::nullopt;
    }
    return n;
}

// Parse an immediate (already trimmed): optional '#', then decimal or 0x-hex.
inline std::optional<std::uint32_t> parse_immediate(const std::string& raw) {
    std::string s = trim(raw);
    if (!s.empty() && s[0] == '#') s = trim(s.substr(1));
    bool hex = false;
    if (s.size() >= 2 && s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) {
        hex = true;
        s = s.substr(2);
    }
    if (s.empty()) return std::nullopt;
    std::uint32_t n = 0;
    for (char c : s) {
        int d;
        if (c >= '0' && c <= '9') d = c - '0';
        else if (hex && c >= 'a' && c <= 'f') d = c - 'a' + 10;
        else if (hex && c >= 'A' && c <= 'F') d = c - 'A' + 10;
        else return std::nullopt;
        n = n * (hex ? 16u : 10u) + static_cast<std::uint32_t>(d);
    }
    return n;
}

// Split by ',' into trimmed tokens (empty input -> no tokens).
inline std::vector<std::string> split_operands(const std::string& s) {
    std::vector<std::string> out;
    if (s.empty()) return out;
    std::size_t start = 0;
    for (;;) {
        std::size_t comma = s.find(',', start);
        if (comma == std::string::npos) {
            out.push_back(trim(s.substr(start)));
            break;
        }
        out.push_back(trim(s.substr(start, comma - start)));
        start = comma + 1;
    }
    return out;
}

inline Operand2 parse_operand2(const std::string& s) {
    std::string t = trim(s);
    if (!t.empty() && t[0] == '#') {
        auto imm = parse_immediate(t);
        if (!imm) throw AssemblerError::invalid_immediate(t);
        return Operand2::imm(*imm);
    }
    auto reg = parse_register(t);
    if (reg) return Operand2::reg(*reg);
    throw AssemblerError::parse_error("Cannot parse operand: " + t);
}

}  // namespace detail

class Assembler {
public:
    std::unordered_map<std::string, std::size_t> labels;

    std::vector<ArmInstruction> parse(const std::string& source) {
        std::vector<ArmInstruction> instructions;
        std::size_t address = 0;

        std::size_t pos = 0;
        while (pos < source.size()) {
            std::size_t nl = source.find('\n', pos);
            std::size_t end = (nl == std::string::npos) ? source.size() : nl;
            std::string line = source.substr(pos, end - pos);
            pos = (nl == std::string::npos) ? source.size() : nl + 1;

            // Strip comments: first ';' then first "//".
            std::size_t semi = line.find(';');
            if (semi != std::string::npos) line = line.substr(0, semi);
            std::size_t slashes = line.find("//");
            if (slashes != std::string::npos) line = line.substr(0, slashes);
            line = detail::trim(line);
            if (line.empty()) continue;

            if (line.back() == ':') {
                std::string name = detail::trim(line.substr(0, line.size() - 1));
                labels[name] = address;
                instructions.push_back(Label{name});
                continue;
            }

            instructions.push_back(parse_instruction(line));
            address += 1;
        }
        return instructions;
    }

    std::vector<std::uint32_t> encode(const std::vector<ArmInstruction>& instrs) const {
        std::vector<std::uint32_t> binary;
        for (const ArmInstruction& in : instrs) {
            if (std::holds_alternative<Label>(in)) {
                continue;  // no output
            } else if (std::holds_alternative<Nop>(in)) {
                binary.push_back(0xE1A00000u);
            } else if (const auto* dp = std::get_if<DataProcessing>(&in)) {
                std::uint32_t cond = 0xE;
                std::uint32_t rd = dp->rd.value_or(0);
                std::uint32_t rn = dp->rn.value_or(0);
                std::uint32_t s = dp->set_flags ? 1u : 0u;
                std::uint32_t opcode = static_cast<std::uint32_t>(dp->opcode);
                std::uint32_t i_bit, op2;
                if (dp->operand2.kind == Operand2::Kind::Immediate) {
                    i_bit = 1;
                    op2 = dp->operand2.value & 0xFFFu;
                } else {
                    i_bit = 0;
                    op2 = dp->operand2.value & 0xFu;
                }
                binary.push_back((cond << 28) | (i_bit << 25) | (opcode << 21) |
                                 (s << 20) | (rn << 16) | (rd << 12) | op2);
            } else if (const auto* ld = std::get_if<Load>(&in)) {
                binary.push_back(0xE5900000u | (ld->rn << 16) | (ld->rd << 12));
            } else if (const auto* st = std::get_if<Store>(&in)) {
                binary.push_back(0xE5800000u | (st->rn << 16) | (st->rd << 12));
            }
        }
        return binary;
    }

private:
    ArmInstruction parse_instruction(const std::string& line) const {
        std::size_t sp = 0;
        while (sp < line.size() && !detail::is_ws(line[sp])) sp++;
        std::string mnem = detail::to_upper(line.substr(0, sp));
        std::string operands_str = (sp < line.size()) ? detail::trim(line.substr(sp + 1)) : "";

        if (mnem == "NOP") return Nop{};

        if (mnem == "MOV" || mnem == "MOVS") {
            auto ops = detail::split_operands(operands_str);
            if (ops.size() != 2) throw AssemblerError::invalid_operand_count(mnem, 2, ops.size());
            auto rd = detail::parse_register(ops[0]);
            if (!rd) throw AssemblerError::invalid_register(ops[0]);
            return DataProcessing{ArmOpcode::Mov, rd, std::nullopt,
                                  detail::parse_operand2(ops[1]), mnem == "MOVS"};
        }

        if (mnem == "ADD" || mnem == "ADDS" || mnem == "SUB" || mnem == "SUBS" ||
            mnem == "AND" || mnem == "ANDS" || mnem == "ORR" || mnem == "ORRS" ||
            mnem == "EOR" || mnem == "EORS" || mnem == "RSB" || mnem == "RSBS") {
            std::string base = mnem;
            while (!base.empty() && base.back() == 'S') base.pop_back();
            bool set_flags = mnem.size() > base.size();
            ArmOpcode opcode;
            if (base == "AND") opcode = ArmOpcode::And;
            else if (base == "EOR") opcode = ArmOpcode::Eor;
            else if (base == "SUB") opcode = ArmOpcode::Sub;
            else if (base == "RSB") opcode = ArmOpcode::Rsb;
            else if (base == "ADD") opcode = ArmOpcode::Add;
            else if (base == "ORR") opcode = ArmOpcode::Orr;
            else throw AssemblerError::unknown_mnemonic(mnem);
            auto ops = detail::split_operands(operands_str);
            if (ops.size() != 3) throw AssemblerError::invalid_operand_count(mnem, 3, ops.size());
            auto rd = detail::parse_register(ops[0]);
            if (!rd) throw AssemblerError::invalid_register(ops[0]);
            auto rn = detail::parse_register(ops[1]);
            if (!rn) throw AssemblerError::invalid_register(ops[1]);
            return DataProcessing{opcode, rd, rn, detail::parse_operand2(ops[2]), set_flags};
        }

        if (mnem == "CMP") {
            auto ops = detail::split_operands(operands_str);
            if (ops.size() != 2) throw AssemblerError::invalid_operand_count(mnem, 2, ops.size());
            auto rn = detail::parse_register(ops[0]);
            if (!rn) throw AssemblerError::invalid_register(ops[0]);
            return DataProcessing{ArmOpcode::Cmp, std::nullopt, rn,
                                  detail::parse_operand2(ops[1]), true};
        }

        if (mnem == "LDR" || mnem == "STR") {
            auto ops = detail::split_operands(operands_str);
            if (ops.size() != 2) throw AssemblerError::invalid_operand_count(mnem, 2, ops.size());
            auto rd = detail::parse_register(ops[0]);
            if (!rd) throw AssemblerError::invalid_register(ops[0]);
            std::string base = ops[1];
            std::size_t a = 0, b = base.size();
            while (a < b && base[a] == '[') a++;
            while (b > a && base[b - 1] == ']') b--;
            base = detail::trim(base.substr(a, b - a));
            auto rn = detail::parse_register(base);
            if (!rn) throw AssemblerError::invalid_register(base);
            if (mnem == "LDR") return Load{*rd, *rn};
            return Store{*rd, *rn};
        }

        throw AssemblerError::unknown_mnemonic(mnem);
    }
};

}  // namespace assembler
}  // namespace ca

#endif  // CA_ASSEMBLER_HPP
