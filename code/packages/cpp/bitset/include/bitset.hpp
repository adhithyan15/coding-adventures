// bitset.hpp — a growable set of bits packed into 64-bit words, in pure ISO
// C++17 (header-only). A faithful port of the Rust `bitset` crate.
// ===========================================================================
//
// Stores bits (indexed from 0) packed 64 to a word. set/toggle auto-grow (with
// capacity doubling); clear/test treat out-of-range indices as unset. The
// bitwise set operations (and/or/xor/not/and_not) return a new bitset. Bit 0 is
// the least-significant bit: from_binary_string("101") sets bits 0 and 2
// (value 5) and to_binary_string prints most-significant bit first.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef BITSET_HPP
#define BITSET_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {

class bitset {
public:
    static constexpr std::size_t bits_per_word = 64;

    // An all-zero bitset of `size` bits.
    explicit bitset(std::size_t size = 0)
        : words_(words_needed(size), 0), len_(size) {}

    // Build from a 128-bit value (high << 64 | low). Zero → empty bitset.
    static bitset from_integer(std::uint64_t low, std::uint64_t high = 0) {
        bitset b;
        if (low == 0 && high == 0) {
            return b;
        }
        if (high != 0) {
            b = bitset(128);
            b.words_[0] = low;
            b.words_[1] = high;
        } else {
            b = bitset(64);
            b.words_[0] = low;
        }
        return b;
    }

    // Parse a most-significant-bit-first '0'/'1' string (its length is the bit
    // length). Throws std::invalid_argument on a non-binary character.
    static bitset from_binary_string(const std::string &s) {
        for (char ch : s) {
            if (ch != '0' && ch != '1') {
                throw std::invalid_argument("bitset::from_binary_string: not binary");
            }
        }
        bitset b(s.size());
        for (std::size_t k = 0; k < s.size(); k++) {
            if (s[s.size() - 1 - k] == '1') {
                b.words_[word_index(k)] |= bitmask(k);
            }
        }
        b.clean_trailing_bits();
        return b;
    }

    // --- single-bit operations ---
    void set(std::size_t i) {
        ensure_capacity(i);
        words_[word_index(i)] |= bitmask(i);
    }
    void toggle(std::size_t i) {
        ensure_capacity(i);
        words_[word_index(i)] ^= bitmask(i);
        clean_trailing_bits();
    }
    void clear(std::size_t i) {
        if (i < len_) {
            words_[word_index(i)] &= ~bitmask(i);
        }
    }
    bool test(std::size_t i) const {
        if (i >= len_) {
            return false;
        }
        return (words_[word_index(i)] & bitmask(i)) != 0;
    }

    // --- bitwise set operations (return a new bitset) ---
    bitset operator&(const bitset &o) const { return binary_op(o, Op::And); }
    bitset operator|(const bitset &o) const { return binary_op(o, Op::Or); }
    bitset operator^(const bitset &o) const { return binary_op(o, Op::Xor); }
    bitset and_not(const bitset &o) const { return binary_op(o, Op::AndNot); }
    bitset operator~() const {
        bitset out;
        out.len_ = len_;
        out.words_.resize(words_.size());
        for (std::size_t i = 0; i < words_.size(); i++) {
            out.words_[i] = ~words_[i];
        }
        out.clean_trailing_bits();
        return out;
    }

    // --- queries ---
    std::size_t popcount() const {
        std::size_t total = 0;
        for (std::uint64_t w : words_) {
            total += popcount64(w);
        }
        return total;
    }
    std::size_t size() const { return len_; }
    std::size_t capacity() const { return words_.size() * bits_per_word; }
    bool any() const {
        for (std::uint64_t w : words_) {
            if (w != 0) {
                return true;
            }
        }
        return false;
    }
    bool none() const { return !any(); }
    bool empty() const { return len_ == 0; }
    bool all() const {
        if (len_ == 0) {
            return true;
        }
        for (std::size_t i = 0; i + 1 < words_.size(); i++) {
            if (words_[i] != ~std::uint64_t{0}) {
                return false;
            }
        }
        std::size_t remaining = len_ % bits_per_word;
        std::uint64_t last = words_.back();
        if (remaining == 0) {
            return last == ~std::uint64_t{0};
        }
        return last == ((std::uint64_t{1} << remaining) - 1);
    }

    // If no bit beyond index 63 is set, returns the low 64 bits, else nullopt.
    std::optional<std::uint64_t> to_integer() const {
        if (len_ == 0) {
            return std::uint64_t{0};
        }
        for (std::size_t i = 1; i < words_.size(); i++) {
            if (words_[i] != 0) {
                return std::nullopt;
            }
        }
        return words_[0];
    }

    // Most-significant-bit-first '0'/'1' string of length size().
    std::string to_binary_string() const {
        std::string s;
        s.reserve(len_);
        for (std::size_t i = len_; i-- > 0;) {
            s.push_back(test(i) ? '1' : '0');
        }
        return s;
    }

private:
    std::vector<std::uint64_t> words_;
    std::size_t len_;

    enum class Op { And, Or, Xor, AndNot };

    static std::size_t words_needed(std::size_t bits) {
        return (bits + (bits_per_word - 1)) / bits_per_word;
    }
    static std::size_t word_index(std::size_t i) { return i / bits_per_word; }
    static std::uint64_t bitmask(std::size_t i) {
        return std::uint64_t{1} << (i % bits_per_word);
    }
    static std::size_t popcount64(std::uint64_t w) {
        std::size_t c = 0;
        while (w != 0) {
            w &= w - 1;
            c++;
        }
        return c;
    }

    void clean_trailing_bits() {
        if (len_ == 0 || words_.empty()) {
            return;
        }
        std::size_t remaining = len_ % bits_per_word;
        if (remaining != 0) {
            words_.back() &= (std::uint64_t{1} << remaining) - 1;
        }
    }

    void ensure_capacity(std::size_t i) {
        if (i < capacity()) {
            if (i >= len_) {
                len_ = i + 1;
            }
            return;
        }
        if (i == (std::size_t)-1) {
            throw std::length_error("bitset: index too large"); // len = i+1 overflows
        }
        // Double capacity, but stop before size_t overflow (which would wrap to
        // 0 and spin forever). If doubling can't reach i, size to cover bit i.
        std::size_t cap = capacity();
        std::size_t new_cap = cap > bits_per_word ? cap : bits_per_word;
        const std::size_t max_size = (std::size_t)-1;
        while (new_cap <= i && new_cap <= max_size / 2) {
            new_cap *= 2;
        }
        std::size_t new_words =
            (new_cap > i) ? (new_cap / bits_per_word) : (i / bits_per_word + 1);
        words_.resize(new_words, 0); // std::vector throws on an impossible size
        len_ = i + 1;
    }

    bitset binary_op(const bitset &o, Op op) const {
        bitset out;
        out.len_ = len_ > o.len_ ? len_ : o.len_;
        std::size_t max_words =
            words_.size() > o.words_.size() ? words_.size() : o.words_.size();
        out.words_.assign(max_words, 0);
        for (std::size_t i = 0; i < max_words; i++) {
            std::uint64_t av = i < words_.size() ? words_[i] : 0;
            std::uint64_t bv = i < o.words_.size() ? o.words_[i] : 0;
            switch (op) {
            case Op::And:
                out.words_[i] = av & bv;
                break;
            case Op::Or:
                out.words_[i] = av | bv;
                break;
            case Op::Xor:
                out.words_[i] = av ^ bv;
                break;
            case Op::AndNot:
                out.words_[i] = av & ~bv;
                break;
            }
        }
        out.clean_trailing_bits();
        return out;
    }
};

} // namespace ca

#endif // BITSET_HPP
