// lz77.hpp — LZ77 sliding-window compression, in pure ISO C++17 (header-only). A
// faithful port of the Rust `lz77` crate.
// ===========================================================================
//
// Compresses by replacing repeated byte runs with backreferences. The output is
// a stream of (offset, length, next_char) tokens: "copy `length` bytes from
// `offset` back, then append next_char"; a literal is (0, 0, byte). Decoding
// copies byte-by-byte so overlapping matches expand correctly. `compress` /
// `decompress` add a compact serialisation (u32 BE count + 4 bytes per token).
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef LZ77_HPP
#define LZ77_HPP

#include <cstddef>
#include <cstdint>
#include <vector>

namespace ca {
namespace lz77 {

struct token {
    std::uint16_t offset;
    std::uint8_t length;
    std::uint8_t next_char;
};

inline bool operator==(const token &a, const token &b) {
    return a.offset == b.offset && a.length == b.length &&
           a.next_char == b.next_char;
}

constexpr std::size_t default_window = 4096;
constexpr std::size_t default_max_match = 255;
constexpr std::size_t default_min_match = 3;

namespace detail {
inline void find_longest_match(const std::vector<std::uint8_t> &data,
                               std::size_t cursor, std::size_t window_size,
                               std::size_t max_match, std::size_t &best_offset,
                               std::size_t &best_length) {
    std::size_t search_start = cursor > window_size ? cursor - window_size : 0;
    std::size_t lookahead_end = cursor + max_match;
    if (lookahead_end > data.size() - 1) {
        lookahead_end = data.size() - 1; // last byte reserved as a next_char
    }
    best_offset = 0;
    best_length = 0;
    for (std::size_t pos = search_start; pos < cursor; pos++) {
        std::size_t length = 0;
        while (cursor + length < lookahead_end &&
               data[pos + length] == data[cursor + length]) {
            length++;
        }
        if (length > best_length) {
            best_length = length;
            best_offset = cursor - pos;
        }
    }
}
} // namespace detail

inline std::vector<token> encode(const std::vector<std::uint8_t> &data,
                                 std::size_t window_size, std::size_t max_match,
                                 std::size_t min_match) {
    std::vector<token> tokens;
    std::size_t cursor = 0;
    while (cursor < data.size()) {
        if (cursor == data.size() - 1) {
            tokens.push_back(token{0, 0, data[cursor]});
            cursor += 1;
            continue;
        }
        std::size_t offset, length;
        detail::find_longest_match(data, cursor, window_size, max_match, offset,
                                   length);
        if (length >= min_match) {
            tokens.push_back(token{static_cast<std::uint16_t>(offset),
                                   static_cast<std::uint8_t>(length),
                                   data[cursor + length]});
            cursor += length + 1;
        } else {
            tokens.push_back(token{0, 0, data[cursor]});
            cursor += 1;
        }
    }
    return tokens;
}

inline std::vector<std::uint8_t> decode(const std::vector<token> &tokens,
                                        const std::vector<std::uint8_t> &initial) {
    std::vector<std::uint8_t> out = initial;
    for (const token &t : tokens) {
        if (t.length > 0) {
            std::size_t start = out.size() - t.offset;
            for (std::size_t i = 0; i < t.length; i++) {
                out.push_back(out[start + i]); // copy handles overlap
            }
        }
        out.push_back(t.next_char);
    }
    return out;
}

inline std::vector<std::uint8_t> serialise(const std::vector<token> &tokens) {
    std::vector<std::uint8_t> buf;
    buf.reserve(4 + tokens.size() * 4);
    std::uint32_t count = static_cast<std::uint32_t>(tokens.size());
    buf.push_back(static_cast<std::uint8_t>(count >> 24));
    buf.push_back(static_cast<std::uint8_t>(count >> 16));
    buf.push_back(static_cast<std::uint8_t>(count >> 8));
    buf.push_back(static_cast<std::uint8_t>(count));
    for (const token &t : tokens) {
        buf.push_back(static_cast<std::uint8_t>(t.offset >> 8));
        buf.push_back(static_cast<std::uint8_t>(t.offset));
        buf.push_back(t.length);
        buf.push_back(t.next_char);
    }
    return buf;
}

inline std::vector<token> deserialise(const std::vector<std::uint8_t> &data) {
    std::vector<token> tokens;
    if (data.size() < 4) {
        return tokens;
    }
    std::size_t declared = (static_cast<std::size_t>(data[0]) << 24) |
                           (static_cast<std::size_t>(data[1]) << 16) |
                           (static_cast<std::size_t>(data[2]) << 8) |
                           static_cast<std::size_t>(data[3]);
    std::size_t actual = (data.size() - 4) / 4;
    if (actual > declared) {
        actual = declared;
    }
    tokens.reserve(actual);
    for (std::size_t i = 0; i < actual; i++) {
        std::size_t base = 4 + i * 4;
        tokens.push_back(token{
            static_cast<std::uint16_t>((static_cast<std::uint16_t>(data[base]) << 8) |
                                       data[base + 1]),
            data[base + 2], data[base + 3]});
    }
    return tokens;
}

inline std::vector<std::uint8_t> compress(const std::vector<std::uint8_t> &data,
                                          std::size_t window_size,
                                          std::size_t max_match,
                                          std::size_t min_match) {
    return serialise(encode(data, window_size, max_match, min_match));
}

inline std::vector<std::uint8_t> decompress(const std::vector<std::uint8_t> &data) {
    return decode(deserialise(data), {});
}

} // namespace lz77
} // namespace ca

#endif // LZ77_HPP
