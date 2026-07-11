// lzw.hpp — LZW compression with variable-width codes, in pure ISO C++17
// (header-only). A faithful port of the Rust `lzw` crate.
// ===========================================================================
//
// Builds a dictionary from the 256 single bytes, adding one entry per step;
// codes start at 9 bits and grow to 16 as the dictionary fills. The stream opens
// with CLEAR (256), closes with STOP (257), and resets with another CLEAR when
// full. Wire format: a 4-byte big-endian original length, then the LSB-first
// bit-packed code stream.
//
// The encoder keys its dictionary on (prefix_code, byte), which assigns codes in
// the same order as the crate's byte-sequence map — so the output is identical.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef LZW_HPP
#define LZW_HPP

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <unordered_map>
#include <vector>

namespace ca {
namespace lzw {

namespace detail {
constexpr unsigned clear_code = 256;
constexpr unsigned stop_code = 257;
constexpr unsigned initial_next_code = 258;
constexpr unsigned initial_code_size = 9;
constexpr unsigned max_code_size = 16;
constexpr std::uint32_t max_entries = 1u << max_code_size; // 65536

// LSB-first bit writer.
struct bit_writer {
    std::vector<std::uint8_t> bytes;
    std::uint64_t buffer = 0;
    unsigned bit_pos = 0;
    void write(unsigned code, unsigned size) {
        buffer |= static_cast<std::uint64_t>(code) << bit_pos;
        bit_pos += size;
        while (bit_pos >= 8) {
            bytes.push_back(static_cast<std::uint8_t>(buffer & 0xff));
            buffer >>= 8;
            bit_pos -= 8;
        }
    }
    void flush() {
        if (bit_pos > 0) {
            bytes.push_back(static_cast<std::uint8_t>(buffer & 0xff));
            buffer = 0;
            bit_pos = 0;
        }
    }
};

// LSB-first bit reader. read() returns false at end of stream.
struct bit_reader {
    const std::uint8_t *data;
    std::size_t len;
    std::size_t pos = 0;
    std::uint64_t buffer = 0;
    unsigned bit_pos = 0;
    bool read(unsigned size, unsigned &code) {
        while (bit_pos < size) {
            if (pos >= len) {
                if (bit_pos == 0) {
                    return false;
                }
                break;
            }
            buffer |= static_cast<std::uint64_t>(data[pos]) << bit_pos;
            pos++;
            bit_pos += 8;
        }
        if (bit_pos < size) {
            return false;
        }
        std::uint64_t mask = (static_cast<std::uint64_t>(1) << size) - 1;
        code = static_cast<unsigned>(buffer & mask);
        buffer >>= size;
        bit_pos -= size;
        return true;
    }
};
} // namespace detail

inline std::vector<std::uint8_t> compress(const std::vector<std::uint8_t> &data) {
    using namespace detail;
    std::unordered_map<std::uint32_t, std::uint32_t> dict; // (prefix<<8|byte) → code
    std::uint32_t next_code = initial_next_code;
    unsigned code_size = initial_code_size;
    bit_writer w;
    long w_code = -1;

    w.write(clear_code, code_size);
    for (std::uint8_t b : data) {
        if (w_code < 0) {
            w_code = b;
            continue;
        }
        std::uint32_t key = (static_cast<std::uint32_t>(w_code) << 8) | b;
        auto it = dict.find(key);
        if (it != dict.end()) {
            w_code = it->second;
            continue;
        }
        w.write(static_cast<unsigned>(w_code), code_size);
        if (next_code < max_entries) {
            dict.emplace(key, next_code);
            next_code++;
            if (next_code > (1u << code_size) && code_size < max_code_size) {
                code_size++;
            }
        } else {
            w.write(clear_code, code_size);
            dict.clear();
            next_code = initial_next_code;
            code_size = initial_code_size;
        }
        w_code = b;
    }
    if (w_code >= 0) {
        w.write(static_cast<unsigned>(w_code), code_size);
    }
    w.write(stop_code, code_size);
    w.flush();

    std::vector<std::uint8_t> out;
    out.reserve(4 + w.bytes.size());
    std::uint32_t original_length = static_cast<std::uint32_t>(data.size());
    out.push_back(static_cast<std::uint8_t>(original_length >> 24));
    out.push_back(static_cast<std::uint8_t>(original_length >> 16));
    out.push_back(static_cast<std::uint8_t>(original_length >> 8));
    out.push_back(static_cast<std::uint8_t>(original_length));
    out.insert(out.end(), w.bytes.begin(), w.bytes.end());
    return out;
}

// Throws std::invalid_argument on a malformed stream.
inline std::vector<std::uint8_t> decompress(const std::vector<std::uint8_t> &data) {
    using namespace detail;
    if (data.size() < 4) {
        throw std::invalid_argument("lzw: input too short (missing header)");
    }
    std::size_t original_length =
        (static_cast<std::size_t>(data[0]) << 24) |
        (static_cast<std::size_t>(data[1]) << 16) |
        (static_cast<std::size_t>(data[2]) << 8) | data[3];

    // dict[code] = byte sequence. Seed 0..255; 256/257 are placeholders.
    std::vector<std::vector<std::uint8_t>> dict;
    dict.reserve(4096);
    for (unsigned b = 0; b < 256; b++) {
        dict.push_back({static_cast<std::uint8_t>(b)});
    }
    dict.emplace_back(); // 256 CLEAR
    dict.emplace_back(); // 257 STOP

    std::uint32_t next_code = initial_next_code;
    unsigned code_size = initial_code_size;
    long prev_code = -1;
    std::vector<std::uint8_t> output;
    bit_reader r{data.data() + 4, data.size() - 4};

    unsigned code;
    if (!r.read(code_size, code) || code != clear_code) {
        throw std::invalid_argument("lzw: expected CLEAR at start");
    }
    while (r.read(code_size, code)) {
        if (code == clear_code) {
            dict.resize(initial_next_code);
            next_code = initial_next_code;
            code_size = initial_code_size;
            prev_code = -1;
            continue;
        }
        if (code == stop_code) {
            break;
        }
        std::vector<std::uint8_t> entry;
        if (code < dict.size()) {
            entry = dict[code];
        } else if (code == dict.size() && prev_code >= 0) {
            entry = dict[prev_code];
            entry.push_back(dict[prev_code][0]);
        } else {
            throw std::invalid_argument("lzw: malformed code stream");
        }
        output.insert(output.end(), entry.begin(), entry.end());

        if (next_code < max_entries) {
            next_code++;
            if (next_code > (1u << code_size) && code_size < max_code_size) {
                code_size++;
            }
        }
        if (prev_code >= 0 && dict.size() < max_entries) {
            std::vector<std::uint8_t> new_entry = dict[prev_code];
            new_entry.push_back(entry[0]);
            dict.push_back(std::move(new_entry));
        }
        prev_code = static_cast<long>(code);
    }
    if (output.size() > original_length) {
        output.resize(original_length);
    }
    return output;
}

} // namespace lzw
} // namespace ca

#endif // LZW_HPP
