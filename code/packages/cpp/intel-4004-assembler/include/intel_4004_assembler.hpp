// intel_4004_assembler.hpp — a two-pass assembler for the Intel 4004 (the first
// commercial microprocessor, 1971), in pure ISO C++17, header-only, in namespace
// ca::intel4004. A faithful port of the Rust `intel-4004-assembler` crate.
// ===========================================================================
//
// `assemble(text)` turns 4004 assembly into a std::vector<std::uint8_t> of
// machine code, or throws ca::intel4004::AssemblerError. Pass 1 builds a symbol
// table (label -> program counter, honouring ORG); pass 2 encodes each
// instruction, padding with zeros for forward ORGs.
//
// A line is `[label:] [mnemonic [operands]] [; comment]`. Mnemonics are
// case-insensitive; operands comma-separated. Registers `Rn`, register pairs
// `Pn`, numbers decimal or 0x-hex, bare identifiers are symbols.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_INTEL_4004_ASSEMBLER_HPP
#define CA_INTEL_4004_ASSEMBLER_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

namespace ca {
namespace intel4004 {

// Thrown on any assembly error (unknown mnemonic, bad operand, unknown symbol).
class AssemblerError : public std::runtime_error {
public:
    explicit AssemblerError(const std::string& msg) : std::runtime_error(msg) {}
};

namespace detail {

inline bool is_ws(char c) { return c == ' ' || c == '\t' || c == '\r'; }

inline std::string trim(const std::string& s) {
    std::size_t a = 0, b = s.size();
    while (a < b && is_ws(s[a])) {
        ++a;
    }
    while (b > a && is_ws(s[b - 1])) {
        --b;
    }
    return s.substr(a, b - a);
}

inline std::string upcase(std::string s) {
    for (char& c : s) {
        if (c >= 'a' && c <= 'z') {
            c = static_cast<char>(c - 'a' + 'A');
        }
    }
    return s;
}

struct ParsedLine {
    std::string label;
    std::string mnemonic;
    std::vector<std::string> operands;
};

inline ParsedLine lex_line(const std::string& raw) {
    ParsedLine pl;
    // Strip a trailing comment, then trim.
    std::string line = raw;
    std::size_t semi = line.find(';');
    if (semi != std::string::npos) {
        line = line.substr(0, semi);
    }
    line = trim(line);
    if (line.empty()) {
        return pl;
    }
    std::string rest = line;
    std::size_t colon = line.find(':');
    if (colon != std::string::npos) {
        std::string prefix = trim(line.substr(0, colon));
        if (prefix.find(' ') == std::string::npos &&
            prefix.find('\t') == std::string::npos) {
            pl.label = prefix;
            rest = trim(line.substr(colon + 1));
        }
    }
    if (rest.empty()) {
        return pl;
    }
    std::size_t k = 0;
    while (k < rest.size() && !is_ws(rest[k])) {
        ++k;
    }
    pl.mnemonic = upcase(rest.substr(0, k));
    std::string operand_text = trim(rest.substr(k));
    if (!operand_text.empty()) {
        std::size_t start = 0;
        for (std::size_t i = 0; i <= operand_text.size(); ++i) {
            if (i == operand_text.size() || operand_text[i] == ',') {
                std::string field = trim(operand_text.substr(start, i - start));
                if (!field.empty()) {
                    pl.operands.push_back(field);
                }
                start = i + 1;
            }
        }
    }
    return pl;
}

inline std::optional<std::uint16_t> parse_number(const std::string& text) {
    const char* p = text.c_str();
    bool hex = false;
    if (p[0] == '0' && p[1] == 'x') {
        hex = true;
        p += 2;
    }
    if (*p == '\0') {
        return std::nullopt;
    }
    unsigned long v = 0;
    for (; *p; ++p) {
        unsigned d;
        char c = *p;
        if (c >= '0' && c <= '9') {
            d = static_cast<unsigned>(c - '0');
        } else if (hex && c >= 'a' && c <= 'f') {
            d = static_cast<unsigned>(c - 'a') + 10;
        } else if (hex && c >= 'A' && c <= 'F') {
            d = static_cast<unsigned>(c - 'A') + 10;
        } else {
            return std::nullopt;
        }
        v = v * (hex ? 16u : 10u) + d;
        if (v > 0xFFFF) {
            return std::nullopt;
        }
    }
    return static_cast<std::uint16_t>(v);
}

inline std::uint8_t parse_u8_prefixed(const std::string& text, char prefix,
                                      const std::string& kind) {
    std::size_t i = 0;
    while (i < text.size() && is_ws(text[i])) {
        ++i;
    }
    while (i < text.size() && text[i] == prefix) {
        ++i;
    }
    if (i >= text.size()) {
        throw AssemblerError(kind + "'" + text + "'");
    }
    unsigned long v = 0;
    for (; i < text.size() && !is_ws(text[i]); ++i) {
        if (text[i] < '0' || text[i] > '9') {
            throw AssemblerError(kind + "'" + text + "'");
        }
        v = v * 10 + static_cast<unsigned>(text[i] - '0');
        if (v > 0xFF) {
            throw AssemblerError(kind + "'" + text + "'");
        }
    }
    return static_cast<std::uint8_t>(v);
}

inline std::uint8_t parse_register(const std::string& t) {
    return parse_u8_prefixed(t, 'R', "Invalid register: ");
}
inline std::uint8_t parse_pair(const std::string& t) {
    return parse_u8_prefixed(t, 'P', "Invalid register pair: ");
}

using SymbolTable = std::unordered_map<std::string, std::size_t>;

inline std::uint16_t resolve_operand(const std::string& text,
                                     const SymbolTable& symbols) {
    if (auto v = parse_number(text)) {
        return *v;
    }
    auto it = symbols.find(text);
    if (it != symbols.end()) {
        return static_cast<std::uint16_t>(it->second);
    }
    throw AssemblerError("Unknown symbol: '" + text + "'");
}

inline std::size_t instruction_size(const std::string& m) {
    if (m == "NOP" || m == "HLT" || m == "WRM" || m == "LDM" || m == "BBL" ||
        m == "INC" || m == "ADD" || m == "SUB" || m == "LD" || m == "XCH" ||
        m == "SRC" || m == "FIN" || m == "JIN") {
        return 1;
    }
    if (m == "JCN" || m == "FIM" || m == "JUN" || m == "JMS" || m == "ISZ" ||
        m == "ADD_IMM") {
        return 2;
    }
    throw AssemblerError("Unknown mnemonic: '" + m + "'");
}

inline void need(const ParsedLine& pl, std::size_t want) {
    if (pl.operands.size() != want) {
        throw AssemblerError(pl.mnemonic + " expects " + std::to_string(want) +
                             " operand(s), got " +
                             std::to_string(pl.operands.size()));
    }
}

inline std::vector<std::uint8_t> encode(const ParsedLine& pl,
                                        const SymbolTable& sym) {
    const std::string& m = pl.mnemonic;
    auto one = [&]() { need(pl, 1); };
    if (m == "NOP") return {0x00};
    if (m == "HLT") return {0x01};
    if (m == "WRM") return {0xE0};
    if (m == "LDM") {
        one();
        return {(std::uint8_t)(0xD0 | (resolve_operand(pl.operands[0], sym) & 0xF))};
    }
    if (m == "BBL") {
        one();
        return {(std::uint8_t)(0xC0 | (resolve_operand(pl.operands[0], sym) & 0xF))};
    }
    if (m == "INC") { one(); return {(std::uint8_t)(0x60 | parse_register(pl.operands[0]))}; }
    if (m == "ADD") { one(); return {(std::uint8_t)(0x80 | parse_register(pl.operands[0]))}; }
    if (m == "SUB") { one(); return {(std::uint8_t)(0x90 | parse_register(pl.operands[0]))}; }
    if (m == "LD")  { one(); return {(std::uint8_t)(0xA0 | parse_register(pl.operands[0]))}; }
    if (m == "XCH") { one(); return {(std::uint8_t)(0xB0 | parse_register(pl.operands[0]))}; }
    if (m == "SRC") { one(); return {(std::uint8_t)(0x20 | (2 * parse_pair(pl.operands[0]) + 1))}; }
    if (m == "FIN") { one(); return {(std::uint8_t)(0x30 | (2 * parse_pair(pl.operands[0])))}; }
    if (m == "JIN") { one(); return {(std::uint8_t)(0x30 | (2 * parse_pair(pl.operands[0]) + 1))}; }
    if (m == "FIM") {
        need(pl, 2);
        return {(std::uint8_t)(0x20 | (2 * parse_pair(pl.operands[0]))),
                (std::uint8_t)resolve_operand(pl.operands[1], sym)};
    }
    if (m == "JCN") {
        need(pl, 2);
        std::uint16_t a = resolve_operand(pl.operands[0], sym);
        std::uint16_t b = resolve_operand(pl.operands[1], sym);
        return {(std::uint8_t)(0x10 | (a & 0xF)), (std::uint8_t)(b & 0xFF)};
    }
    if (m == "JUN") {
        one();
        std::uint16_t a = resolve_operand(pl.operands[0], sym);
        return {(std::uint8_t)(0x40 | ((a >> 8) & 0xF)), (std::uint8_t)(a & 0xFF)};
    }
    if (m == "JMS") {
        one();
        std::uint16_t a = resolve_operand(pl.operands[0], sym);
        return {(std::uint8_t)(0x50 | ((a >> 8) & 0xF)), (std::uint8_t)(a & 0xFF)};
    }
    if (m == "ISZ") {
        need(pl, 2);
        std::uint16_t a = resolve_operand(pl.operands[1], sym);
        return {(std::uint8_t)(0x70 | parse_register(pl.operands[0])),
                (std::uint8_t)(a & 0xFF)};
    }
    if (m == "ADD_IMM") {
        need(pl, 3);
        std::uint8_t reg = parse_register(pl.operands[1]);
        std::uint16_t imm = resolve_operand(pl.operands[2], sym);
        return {(std::uint8_t)(0xD0 | (imm & 0xF)), (std::uint8_t)(0x80 | reg)};
    }
    throw AssemblerError("Unknown mnemonic: '" + m + "'");
}

}  // namespace detail

// assemble — turn 4004 assembly `text` into machine code (throws
// AssemblerError on any error).
inline std::vector<std::uint8_t> assemble(const std::string& text) {
    using namespace detail;
    std::vector<ParsedLine> lines;
    std::size_t pos = 0;
    while (true) {
        std::size_t nl = text.find('\n', pos);
        std::string raw = text.substr(pos, nl == std::string::npos
                                               ? std::string::npos
                                               : nl - pos);
        lines.push_back(lex_line(raw));
        if (nl == std::string::npos) {
            break;
        }
        pos = nl + 1;
    }

    // Pass 1: symbol table.
    SymbolTable symbols;
    std::size_t pc = 0;
    for (const ParsedLine& pl : lines) {
        if (!pl.label.empty()) {
            symbols[pl.label] = pc;
        }
        if (pl.mnemonic.empty()) {
            continue;
        }
        if (pl.mnemonic == "ORG") {
            if (pl.operands.empty()) {
                throw AssemblerError("ORG requires an operand");
            }
            auto v = parse_number(pl.operands[0]);
            if (!v) {
                throw AssemblerError("Invalid number: '" + pl.operands[0] + "'");
            }
            pc = *v;
            continue;
        }
        pc += instruction_size(pl.mnemonic);
    }

    // Pass 2: encode.
    std::vector<std::uint8_t> out;
    pc = 0;
    for (const ParsedLine& pl : lines) {
        if (pl.mnemonic.empty()) {
            continue;
        }
        if (pl.mnemonic == "ORG") {
            if (pl.operands.empty()) {
                throw AssemblerError("ORG requires an operand");
            }
            auto v = parse_number(pl.operands[0]);
            if (!v) {
                throw AssemblerError("Invalid number: '" + pl.operands[0] + "'");
            }
            while (pc < *v) {
                out.push_back(0);
                ++pc;
            }
            continue;
        }
        std::vector<std::uint8_t> enc = encode(pl, symbols);
        pc += enc.size();
        out.insert(out.end(), enc.begin(), enc.end());
    }
    return out;
}

}  // namespace intel4004
}  // namespace ca

#endif  // CA_INTEL_4004_ASSEMBLER_HPP
