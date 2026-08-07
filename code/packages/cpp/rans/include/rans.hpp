// rans.hpp — table-based rANS (range Asymmetric Numeral Systems) entropy
// coding, in pure ISO C++17, header-only, in namespace ca::rans. A faithful
// port of the Rust `rans` crate.
// ===========================================================================
//
// rANS is a modern entropy coder (the "A" in Zstandard/JPEG XL): it codes a
// stream of alphabet symbols against a fixed frequency table.
//
// AnsTable — built from raw symbol counts, normalised (largest-remainder) so the
//   frequencies sum to a power of two M = 2^k (M >= alphabet size, M <= 2^16);
//   a flat M-entry decode table gives O(1) lookup.
// RansEncoder — `put` symbols in REVERSE order (rANS is LIFO), then `finish`.
// RansDecoder — `get` symbols back in forward order.
//
// The decode table guarantees `slot - cumfreq` is in [0, freq) for every slot,
// so the decoder never goes out of bounds or underflows for any input state.
//
// Errors throw std::invalid_argument (table build / short decoder input) or
// std::out_of_range (an encoder symbol outside the alphabet).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Arithmetic is 64-bit (no 128-bit ints).
#ifndef CA_RANS_HPP
#define CA_RANS_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <numeric>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace rans {

class AnsTable {
public:
    // Build a table from raw (unnormalised) symbol counts.
    static AnsTable build(const std::vector<std::uint32_t>& counts) {
        if (counts.empty()) {
            throw std::invalid_argument("AnsTable: counts must not be empty");
        }
        if (counts.size() > 256) {
            throw std::invalid_argument("AnsTable: alphabet size exceeds 256");
        }
        std::size_t n = counts.size();
        std::uint64_t total = 0;
        for (std::uint32_t c : counts) {
            total += c;
        }
        if (total == 0) {
            throw std::invalid_argument("AnsTable: all counts are zero");
        }

        std::uint64_t min_m = std::max<std::uint64_t>(n, total);
        if (min_m < 1) {
            min_m = 1;
        }
        if (min_m > (1ull << 16)) {
            throw std::invalid_argument(
                "AnsTable: normalised table size M would exceed 2^16");
        }
        std::uint32_t log2m = 0, m;
        {
            std::uint64_t mm = 1;
            while (mm < min_m) {
                ++log2m;
                mm <<= 1;
            }
            m = static_cast<std::uint32_t>(mm);
        }

        std::vector<std::uint32_t> freq(n);
        std::uint32_t remainder = m;
        for (std::size_t i = 0; i < n; ++i) {
            freq[i] = static_cast<std::uint32_t>(
                (static_cast<std::uint64_t>(counts[i]) * m) / total);
            remainder -= freq[i];
        }

        // Distribute the remainder: zero-freq first, then descending fractional
        // part, then ascending index (a total order -> deterministic).
        std::vector<std::size_t> order(n);
        std::iota(order.begin(), order.end(), std::size_t(0));
        std::sort(order.begin(), order.end(), [&](std::size_t a, std::size_t b) {
            bool az = freq[a] == 0, bz = freq[b] == 0;
            if (az != bz) {
                return az;
            }
            std::uint64_t fa = (static_cast<std::uint64_t>(counts[a]) * m) % total;
            std::uint64_t fb = (static_cast<std::uint64_t>(counts[b]) * m) % total;
            if (fa != fb) {
                return fa > fb;
            }
            return a < b;
        });
        for (std::size_t i = 0; i < n && remainder > 0; ++i) {
            ++freq[order[i]];
            --remainder;
        }

        for (std::size_t i = 0; i < n; ++i) {
            if (freq[i] == 0) {
                throw std::invalid_argument(
                    "AnsTable: a symbol has zero frequency after normalisation");
            }
        }

        std::vector<std::uint32_t> cumfull(n + 1, 0);
        for (std::size_t i = 0; i < n; ++i) {
            cumfull[i + 1] = cumfull[i] + freq[i];
        }

        AnsTable t;
        t.n_ = n;
        t.m_ = m;
        t.log2m_ = log2m;
        t.freq_ = std::move(freq);
        t.cumfreq_.assign(cumfull.begin(), cumfull.begin() + static_cast<std::ptrdiff_t>(n));
        t.decode_sym_.assign(m, 0);
        t.decode_freq_.assign(m, 0);
        t.decode_cumfreq_.assign(m, 0);
        for (std::size_t sym = 0; sym < n; ++sym) {
            for (std::uint32_t slot = cumfull[sym]; slot < cumfull[sym + 1];
                 ++slot) {
                t.decode_sym_[slot] = static_cast<std::uint8_t>(sym);
                t.decode_freq_[slot] = t.freq_[sym];
                t.decode_cumfreq_[slot] = cumfull[sym];
            }
        }
        return t;
    }

    std::uint32_t m() const { return m_; }
    std::uint32_t log2m() const { return log2m_; }
    std::size_t alphabet_size() const { return n_; }
    std::optional<std::uint32_t> freq(std::size_t s) const {
        if (s >= n_) {
            return std::nullopt;
        }
        return freq_[s];
    }
    std::optional<std::uint32_t> cumfreq(std::size_t s) const {
        if (s >= n_) {
            return std::nullopt;
        }
        return cumfreq_[s];
    }

    // Internal accessors used by the encoder/decoder.
    std::uint32_t freq_at(std::size_t s) const { return freq_[s]; }
    std::uint32_t cumfreq_at(std::size_t s) const { return cumfreq_[s]; }
    std::uint8_t decode_sym(std::uint32_t slot) const { return decode_sym_[slot]; }
    std::uint32_t decode_freq(std::uint32_t slot) const { return decode_freq_[slot]; }
    std::uint32_t decode_cumfreq(std::uint32_t slot) const {
        return decode_cumfreq_[slot];
    }

private:
    std::size_t n_ = 0;
    std::vector<std::uint32_t> freq_;
    std::vector<std::uint32_t> cumfreq_;
    std::uint32_t m_ = 0;
    std::uint32_t log2m_ = 0;
    std::vector<std::uint8_t> decode_sym_;
    std::vector<std::uint32_t> decode_freq_;
    std::vector<std::uint32_t> decode_cumfreq_;
};

// rANS encoder. Symbols must be `put` in reverse order. Borrows `table` (it must
// outlive the encoder, like the crate's &AnsTable).
class RansEncoder {
public:
    explicit RansEncoder(const AnsTable& table)
        : table_(&table), x_(table.m()) {}

    void put(std::uint8_t symbol) {
        std::size_t s = symbol;
        if (s >= table_->alphabet_size()) {
            throw std::out_of_range("RansEncoder::put: symbol out of range");
        }
        std::uint64_t f = table_->freq_at(s);
        std::uint64_t m = table_->m();
        std::uint64_t upper = f << 8; // f * 256
        while (x_ >= upper) {
            pending_.push_back(static_cast<std::uint8_t>(x_ & 0xFF));
            x_ >>= 8;
        }
        x_ = (x_ / f) * m + table_->cumfreq_at(s) + (x_ % f);
    }

    // Flush and return the byte stream (8-byte big-endian state + renorm bytes).
    std::vector<std::uint8_t> finish() {
        std::uint64_t x = x_;
        for (int shift = 0; shift < 64; shift += 8) {
            pending_.push_back(static_cast<std::uint8_t>((x >> shift) & 0xFF));
        }
        std::reverse(pending_.begin(), pending_.end());
        return std::move(pending_);
    }

private:
    const AnsTable* table_;
    std::uint64_t x_;
    std::vector<std::uint8_t> pending_;
};

// rANS decoder. Owns a copy of the input bytes (the crate borrows &[u8]); this
// keeps `RansDecoder(table, enc.finish())` lifetime-safe. Borrows `table`.
class RansDecoder {
public:
    RansDecoder(const AnsTable& table, std::vector<std::uint8_t> data)
        : table_(&table), data_(std::move(data)), pos_(8), x_(0) {
        if (data_.size() < 8) {
            throw std::invalid_argument(
                "RansDecoder: data too short (need at least 8 bytes)");
        }
        x_ = (static_cast<std::uint64_t>(data_[0]) << 56) |
             (static_cast<std::uint64_t>(data_[1]) << 48) |
             (static_cast<std::uint64_t>(data_[2]) << 40) |
             (static_cast<std::uint64_t>(data_[3]) << 32) |
             (static_cast<std::uint64_t>(data_[4]) << 24) |
             (static_cast<std::uint64_t>(data_[5]) << 16) |
             (static_cast<std::uint64_t>(data_[6]) << 8) |
             static_cast<std::uint64_t>(data_[7]);
    }

    std::uint8_t get() {
        std::uint64_t m = table_->m();
        std::uint32_t slot = static_cast<std::uint32_t>(x_ % m); // < m
        std::uint8_t sym = table_->decode_sym(slot);
        std::uint64_t f = table_->decode_freq(slot);
        std::uint64_t cf = table_->decode_cumfreq(slot);
        x_ = f * (x_ / m) + (x_ % m) - cf; // (x_ % m) >= cf by construction
        while (x_ < m) {
            if (pos_ < data_.size()) {
                x_ = (x_ << 8) | data_[pos_];
                ++pos_;
            } else {
                x_ <<= 8;
                if (x_ == 0) {
                    // Stream exhausted and state stuck at 0 (malformed input):
                    // stop instead of looping forever.
                    break;
                }
            }
        }
        return sym;
    }

    bool is_exhausted() const { return pos_ >= data_.size(); }

private:
    const AnsTable* table_;
    std::vector<std::uint8_t> data_;
    std::size_t pos_;
    std::uint64_t x_;
};

}  // namespace rans
}  // namespace ca

#endif  // CA_RANS_HPP
