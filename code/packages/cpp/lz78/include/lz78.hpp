// lz78.hpp — the LZ78 (1978) lossless compression algorithm, in pure ISO C++17,
// header-only, in namespace ca::lz78. A faithful port of the Rust `lz78` crate
// (CMP01).
// ===========================================================================
//
// LZ78 builds an explicit trie dictionary as it encodes; encoder and decoder
// build the same dictionary independently, so none is transmitted. Each token is
// a (dict_index, next_char) pair — dict_index is the id of the longest matching
// dictionary prefix (0 for a literal), next_char the byte that follows.
//
// Wire format (CMP01), big-endian: u32 original length, u32 token count, then
// token_count * 4 bytes ([dict_index u16][next_char u8][0x00]).
//
// ROBUSTNESS: decode / decompress operate on possibly-untrusted bytes. Where the
// Rust crate would panic on an out-of-range dictionary index or hang on a cyclic
// one, this port bounds-checks and stops safely; for well-formed streams the
// output is identical.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_LZ78_HPP
#define CA_LZ78_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <unordered_map>
#include <utility>
#include <vector>

namespace ca {
namespace lz78 {

struct Token {
    std::uint16_t dict_index;
    std::uint8_t next_char;
    bool operator==(const Token& o) const {
        return dict_index == o.dict_index && next_char == o.next_char;
    }
    bool operator!=(const Token& o) const { return !(*this == o); }
};

// A byte-at-a-time trie walker (the crate's reusable dictionary abstraction).
class TrieCursor {
public:
    TrieCursor() : arena_(1), current_(0) {}

    bool step(std::uint8_t byte) {
        auto& children = arena_[current_].children;
        auto it = children.find(byte);
        if (it != children.end()) {
            current_ = it->second;
            return true;
        }
        return false;
    }

    void insert(std::uint8_t byte, std::uint16_t dict_id) {
        std::size_t new_idx = arena_.size();
        Node n;
        n.dict_id = dict_id;
        arena_.push_back(std::move(n));
        arena_[current_].children[byte] = new_idx;
    }

    void reset() { current_ = 0; }
    std::uint16_t dict_id() const { return arena_[current_].dict_id; }
    bool at_root() const { return current_ == 0; }

private:
    struct Node {
        std::uint16_t dict_id = 0;
        std::unordered_map<std::uint8_t, std::size_t> children;
    };
    std::vector<Node> arena_;
    std::size_t current_;
};

// Encode `data` into a token stream. `max_dict_size` caps the dictionary.
inline std::vector<Token> encode(const std::vector<std::uint8_t>& data,
                                 std::size_t max_dict_size) {
    TrieCursor cursor;
    unsigned next_id = 1;
    std::vector<Token> tokens;
    for (std::uint8_t byte : data) {
        if (!cursor.step(byte)) {
            tokens.push_back(Token{cursor.dict_id(), byte});
            if (static_cast<std::size_t>(next_id) < max_dict_size) {
                cursor.insert(byte, static_cast<std::uint16_t>(next_id));
                ++next_id;
            }
            cursor.reset();
        }
    }
    if (!cursor.at_root()) {
        tokens.push_back(Token{cursor.dict_id(), 0});
    }
    return tokens;
}

namespace detail {
inline void reconstruct_append(
    std::vector<std::uint8_t>& out,
    const std::vector<std::pair<std::uint16_t, std::uint8_t>>& table,
    std::uint16_t index) {
    std::size_t start = out.size();
    std::size_t idx = index, iterations = 0;
    while (idx != 0) {
        if (idx >= table.size()) {
            break; // out-of-range reference (malformed)
        }
        if (iterations++ > table.size()) {
            break; // cyclic reference (malformed)
        }
        out.push_back(table[idx].second);
        idx = table[idx].first;
    }
    std::reverse(out.begin() + static_cast<std::ptrdiff_t>(start), out.end());
}
}  // namespace detail

// Decode a token stream. If `original_length` is set, the output is truncated to
// it (stripping the flush sentinel); otherwise all bytes are emitted.
inline std::vector<std::uint8_t> decode(
    const std::vector<Token>& tokens,
    std::optional<std::size_t> original_length) {
    std::vector<std::pair<std::uint16_t, std::uint8_t>> table;
    table.emplace_back(std::uint16_t(0), std::uint8_t(0)); // root sentinel
    std::vector<std::uint8_t> output;
    for (const Token& tok : tokens) {
        detail::reconstruct_append(output, table, tok.dict_index);
        if (!original_length || output.size() < *original_length) {
            output.push_back(tok.next_char);
        }
        table.emplace_back(tok.dict_index, tok.next_char);
        if (original_length && output.size() >= *original_length) {
            break; // already have enough; the rest would be truncated away
        }
    }
    if (original_length && output.size() > *original_length) {
        output.resize(*original_length);
    }
    return output;
}

// One-shot compress to the CMP01 wire format.
inline std::vector<std::uint8_t> compress(const std::vector<std::uint8_t>& data,
                                          std::size_t max_dict_size) {
    std::vector<Token> tokens = encode(data, max_dict_size);
    std::vector<std::uint8_t> buf;
    auto put_be32 = [&buf](std::uint32_t v) {
        buf.push_back(static_cast<std::uint8_t>((v >> 24) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>((v >> 16) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>(v & 0xFF));
    };
    put_be32(static_cast<std::uint32_t>(data.size()));
    put_be32(static_cast<std::uint32_t>(tokens.size()));
    for (const Token& tok : tokens) {
        buf.push_back(static_cast<std::uint8_t>((tok.dict_index >> 8) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>(tok.dict_index & 0xFF));
        buf.push_back(tok.next_char);
        buf.push_back(0x00);
    }
    return buf;
}

// One-shot decompress from the CMP01 wire format.
inline std::vector<std::uint8_t> decompress(
    const std::vector<std::uint8_t>& data) {
    if (data.size() < 8) {
        return {};
    }
    std::size_t orig_len = (static_cast<std::size_t>(data[0]) << 24) |
                           (static_cast<std::size_t>(data[1]) << 16) |
                           (static_cast<std::size_t>(data[2]) << 8) |
                           static_cast<std::size_t>(data[3]);
    std::size_t token_count = (static_cast<std::size_t>(data[4]) << 24) |
                              (static_cast<std::size_t>(data[5]) << 16) |
                              (static_cast<std::size_t>(data[6]) << 8) |
                              static_cast<std::size_t>(data[7]);
    std::size_t avail = (data.size() - 8) / 4;
    std::size_t nread = std::min(token_count, avail);
    std::vector<Token> tokens;
    tokens.reserve(nread);
    for (std::size_t i = 0; i < nread; ++i) {
        std::size_t base = 8 + i * 4;
        tokens.push_back(Token{
            static_cast<std::uint16_t>((static_cast<unsigned>(data[base]) << 8) |
                                       data[base + 1]),
            data[base + 2]});
    }
    return decode(tokens, orig_len);
}

}  // namespace lz78
}  // namespace ca

#endif  // CA_LZ78_HPP
