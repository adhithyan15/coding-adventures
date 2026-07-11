// wasm_leb128.hpp — LEB128 variable-length integer coding, in pure ISO C++17,
// header-only, in namespace ca::leb128. A faithful port of the Rust
// `wasm-leb128` crate.
// ===========================================================================
//
// LEB128 ("Little-Endian Base 128") is the varint format used by WebAssembly,
// DWARF, and Android DEX. Each byte holds 7 data bits; the high bit (0x80) is a
// continuation flag, set on every byte but the last, and groups are emitted
// least-significant first.
//
//   624485 -> 0xE5 0x8E 0x26
//
// Unsigned values are zero-extended. Signed values use two's complement: the
// encoder stops once the remaining bits are all-0/all-1 and the last group's
// sign bit agrees; the decoder sign-extends from the last group's bit 6.
//
//   -2 -> 0x7E
//
// Encoding returns a std::vector (never fails). Decoding returns
// {value, bytes_consumed} on success and throws ca::leb128::Error on a bad
// offset, an over-wide sequence, or an unterminated one.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_WASM_LEB128_HPP
#define CA_WASM_LEB128_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace leb128 {

// Thrown when decoding fails; `offset` is the decode start offset (matching the
// Rust `Leb128Error`).
class Error : public std::runtime_error {
public:
    Error(const std::string& message, std::size_t offset)
        : std::runtime_error("LEB128 error at offset " +
                             std::to_string(offset) + ": " + message),
          offset(offset) {}
    std::size_t offset;
};

// ---- encoding ---------------------------------------------------------

inline std::vector<std::uint8_t> encode_unsigned(std::uint64_t value) {
    std::vector<std::uint8_t> result;
    for (;;) {
        std::uint8_t byte = static_cast<std::uint8_t>(value & 0x7Fu);
        value >>= 7;
        if (value != 0) {
            byte |= 0x80u;
        }
        result.push_back(byte);
        if (value == 0) {
            break;
        }
    }
    return result;
}

inline std::vector<std::uint8_t> encode_signed(std::int64_t value) {
    std::vector<std::uint8_t> result;
    bool done = false;
    while (!done) {
        std::uint8_t byte =
            static_cast<std::uint8_t>(static_cast<std::uint64_t>(value) & 0x7Fu);
        // Arithmetic right shift by 7 (sign-propagating), spelled to be
        // well-defined on every target regardless of the platform's
        // signed-shift behaviour.
        std::uint64_t u;
        std::memcpy(&u, &value, sizeof u);
        u >>= 7;
        if (value < 0) {
            u |= ~(~std::uint64_t(0) >> 7); // set the vacated top 7 bits
        }
        std::memcpy(&value, &u, sizeof value);

        done = (value == 0 && (byte & 0x40) == 0) ||
               (value == -1 && (byte & 0x40) != 0);
        if (!done) {
            byte |= 0x80u;
        }
        result.push_back(byte);
    }
    return result;
}

// ---- decoding ---------------------------------------------------------

// Decode an unsigned LEB128 value from data[offset .. len); returns
// {value, bytes_consumed}. Throws Error on failure.
inline std::pair<std::uint64_t, std::size_t> decode_unsigned(
    const std::uint8_t* data, std::size_t len, std::size_t offset) {
    if (offset >= len) {
        throw Error("offset is out of bounds for the data", offset);
    }
    std::uint64_t value = 0;
    std::uint32_t shift = 0;
    std::size_t consumed = 0;
    for (std::size_t i = offset; i < len; ++i) {
        std::uint8_t byte = data[i];
        value |= static_cast<std::uint64_t>(byte & 0x7Fu) << shift;
        ++consumed;
        shift += 7;
        if ((byte & 0x80u) == 0) {
            return {value, consumed};
        }
        if (shift >= 70) {
            throw Error("LEB128 sequence exceeds maximum u64 width (70 bits)",
                        offset);
        }
    }
    throw Error("unexpected end of data: LEB128 sequence is unterminated",
                offset);
}

// Decode a signed LEB128 value (sign-extending as needed). Throws Error on
// failure.
inline std::pair<std::int64_t, std::size_t> decode_signed(
    const std::uint8_t* data, std::size_t len, std::size_t offset) {
    if (offset >= len) {
        throw Error("offset is out of bounds for the data", offset);
    }
    std::uint64_t value = 0;
    std::uint32_t shift = 0;
    std::size_t consumed = 0;
    for (std::size_t i = offset; i < len; ++i) {
        std::uint8_t byte = data[i];
        value |= static_cast<std::uint64_t>(byte & 0x7Fu) << shift;
        ++consumed;
        shift += 7;
        if ((byte & 0x80u) == 0) {
            if (shift < 64 && (byte & 0x40u) != 0) {
                value |= (~std::uint64_t(0)) << shift; // sign-extend
            }
            std::int64_t signed_val;
            std::memcpy(&signed_val, &value, sizeof signed_val);
            return {signed_val, consumed};
        }
        if (shift >= 70) {
            throw Error("LEB128 sequence exceeds maximum i64 width (70 bits)",
                        offset);
        }
    }
    throw Error("unexpected end of data: LEB128 sequence is unterminated",
                offset);
}

// Convenience overloads over a std::vector.
inline std::pair<std::uint64_t, std::size_t> decode_unsigned(
    const std::vector<std::uint8_t>& data, std::size_t offset) {
    return decode_unsigned(data.data(), data.size(), offset);
}
inline std::pair<std::int64_t, std::size_t> decode_signed(
    const std::vector<std::uint8_t>& data, std::size_t offset) {
    return decode_signed(data.data(), data.size(), offset);
}

}  // namespace leb128
}  // namespace ca

#endif  // CA_WASM_LEB128_HPP
