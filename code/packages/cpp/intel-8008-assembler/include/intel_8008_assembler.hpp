// intel_8008_assembler.hpp — a two-pass Intel 8008 assembler, header-only C++17.
// ==============================================================================
//
// A faithful port of the Rust `intel-8008-assembler` crate, in namespace
// `ca::intel8008_assembler`: it turns Intel 8008 assembly *text* into raw
// machine-code bytes.
//
// Two passes are needed because of forward references (`JMP loop_end` can appear
// before `loop_end:` is defined):
//
//   Pass 1 — walk every line, track a program counter, and record each label's
//            address in a symbol table.
//   Pass 2 — walk again and encode each instruction, now that every label's
//            address is known.
//
// Where the Rust crate returns `Result<_, AssemblerError>`, this port throws
// `ca::intel8008_assembler::AssemblerError` (a std::runtime_error). Pure ISO
// C++17: <cstdint>, <map>, <stdexcept>, <string>, <vector>.

#ifndef INTEL_8008_ASSEMBLER_HPP
#define INTEL_8008_ASSEMBLER_HPP

#include <cctype>
#include <cstddef>
#include <cstdint>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace intel8008_assembler {

// The single error type for all assembly failures (message preserved verbatim).
class AssemblerError : public std::runtime_error {
  public:
    explicit AssemblerError(const std::string& message)
        : std::runtime_error(message) {}
};

// Symbol table: label name -> byte address. std::map keeps it ordered (the
// assembler only ever *looks up* symbols, so ordering is immaterial to output;
// it just makes the type deterministic).
using Symbols = std::map<std::string, std::size_t>;

// Maximum 14-bit address on the Intel 8008 (16 KB address space).
constexpr std::size_t kMaxAddress = 0x3FFF;

namespace detail {

// Register encoding: B=0, C=1, D=2, E=3, H=4, L=5, M=6 (mem at H:L), A=7.
inline std::uint8_t parse_register(const std::string& name) {
    // Trim, then uppercase (register names have no interior whitespace).
    std::string u;
    std::size_t b = 0, e = name.size();
    while (b < e && std::isspace(static_cast<unsigned char>(name[b]))) ++b;
    while (e > b && std::isspace(static_cast<unsigned char>(name[e - 1]))) --e;
    for (std::size_t i = b; i < e; ++i) {
        char c = name[i];
        u.push_back(static_cast<char>(
            (c >= 'a' && c <= 'z') ? c - ('a' - 'A') : c));
    }
    if (u == "B") return 0;
    if (u == "C") return 1;
    if (u == "D") return 2;
    if (u == "E") return 3;
    if (u == "H") return 4;
    if (u == "L") return 5;
    if (u == "M") return 6;
    if (u == "A") return 7;
    throw AssemblerError("Invalid 8008 register: \"" + u +
                         "\". Valid registers: A, B, C, D, E, H, L, M");
}

inline std::string trim(const std::string& s) {
    std::size_t b = 0, e = s.size();
    while (b < e && std::isspace(static_cast<unsigned char>(s[b]))) ++b;
    while (e > b && std::isspace(static_cast<unsigned char>(s[e - 1]))) --e;
    return s.substr(b, e - b);
}

inline std::string to_lower(const std::string& s) {
    std::string o;
    o.reserve(s.size());
    for (char c : s) {
        o.push_back(static_cast<char>(
            (c >= 'A' && c <= 'Z') ? c + ('a' - 'A') : c));
    }
    return o;
}

// Parse a decimal or 0x-prefixed hex literal into a value.
inline std::size_t parse_number(const std::string& raw) {
    std::string t = trim(raw);
    bool hex = false;
    std::string digits = t;
    if (t.size() >= 2 && t[0] == '0' && (t[1] == 'x' || t[1] == 'X')) {
        hex = true;
        digits = t.substr(2);
    }
    if (digits.empty()) {
        throw AssemblerError((hex ? "Invalid hex literal: \""
                                  : "Invalid numeric literal: \"") +
                             t + "\"");
    }
    std::size_t value = 0;
    const std::size_t base = hex ? 16u : 10u;
    for (char c : digits) {
        unsigned d;
        if (c >= '0' && c <= '9') {
            d = static_cast<unsigned>(c - '0');
        } else if (hex && c >= 'a' && c <= 'f') {
            d = static_cast<unsigned>(c - 'a' + 10);
        } else if (hex && c >= 'A' && c <= 'F') {
            d = static_cast<unsigned>(c - 'A' + 10);
        } else {
            throw AssemblerError((hex ? "Invalid hex literal: \""
                                      : "Invalid numeric literal: \"") +
                                 t + "\"");
        }
        // Overflow guard (Rust's usize::parse errors on overflow).
        if (value > (static_cast<std::size_t>(-1) - d) / base) {
            throw AssemblerError((hex ? "Invalid hex literal: \""
                                      : "Invalid numeric literal: \"") +
                                 t + "\"");
        }
        value = value * base + d;
    }
    return value;
}

// Resolve `hi(sym)` / `lo(sym)`. Returns {matched, value}.
inline bool resolve_hi_lo(const std::string& s, const Symbols& symbols,
                          std::size_t& out) {
    std::string lower = to_lower(s);
    bool is_hi;
    if (lower.rfind("hi(", 0) == 0 && !lower.empty() && lower.back() == ')') {
        is_hi = true;
    } else if (lower.rfind("lo(", 0) == 0 && !lower.empty() &&
               lower.back() == ')') {
        is_hi = false;
    } else {
        return false;
    }
    // sym name taken from the original (case-preserved) string: s[3 .. len-1]
    std::string sym = s.substr(3, s.size() - 4);
    auto it = symbols.find(sym);
    if (it == symbols.end()) {
        throw AssemblerError("Undefined label in \"" + s + "\": \"" + sym +
                             "\"");
    }
    std::size_t addr = it->second;
    out = is_hi ? ((addr >> 8) & 0x3F) : (addr & 0xFF);
    return true;
}

inline std::size_t resolve_operand(const std::string& operand,
                                   const Symbols& symbols, std::size_t pc) {
    std::string s = trim(operand);
    if (s == "$") {
        return pc;
    }
    std::size_t hilo;
    if (resolve_hi_lo(s, symbols, hilo)) {
        return hilo;
    }
    if ((s.size() >= 2 && s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) ||
        (!s.empty() &&
         ((s[0] >= '0' && s[0] <= '9') || s[0] == '-'))) {
        return parse_number(s);
    }
    auto it = symbols.find(s);
    if (it == symbols.end()) {
        throw AssemblerError("Undefined label: \"" + s + "\"");
    }
    return it->second;
}

inline void check_range(const std::string& name, std::size_t value,
                        std::size_t lo, std::size_t hi) {
    if (value < lo || value > hi) {
        throw AssemblerError(name + " value " + std::to_string(value) +
                             " is out of range [" + std::to_string(lo) + ", " +
                             std::to_string(hi) + "]");
    }
}

inline void expect_operands(const std::string& mnemonic,
                            const std::vector<std::string>& operands,
                            std::size_t count) {
    if (operands.size() != count) {
        throw AssemblerError(mnemonic + " expects " + std::to_string(count) +
                             " operand(s), got " +
                             std::to_string(operands.size()));
    }
}

// Fixed 1-byte opcode (returns/rotations/halt), or -1 if not fixed.
inline int fixed_opcode(const std::string& m) {
    if (m == "RLC") return 0x02;
    if (m == "RRC") return 0x0A;
    if (m == "RAL") return 0x12;
    if (m == "RAR") return 0x1A;
    if (m == "RFC" || m == "RET") return 0x03;
    if (m == "RFZ") return 0x0B;
    if (m == "RFS") return 0x13;
    if (m == "RFP") return 0x1B;
    if (m == "RTC") return 0x07;
    if (m == "RTZ") return 0x0F;
    if (m == "RTS") return 0x17;
    if (m == "RTP") return 0x1F;
    if (m == "HLT") return 0xFF;
    return -1;
}
inline int alu_reg_base(const std::string& m) {
    if (m == "ADD") return 0x80;
    if (m == "ADC") return 0x88;
    if (m == "SUB") return 0x90;
    if (m == "SBB") return 0x98;
    if (m == "ANA") return 0xA0;
    if (m == "XRA") return 0xA8;
    if (m == "ORA") return 0xB0;
    if (m == "CMP") return 0xB8;
    return -1;
}
inline int alu_imm_opcode(const std::string& m) {
    if (m == "ADI") return 0xC4;
    if (m == "ACI") return 0xCC;
    if (m == "SUI") return 0xD4;
    if (m == "SBI") return 0xDC;
    if (m == "ANI") return 0xE4;
    if (m == "XRI") return 0xEC;
    if (m == "ORI") return 0xF4;
    if (m == "CPI") return 0xFC;
    return -1;
}
inline int jump_call_opcode(const std::string& m) {
    if (m == "JMP") return 0x7C;
    if (m == "CAL") return 0x7E;
    if (m == "JFC") return 0x40;
    if (m == "JTC") return 0x44;
    if (m == "JFZ") return 0x48;
    if (m == "JTZ") return 0x4C;
    if (m == "JFS") return 0x50;
    if (m == "JTS") return 0x54;
    if (m == "JFP") return 0x58;
    if (m == "JTP") return 0x5C;
    if (m == "CFC") return 0x42;
    if (m == "CTC") return 0x46;
    if (m == "CFZ") return 0x4A;
    if (m == "CTZ") return 0x4E;
    if (m == "CFS") return 0x52;
    if (m == "CTS") return 0x56;
    if (m == "CFP") return 0x5A;
    if (m == "CTP") return 0x5E;
    return -1;
}

}  // namespace detail

// Return the encoded byte size of a mnemonic (0 for ORG). Throws on unknown.
inline std::size_t instruction_size(const std::string& m) {
    if (m == "RFC" || m == "RET" || m == "RTC" || m == "RFZ" || m == "RTZ" ||
        m == "RFS" || m == "RTS" || m == "RFP" || m == "RTP" || m == "RLC" ||
        m == "RRC" || m == "RAL" || m == "RAR" || m == "HLT") {
        return 1;
    }
    if (m == "ADD" || m == "ADC" || m == "SUB" || m == "SBB" || m == "ANA" ||
        m == "XRA" || m == "ORA" || m == "CMP") {
        return 1;
    }
    if (m == "MOV" || m == "INR" || m == "DCR" || m == "IN" || m == "OUT" ||
        m == "RST") {
        return 1;
    }
    if (m == "MVI" || m == "ADI" || m == "ACI" || m == "SUI" || m == "SBI" ||
        m == "ANI" || m == "XRI" || m == "ORI" || m == "CPI") {
        return 2;
    }
    if (m == "JMP" || m == "CAL" || m == "JFC" || m == "JTC" || m == "JFZ" ||
        m == "JTZ" || m == "JFS" || m == "JTS" || m == "JFP" || m == "JTP" ||
        m == "CFC" || m == "CTC" || m == "CFZ" || m == "CTZ" || m == "CFS" ||
        m == "CTS" || m == "CFP" || m == "CTP") {
        return 3;
    }
    if (m == "ORG") {
        return 0;
    }
    throw AssemblerError("Unknown mnemonic: \"" + m + "\"");
}

// Encode one instruction into bytes (the heart of Pass 2).
inline std::vector<std::uint8_t> encode_instruction(
    const std::string& mnemonic, const std::vector<std::string>& operands,
    const Symbols& symbols, std::size_t pc) {
    using namespace detail;
    if (mnemonic == "ORG") {
        return {};
    }
    int fx = fixed_opcode(mnemonic);
    if (fx >= 0) {
        expect_operands(mnemonic, operands, 0);
        return {static_cast<std::uint8_t>(fx)};
    }
    if (mnemonic == "MOV") {
        expect_operands(mnemonic, operands, 2);
        std::uint8_t dst = parse_register(operands[0]);
        std::uint8_t src = parse_register(operands[1]);
        return {static_cast<std::uint8_t>(0x40 | (dst << 3) | src)};
    }
    if (mnemonic == "MVI") {
        expect_operands(mnemonic, operands, 2);
        std::uint8_t r = parse_register(operands[0]);
        std::size_t d8 = resolve_operand(operands[1], symbols, pc);
        check_range(mnemonic + " immediate", d8, 0, 255);
        return {static_cast<std::uint8_t>((r << 3) | 0x06),
                static_cast<std::uint8_t>(d8)};
    }
    if (mnemonic == "INR") {
        expect_operands(mnemonic, operands, 1);
        std::uint8_t r = parse_register(operands[0]);
        return {static_cast<std::uint8_t>(r << 3)};
    }
    if (mnemonic == "DCR") {
        expect_operands(mnemonic, operands, 1);
        std::uint8_t r = parse_register(operands[0]);
        return {static_cast<std::uint8_t>((r << 3) | 0x01)};
    }
    if (mnemonic == "RST") {
        expect_operands(mnemonic, operands, 1);
        std::size_t n = resolve_operand(operands[0], symbols, pc);
        check_range("RST n", n, 0, 7);
        return {static_cast<std::uint8_t>((static_cast<std::uint8_t>(n) << 3) |
                                          0x05)};
    }
    int base = alu_reg_base(mnemonic);
    if (base >= 0) {
        expect_operands(mnemonic, operands, 1);
        std::uint8_t r = parse_register(operands[0]);
        return {static_cast<std::uint8_t>(base | r)};
    }
    int imm = alu_imm_opcode(mnemonic);
    if (imm >= 0) {
        expect_operands(mnemonic, operands, 1);
        std::size_t d8 = resolve_operand(operands[0], symbols, pc);
        check_range(mnemonic + " immediate", d8, 0, 255);
        return {static_cast<std::uint8_t>(imm),
                static_cast<std::uint8_t>(d8)};
    }
    if (mnemonic == "IN") {
        expect_operands(mnemonic, operands, 1);
        std::size_t p = resolve_operand(operands[0], symbols, pc);
        check_range("IN port", p, 0, 7);
        return {static_cast<std::uint8_t>(0x41 |
                                          (static_cast<std::uint8_t>(p) << 3))};
    }
    if (mnemonic == "OUT") {
        expect_operands(mnemonic, operands, 1);
        std::size_t p = resolve_operand(operands[0], symbols, pc);
        check_range("OUT port", p, 0, 23);
        return {static_cast<std::uint8_t>(static_cast<std::uint8_t>(p) << 1)};
    }
    int jc = jump_call_opcode(mnemonic);
    if (jc >= 0) {
        expect_operands(mnemonic, operands, 1);
        std::size_t addr = resolve_operand(operands[0], symbols, pc);
        check_range(mnemonic + " address", addr, 0, kMaxAddress);
        return {static_cast<std::uint8_t>(jc),
                static_cast<std::uint8_t>(addr & 0xFF),
                static_cast<std::uint8_t>((addr >> 8) & 0x3F)};
    }
    throw AssemblerError("Unknown mnemonic: \"" + mnemonic + "\"");
}

namespace detail {

struct ParsedLine {
    bool has_label = false;
    std::string label;
    bool has_mnemonic = false;
    std::string mnemonic;
    std::vector<std::string> operands;
};

// Parse a leading `ident:` label; sets out_label and returns the rest.
inline std::string parse_label_prefix(const std::string& s, bool& has_label,
                                      std::string& out_label) {
    has_label = false;
    if (s.empty()) {
        return s;
    }
    char first = s[0];
    bool alpha = (first >= 'a' && first <= 'z') || (first >= 'A' && first <= 'Z');
    if (!alpha && first != '_') {
        return s;
    }
    std::size_t end = 0;
    while (end < s.size()) {
        char c = s[end];
        bool ident = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                     (c >= '0' && c <= '9') || c == '_';
        if (!ident) {
            break;
        }
        ++end;
    }
    if (end < s.size() && s[end] == ':') {
        has_label = true;
        out_label = s.substr(0, end);
        return s.substr(end + 1);
    }
    return s;
}

inline ParsedLine lex_line(const std::string& source) {
    ParsedLine line;
    // Step 1: strip comment (everything from the first ';').
    std::string text = source;
    std::size_t semi = text.find(';');
    if (semi != std::string::npos) {
        text = text.substr(0, semi);
    }
    // Step 2: trim_end.
    std::size_t e = text.size();
    while (e > 0 && std::isspace(static_cast<unsigned char>(text[e - 1]))) --e;
    text = text.substr(0, e);
    // Step 3: trim_start for label detection.
    std::size_t b = 0;
    while (b < text.size() &&
           std::isspace(static_cast<unsigned char>(text[b]))) ++b;
    std::string stripped = text.substr(b);
    // Step 4: label prefix.
    std::string after = parse_label_prefix(stripped, line.has_label, line.label);
    // rest = after trimmed at start.
    std::size_t rb = 0;
    while (rb < after.size() &&
           std::isspace(static_cast<unsigned char>(after[rb]))) ++rb;
    std::string rest = after.substr(rb);
    if (rest.empty()) {
        return line;  // label-only or blank
    }
    // Step 5: mnemonic = first whitespace-delimited token, uppercased.
    std::size_t sp = 0;
    while (sp < rest.size() &&
           !std::isspace(static_cast<unsigned char>(rest[sp]))) ++sp;
    std::string mnem_raw = rest.substr(0, sp);
    std::string operand_text = trim(rest.substr(sp));
    std::string mnem;
    for (char c : mnem_raw) {
        mnem.push_back(static_cast<char>(
            (c >= 'a' && c <= 'z') ? c - ('a' - 'A') : c));
    }
    line.has_mnemonic = true;
    line.mnemonic = mnem;
    // Step 6: split operands on ',', trim each, drop empties.
    if (!operand_text.empty()) {
        std::size_t start = 0;
        for (std::size_t i = 0; i <= operand_text.size(); ++i) {
            if (i == operand_text.size() || operand_text[i] == ',') {
                std::string piece = trim(operand_text.substr(start, i - start));
                if (!piece.empty()) {
                    line.operands.push_back(piece);
                }
                start = i + 1;
            }
        }
    }
    return line;
}

// Split text into lines like Rust's str::lines() (handles \r\n, drops a single
// trailing empty line produced by a final \n).
inline std::vector<std::string> split_lines(const std::string& text) {
    std::vector<std::string> out;
    std::size_t start = 0;
    for (std::size_t i = 0; i <= text.size(); ++i) {
        if (i == text.size() || text[i] == '\n') {
            std::string line = text.substr(start, i - start);
            if (!line.empty() && line.back() == '\r') {
                line.pop_back();
            }
            if (i == text.size()) {
                if (start < i) {
                    out.push_back(line);
                }
            } else {
                out.push_back(line);
            }
            start = i + 1;
        }
    }
    return out;
}

}  // namespace detail

// Two-pass assemble: text -> machine-code bytes (throws AssemblerError).
inline std::vector<std::uint8_t> assemble(const std::string& text) {
    using namespace detail;
    std::vector<ParsedLine> lines;
    for (const auto& src : split_lines(text)) {
        lines.push_back(lex_line(src));
    }

    // Pass 1 — symbol table.
    Symbols symbols;
    std::size_t pc = 0;
    for (const auto& line : lines) {
        if (line.has_label) {
            symbols[line.label] = pc;
        }
        if (!line.has_mnemonic) {
            continue;
        }
        if (line.mnemonic == "ORG") {
            if (line.operands.empty()) {
                throw AssemblerError("ORG requires an address operand");
            }
            std::size_t addr = parse_number(line.operands[0]);
            if (addr > kMaxAddress) {
                throw AssemblerError("ORG address exceeds Intel 8008 address "
                                     "space");
            }
            pc = addr;
            continue;
        }
        pc += instruction_size(line.mnemonic);
    }

    // Pass 2 — emit bytes.
    std::vector<std::uint8_t> output;
    pc = 0;
    for (const auto& line : lines) {
        if (!line.has_mnemonic) {
            continue;
        }
        if (line.mnemonic == "ORG") {
            if (line.operands.empty()) {
                throw AssemblerError("ORG requires an address operand");
            }
            std::size_t org = parse_number(line.operands[0]);
            if (org > kMaxAddress) {
                throw AssemblerError("ORG address exceeds Intel 8008 address "
                                     "space");
            }
            if (org > pc) {
                output.insert(output.end(), org - pc,
                              static_cast<std::uint8_t>(0xFF));
            }
            pc = org;
            continue;
        }
        std::vector<std::uint8_t> encoded =
            encode_instruction(line.mnemonic, line.operands, symbols, pc);
        pc += encoded.size();
        output.insert(output.end(), encoded.begin(), encoded.end());
    }
    return output;
}

}  // namespace intel8008_assembler
}  // namespace ca

#endif  // INTEL_8008_ASSEMBLER_HPP
