// range_coder.hpp — the VP8 boolean range coder (RFC 6386 §7), in pure ISO
// C++17, header-only, in namespace ca::range_coder. A faithful port of the Rust
// `range-coder` crate.
// ===========================================================================
//
// A boolean range coder (binary arithmetic coder) compresses a sequence of
// bits, each with an 8-bit probability that the bit is 0 (`prob`, 128 = 50/50).
// It is the entropy stage of VP8 / WebP.
//
//   split = 1 + (((range - 1) * prob) >> 8)   // the +1 keeps both halves
//   bit==0 -> lower sub-interval, bit==1 -> upper
//
// Encode a sequence with BoolEncoder, `finish` to bytes; decode with BoolDecoder
// using the same probabilities to recover the bits. Bits are MSB-first; an
// exhausted stream reads as zeros.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_RANGE_CODER_HPP
#define CA_RANGE_CODER_HPP

#include <cstddef>
#include <cstdint>
#include <vector>

namespace ca {
namespace range_coder {

// VP8 boolean range encoder.
class BoolEncoder {
public:
    BoolEncoder() : bottom_(0), range_(255), bit_count_(-24) {}

    // Encode one bit; `prob` is the probability the bit is 0 (0..255).
    void write_bit(bool bit, std::uint8_t prob) {
        std::uint32_t split =
            1u + (((range_ - 1u) * static_cast<std::uint32_t>(prob)) >> 8);
        if (bit) {
            bottom_ += split; // upper sub-interval
            range_ -= split;
        } else {
            range_ = split; // lower sub-interval
        }
        while (range_ < 128u) {
            range_ <<= 1;
            bottom_ <<= 1;
            bit_count_ += 1;
            if (bit_count_ == 0) {
                output_.push_back(
                    static_cast<std::uint8_t>((bottom_ >> 24) & 0xFFu));
                bottom_ &= 0x00FFFFFFull;
                bit_count_ = -8;
            }
        }
    }

    // Encode the low `n` bits of `value`, MSB first, with prob = 128. n <= 32.
    void write_bits(std::uint32_t value, std::uint8_t n) {
        for (int i = static_cast<int>(n) - 1; i >= 0; --i) {
            // Guard the shift: bits at i >= 32 are 0 (value is 32-bit), which
            // also avoids shift-width UB if a caller ignores the n <= 32
            // contract.
            bool bit = (i < 32) && (((value >> i) & 1u) != 0);
            write_bit(bit, 128);
        }
    }

    // Flush 32 zero bits and return the encoded bytes.
    std::vector<std::uint8_t> finish() {
        for (int i = 0; i < 32; ++i) {
            write_bit(false, 128);
        }
        return std::move(output_);
    }

private:
    std::uint64_t bottom_;
    std::uint32_t range_;
    int bit_count_;
    std::vector<std::uint8_t> output_;
};

// VP8 boolean range decoder. Owns a copy of the input bytes (unlike the crate's
// borrowed &[u8]): borrowing a temporary in C++ — e.g. `BoolDecoder(enc.finish())`
// — would dangle, so the decoder holds its own buffer for lifetime safety.
class BoolDecoder {
public:
    explicit BoolDecoder(std::vector<std::uint8_t> data)
        : data_(std::move(data)),
          pos_(2),
          bit_pos_(0),
          range_(255),
          value_(0) {
        if (data_.size() >= 2) {
            value_ = (static_cast<std::uint32_t>(data_[0]) << 8) |
                     static_cast<std::uint32_t>(data_[1]);
        } else if (data_.size() == 1) {
            value_ = static_cast<std::uint32_t>(data_[0]) << 8;
        }
    }
    BoolDecoder(const std::uint8_t* data, std::size_t len)
        : BoolDecoder(std::vector<std::uint8_t>(data, data + len)) {}

    // Decode one bit; returns true (1) or false (0).
    bool read_bit(std::uint8_t prob) {
        std::uint32_t split =
            1u + (((range_ - 1u) * static_cast<std::uint32_t>(prob)) >> 8);
        std::uint32_t bigsplit = split << 8;
        bool bit;
        if (value_ >= bigsplit) {
            range_ -= split;
            value_ -= bigsplit;
            bit = true;
        } else {
            range_ = split;
            bit = false;
        }
        while (range_ < 128u) {
            range_ <<= 1;
            value_ = (value_ << 1) | next_msb_bit();
        }
        return bit;
    }

    // Decode `n` bits (prob = 128) MSB-first; n == 0 returns 0.
    std::uint32_t read_bits(std::uint8_t n) {
        std::uint32_t result = 0;
        for (std::uint8_t i = 0; i < n; ++i) {
            result = (result << 1) | static_cast<std::uint32_t>(read_bit(128));
        }
        return result;
    }

    bool is_exhausted() const { return pos_ >= data_.size(); }

private:
    std::uint32_t next_msb_bit() {
        if (pos_ >= data_.size()) {
            return 0; // pad exhausted stream with zeros
        }
        std::uint8_t byte = data_[pos_];
        std::uint32_t bit =
            (static_cast<std::uint32_t>(byte) >> (7 - bit_pos_)) & 1u;
        ++bit_pos_;
        if (bit_pos_ == 8) {
            bit_pos_ = 0;
            ++pos_;
        }
        return bit;
    }

    std::vector<std::uint8_t> data_;
    std::size_t pos_;
    std::uint8_t bit_pos_;
    std::uint32_t range_;
    std::uint32_t value_;
};

}  // namespace range_coder
}  // namespace ca

#endif  // CA_RANGE_CODER_HPP
