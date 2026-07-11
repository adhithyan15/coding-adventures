// bloom_filter.hpp — a Bloom filter for probabilistic membership tests, in pure
// ISO C++17 (header-only). A faithful port of the Rust `bloom-filter` crate
// (DT22).
// ===========================================================================
//
// A Bloom filter is a compact, probabilistic "set": it answers "definitely not
// present" or "possibly present", never giving a false negative but occasionally
// a false positive. It holds only a bit array of `m` bits; each element sets `k`
// bits chosen by `k` hash functions.
//
//   add(x):      set the k bits h_1(x)..h_k(x)
//   contains(x): true iff ALL k of those bits are set
//
// The k indices come from double hashing, index_i = h1 + i*h2 (mod m), with h1
// and h2 derived from two independent hashes (FNV-1a and djb2) finalised through
// fmix32 — matching the Rust crate.
//
// Sizing: m = ceil(-n ln p / (ln 2)^2), k = round((m/n) ln 2). ISO C++ has no
// natural log in the core language and the strict harness does not link libm, so
// this header carries a small, self-contained `ln`.
//
// Note on hashing: the Rust crate hashes an element's Debug string; this port
// hashes the raw bytes you pass to add()/contains(). The filter is
// self-consistent (add and contains hash identically), so the no-false-negatives
// guarantee holds — only the concrete bit patterns differ between ports.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef BLOOM_FILTER_HPP
#define BLOOM_FILTER_HPP

#include <cfloat> // DBL_MAX (to reject non-finite inputs without libm)
#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {

namespace detail {

constexpr double LN2 = 0.6931471805599453;
// 2^53: beyond this a double→integer cast is unreliable, so we saturate.
constexpr double DBL_INT_LIMIT = 9007199254740992.0;

// Natural log for x > 0, via range reduction x = m·2^e (m in [1,2)) and the
// series ln(m) = 2·atanh(t), t = (m-1)/(m+1) in [0, 1/3].
inline double iso_ln(double x) {
    // Reject non-finite / out-of-domain input: x <= 0 and NaN both fail the
    // (x > 0.0) test; +inf is caught by (x > DBL_MAX). This keeps the halving
    // loop below finite (inf * 0.5 == inf would spin forever).
    if (!(x > 0.0) || x > DBL_MAX) {
        return 0.0;
    }
    int e = 0;
    while (x >= 2.0) {
        x *= 0.5;
        e++;
    }
    while (x < 1.0) {
        x *= 2.0;
        e--;
    }
    double t = (x - 1.0) / (x + 1.0);
    double t2 = t * t;
    double term = t;
    double sum = 0.0;
    for (int k = 1; k <= 25; k += 2) {
        sum += term / static_cast<double>(k);
        term *= t2;
    }
    return 2.0 * sum + static_cast<double>(e) * LN2;
}

inline std::size_t d_to_size(double x, double bias) {
    if (x != x) {
        return 0; // NaN → not a valid size (casting NaN to an int is UB)
    }
    if (x <= 0.0) {
        return 0;
    }
    if (x >= DBL_INT_LIMIT) {
        return static_cast<std::size_t>(-1);
    }
    return static_cast<std::size_t>(
        static_cast<unsigned long long>(x + bias));
}
inline std::size_t d_ceil_size(double x) {
    if (x != x) {
        return 0; // NaN guard (see d_to_size)
    }
    if (x <= 0.0) {
        return 0;
    }
    if (x >= DBL_INT_LIMIT) {
        return static_cast<std::size_t>(-1);
    }
    auto t = static_cast<std::size_t>(static_cast<unsigned long long>(x));
    if (static_cast<double>(t) < x) {
        t += 1;
    }
    return t;
}

inline std::uint32_t fnv1a_32(const std::uint8_t *d, std::size_t n) {
    std::uint32_t h = 0x811c9dc5u;
    for (std::size_t i = 0; i < n; i++) {
        h ^= d[i];
        h *= 0x01000193u;
    }
    return h;
}
inline std::uint64_t djb2(const std::uint8_t *d, std::size_t n) {
    std::uint64_t h = 5381u;
    for (std::size_t i = 0; i < n; i++) {
        h = (h << 5) + h + d[i];
    }
    return h;
}
inline std::uint32_t fmix32(std::uint32_t h) {
    h ^= h >> 16;
    h *= 0x85ebca6bu;
    h ^= h >> 13;
    h *= 0xc2b2ae35u;
    h ^= h >> 16;
    return h;
}

} // namespace detail

// The reason a construction was rejected (mirrors Rust's BloomFilterError).
enum class bloom_error {
    invalid_expected_items,
    invalid_false_positive_rate,
    invalid_bit_count,
    invalid_hash_count
};

class bloom_filter {
public:
    // Size a filter for `expected_items` and false-positive rate `p` in (0,1).
    // Throws std::invalid_argument on a bad parameter (like Rust's `new`).
    explicit bloom_filter(std::size_t expected_items = 1000,
                          double false_positive_rate = 0.01) {
        auto err = validate_rate(expected_items, false_positive_rate);
        if (err) {
            throw std::invalid_argument("bloom_filter: invalid configuration");
        }
        std::size_t m = optimal_m(expected_items, false_positive_rate);
        std::size_t k = optimal_k(m, expected_items);
        init(m, k, expected_items);
    }

    // Non-throwing variant returning std::nullopt on a bad parameter.
    static std::optional<bloom_filter> try_create(std::size_t expected_items,
                                                  double false_positive_rate) {
        if (validate_rate(expected_items, false_positive_rate)) {
            return std::nullopt;
        }
        std::size_t m = optimal_m(expected_items, false_positive_rate);
        std::size_t k = optimal_k(m, expected_items);
        return bloom_filter(m, k, expected_items, tag{});
    }

    // Build from explicit bit and hash counts (both > 0).
    static bloom_filter from_params(std::size_t bit_count,
                                    std::size_t hash_count) {
        if (bit_count == 0 || hash_count == 0) {
            throw std::invalid_argument("bloom_filter: invalid parameters");
        }
        return bloom_filter(bit_count, hash_count, 0, tag{});
    }
    static std::optional<bloom_filter> try_from_params(std::size_t bit_count,
                                                       std::size_t hash_count) {
        if (bit_count == 0 || hash_count == 0) {
            return std::nullopt;
        }
        return bloom_filter(bit_count, hash_count, 0, tag{});
    }

    void add(const void *data, std::size_t len) {
        std::uint32_t h1, h2;
        hash_bases(data, len, h1, h2);
        for (std::size_t i = 0; i < hash_count_; i++) {
            std::size_t byte_idx, bit;
            index(h1, h2, i, byte_idx, bit);
            std::uint8_t mask = static_cast<std::uint8_t>(1u << bit);
            if ((bits_[byte_idx] & mask) == 0) {
                bits_[byte_idx] |= mask;
                bits_set_++;
            }
        }
        items_added_++;
    }
    void add(const std::string &s) { add(s.data(), s.size()); }

    bool contains(const void *data, std::size_t len) const {
        std::uint32_t h1, h2;
        hash_bases(data, len, h1, h2);
        for (std::size_t i = 0; i < hash_count_; i++) {
            std::size_t byte_idx, bit;
            index(h1, h2, i, byte_idx, bit);
            std::uint8_t mask = static_cast<std::uint8_t>(1u << bit);
            if ((bits_[byte_idx] & mask) == 0) {
                return false;
            }
        }
        return true;
    }
    bool contains(const std::string &s) const {
        return contains(s.data(), s.size());
    }

    std::size_t bit_count() const { return bit_count_; }
    std::size_t hash_count() const { return hash_count_; }
    std::size_t bits_set() const { return bits_set_; }
    std::size_t size_bytes() const { return bits_.size(); }

    double fill_ratio() const {
        if (bit_count_ == 0) {
            return 0.0;
        }
        return static_cast<double>(bits_set_) / static_cast<double>(bit_count_);
    }
    double estimated_false_positive_rate() const {
        if (bits_set_ == 0) {
            return 0.0;
        }
        double ratio = fill_ratio();
        double p = 1.0;
        for (std::size_t i = 0; i < hash_count_; i++) {
            p *= ratio; // ratio^k without libm's pow
        }
        return p;
    }
    bool is_over_capacity() const {
        if (expected_items_ == 0) {
            return false;
        }
        return items_added_ > expected_items_;
    }

    static std::size_t optimal_m(std::size_t n, double p) {
        double m = (-static_cast<double>(n) * detail::iso_ln(p)) /
                   (detail::LN2 * detail::LN2);
        return detail::d_ceil_size(m);
    }
    static std::size_t optimal_k(std::size_t m, std::size_t n) {
        if (n == 0) {
            return 1;
        }
        std::size_t k = detail::d_to_size(
            (static_cast<double>(m) / static_cast<double>(n)) * detail::LN2,
            0.5);
        return k < 1 ? 1 : k;
    }
    static std::size_t capacity_for_memory(std::size_t memory_bytes, double p) {
        double m = static_cast<double>(memory_bytes) * 8.0;
        double n = (-m * (detail::LN2 * detail::LN2)) / detail::iso_ln(p);
        return detail::d_to_size(n, 0.0);
    }

private:
    struct tag {};
    // Delegated constructor used by the static factories (already validated).
    bloom_filter(std::size_t bit_count, std::size_t hash_count,
                 std::size_t expected_items, tag) {
        init(bit_count, hash_count, expected_items);
    }

    static std::optional<bloom_error> validate_rate(std::size_t expected_items,
                                                    double p) {
        if (expected_items == 0) {
            return bloom_error::invalid_expected_items;
        }
        if (!(p > 0.0 && p < 1.0)) {
            return bloom_error::invalid_false_positive_rate;
        }
        return std::nullopt;
    }

    void init(std::size_t bit_count, std::size_t hash_count,
              std::size_t expected_items) {
        bit_count_ = bit_count;
        hash_count_ = hash_count;
        expected_items_ = expected_items;
        std::size_t byte_count = (bit_count == 0) ? 1 : (bit_count + 7) / 8;
        bits_.assign(byte_count, 0);
        bits_set_ = 0;
        items_added_ = 0;
    }

    void hash_bases(const void *data, std::size_t len, std::uint32_t &h1,
                    std::uint32_t &h2) const {
        const std::uint8_t *bytes = static_cast<const std::uint8_t *>(data);
        std::uint64_t h2raw = detail::djb2(bytes, len);
        std::uint32_t folded =
            static_cast<std::uint32_t>((h2raw ^ (h2raw >> 32)) & 0xffffffffu);
        h1 = detail::fmix32(detail::fnv1a_32(bytes, len));
        h2 = detail::fmix32(folded) | 1u;
    }
    void index(std::uint32_t h1, std::uint32_t h2, std::size_t i,
               std::size_t &byte_idx, std::size_t &bit) const {
        std::uint64_t idx = (static_cast<std::uint64_t>(h1) +
                             static_cast<std::uint64_t>(i) *
                                 static_cast<std::uint64_t>(h2)) %
                            static_cast<std::uint64_t>(bit_count_);
        byte_idx = static_cast<std::size_t>(idx / 8);
        bit = static_cast<std::size_t>(idx % 8);
    }

    std::size_t bit_count_ = 0;
    std::size_t hash_count_ = 0;
    std::size_t expected_items_ = 0;
    std::vector<std::uint8_t> bits_;
    std::size_t bits_set_ = 0;
    std::size_t items_added_ = 0;
};

} // namespace ca

#endif // BLOOM_FILTER_HPP
