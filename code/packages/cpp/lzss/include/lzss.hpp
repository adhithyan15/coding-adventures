// lzss.hpp — the LZSS lossless compression algorithm, in pure ISO C++17,
// header-only, in namespace ca::lzss. A faithful port of the Rust `lzss` crate
// (CMP02).
// ===========================================================================
//
// LZSS (Storer & Szymanski, 1982) is the sliding-window LZ77 variant used by
// DEFLATE and friends. At each position it searches the last `window_size` bytes
// for the longest match; a match of at least `min_match` bytes becomes a
// back-reference token, otherwise a literal byte is emitted. Matches may overlap
// the cursor, so runs encode as one short back-reference.
//
//   Literal(b)              a single byte
//   Match{offset, length}   copy `length` bytes from `offset` positions back
//
// Wire format (CMP02, big-endian): u32 original length, u32 block count, then
// blocks of a 1-byte flag (bit b => token b is a match) followed by each token's
// data (match = 2-byte offset + 1-byte length; literal = 1 byte).
//
// ROBUSTNESS: decode / decompress accept untrusted bytes — malformed matches
// (offset 0 or beyond the output) are skipped, the block count is capped to the
// payload, and output is bounded by the declared length.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_LZSS_HPP
#define CA_LZSS_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

namespace ca {
namespace lzss {

constexpr std::size_t DEFAULT_WINDOW_SIZE = 4096;
constexpr std::size_t DEFAULT_MAX_MATCH = 255;
constexpr std::size_t DEFAULT_MIN_MATCH = 3;

// One LZSS token: a literal byte or a back-reference match.
struct Token {
    bool is_match;
    std::uint8_t literal;    // when !is_match
    std::uint16_t offset;    // when is_match
    std::uint8_t length;     // when is_match

    static Token lit(std::uint8_t b) { return Token{false, b, 0, 0}; }
    static Token match(std::uint16_t off, std::uint8_t len) {
        return Token{true, 0, off, len};
    }
    bool operator==(const Token& o) const {
        if (is_match != o.is_match) {
            return false;
        }
        return is_match ? (offset == o.offset && length == o.length)
                        : (literal == o.literal);
    }
    bool operator!=(const Token& o) const { return !(*this == o); }
};

namespace detail {
// Longest match for data[cursor..] in data[win_start..cursor], overlap-allowed.
inline void find_longest_match(const std::vector<std::uint8_t>& data,
                               std::size_t cursor, std::size_t win_start,
                               std::size_t max_match, std::uint16_t& best_off,
                               std::uint8_t& best_len) {
    std::size_t blen = 0, boff = 0;
    std::size_t lookahead_end = std::min(cursor + max_match, data.size());
    for (std::size_t pos = win_start; pos < cursor; ++pos) {
        std::size_t l = 0;
        while (cursor + l < lookahead_end && data[pos + l] == data[cursor + l]) {
            ++l;
        }
        if (l > blen) {
            blen = l;
            boff = cursor - pos;
        }
    }
    best_off = static_cast<std::uint16_t>(boff);
    best_len = static_cast<std::uint8_t>(blen);
}
}  // namespace detail

// Encode `data` into a token stream.
inline std::vector<Token> encode(const std::vector<std::uint8_t>& data,
                                 std::size_t window_size, std::size_t max_match,
                                 std::size_t min_match) {
    std::vector<Token> tokens;
    std::size_t cursor = 0;
    while (cursor < data.size()) {
        std::size_t win_start =
            cursor > window_size ? cursor - window_size : 0;
        std::uint16_t offset;
        std::uint8_t length;
        detail::find_longest_match(data, cursor, win_start, max_match, offset,
                                   length);
        if (static_cast<std::size_t>(length) >= min_match) {
            tokens.push_back(Token::match(offset, length));
            cursor += length;
        } else {
            tokens.push_back(Token::lit(data[cursor]));
            cursor += 1;
        }
    }
    return tokens;
}

// Decode a token stream (truncated to `original_length` when set).
inline std::vector<std::uint8_t> decode(
    const std::vector<Token>& tokens,
    std::optional<std::size_t> original_length) {
    std::vector<std::uint8_t> output;
    for (const Token& tok : tokens) {
        if (!tok.is_match) {
            output.push_back(tok.literal);
        } else {
            std::size_t off = tok.offset;
            if (off != 0 && off <= output.size()) {
                std::size_t start = output.size() - off;
                for (std::size_t i = 0; i < tok.length; ++i) {
                    // Copy to a local first: push_back(output[...]) would dangle
                    // the reference if the vector reallocates.
                    std::uint8_t byte = output[start + i];
                    output.push_back(byte);
                }
            }
        }
        if (original_length && output.size() >= *original_length) {
            break; // the rest would be truncated away
        }
    }
    if (original_length && output.size() > *original_length) {
        output.resize(*original_length);
    }
    return output;
}

// Serialise a token list to the CMP02 wire format.
inline std::vector<std::uint8_t> serialise(const std::vector<Token>& tokens,
                                           std::size_t original_length) {
    std::vector<std::uint8_t> buf;
    std::size_t block_count = (tokens.size() + 7) / 8;
    auto put_be32 = [&buf](std::uint32_t v) {
        buf.push_back(static_cast<std::uint8_t>((v >> 24) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>((v >> 16) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
        buf.push_back(static_cast<std::uint8_t>(v & 0xFF));
    };
    put_be32(static_cast<std::uint32_t>(original_length));
    put_be32(static_cast<std::uint32_t>(block_count));
    for (std::size_t blk = 0; blk < block_count; ++blk) {
        std::size_t base = blk * 8;
        std::size_t chunk = std::min<std::size_t>(tokens.size() - base, 8);
        std::uint8_t flag = 0;
        for (std::size_t bit = 0; bit < chunk; ++bit) {
            if (tokens[base + bit].is_match) {
                flag |= static_cast<std::uint8_t>(1u << bit);
            }
        }
        buf.push_back(flag);
        for (std::size_t bit = 0; bit < chunk; ++bit) {
            const Token& tk = tokens[base + bit];
            if (tk.is_match) {
                buf.push_back(static_cast<std::uint8_t>((tk.offset >> 8) & 0xFF));
                buf.push_back(static_cast<std::uint8_t>(tk.offset & 0xFF));
                buf.push_back(tk.length);
            } else {
                buf.push_back(tk.literal);
            }
        }
    }
    return buf;
}

// Deserialise CMP02 bytes into (tokens, original_length).
inline std::pair<std::vector<Token>, std::size_t> deserialise(
    const std::vector<std::uint8_t>& data) {
    if (data.size() < 8) {
        return {{}, 0};
    }
    std::size_t orig_len = (static_cast<std::size_t>(data[0]) << 24) |
                           (static_cast<std::size_t>(data[1]) << 16) |
                           (static_cast<std::size_t>(data[2]) << 8) |
                           static_cast<std::size_t>(data[3]);
    std::size_t block_count = (static_cast<std::size_t>(data[4]) << 24) |
                              (static_cast<std::size_t>(data[5]) << 16) |
                              (static_cast<std::size_t>(data[6]) << 8) |
                              static_cast<std::size_t>(data[7]);
    std::size_t max_possible = data.size() - 8;
    if (block_count > max_possible) {
        block_count = max_possible;
    }
    std::vector<Token> tokens;
    std::size_t pos = 8;
    for (std::size_t blk = 0; blk < block_count; ++blk) {
        if (pos >= data.size()) {
            break;
        }
        std::uint8_t flag = data[pos++];
        for (int bit = 0; bit < 8; ++bit) {
            if (pos >= data.size()) {
                break;
            }
            if (flag & static_cast<std::uint8_t>(1u << bit)) {
                if (pos + 3 > data.size()) {
                    break;
                }
                tokens.push_back(Token::match(
                    static_cast<std::uint16_t>(
                        (static_cast<unsigned>(data[pos]) << 8) | data[pos + 1]),
                    data[pos + 2]));
                pos += 3;
            } else {
                tokens.push_back(Token::lit(data[pos]));
                pos += 1;
            }
        }
    }
    return {tokens, orig_len};
}

// One-shot compress / decompress with the default parameters.
inline std::vector<std::uint8_t> compress(
    const std::vector<std::uint8_t>& data) {
    std::vector<Token> tokens =
        encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
    return serialise(tokens, data.size());
}

inline std::vector<std::uint8_t> decompress(
    const std::vector<std::uint8_t>& data) {
    auto pr = deserialise(data);
    return decode(pr.first, pr.second);
}

}  // namespace lzss
}  // namespace ca

#endif  // CA_LZSS_HPP
