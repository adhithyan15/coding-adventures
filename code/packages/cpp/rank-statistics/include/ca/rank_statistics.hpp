// Exact distribution-free rank statistics for finite binary64 samples.
#ifndef CA_RANK_STATISTICS_HPP
#define CA_RANK_STATISTICS_HPP

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <vector>

namespace ca { namespace rank_statistics {

struct Interval { double low; double high; };

namespace detail {
// A tiny unsigned integer is enough here: the recurrence only needs addition,
// multiplication/division by a small sample rank, shifts by one, and compare.
// Base 2^32 keeps each limb product in uint64_t and avoids a big-number package.
class BigNat {
    std::vector<std::uint32_t> limbs_; // least-significant limb first
public:
    explicit BigNat(std::uint32_t value = 0) : limbs_{value} {}
    void add(const BigNat& rhs) {
        const std::size_t count = std::max(limbs_.size(), rhs.limbs_.size());
        limbs_.resize(count, 0);
        std::uint64_t carry = 0;
        for (std::size_t i = 0; i < count; ++i) {
            const std::uint64_t sum = static_cast<std::uint64_t>(limbs_[i]) +
                (i < rhs.limbs_.size() ? rhs.limbs_[i] : 0) + carry;
            limbs_[i] = static_cast<std::uint32_t>(sum);
            carry = sum >> 32;
        }
        if (carry != 0) limbs_.push_back(static_cast<std::uint32_t>(carry));
    }
    void multiply(std::uint32_t factor) {
        std::uint64_t carry = 0;
        for (auto& limb : limbs_) {
            const std::uint64_t product = static_cast<std::uint64_t>(limb) * factor + carry;
            limb = static_cast<std::uint32_t>(product);
            carry = product >> 32;
        }
        if (carry != 0) limbs_.push_back(static_cast<std::uint32_t>(carry));
    }
    void divide_exact(std::uint32_t divisor) {
        std::uint64_t remainder = 0;
        for (std::size_t i = limbs_.size(); i != 0; --i) {
            const std::uint64_t numerator = (remainder << 32) | limbs_[i - 1];
            limbs_[i - 1] = static_cast<std::uint32_t>(numerator / divisor);
            remainder = numerator % divisor;
        }
        if (remainder != 0) throw std::logic_error("binomial recurrence was not exact");
        while (limbs_.size() > 1 && limbs_.back() == 0) limbs_.pop_back();
    }
    int compare(const BigNat& rhs) const noexcept {
        if (limbs_.size() != rhs.limbs_.size()) return limbs_.size() < rhs.limbs_.size() ? -1 : 1;
        for (std::size_t i = limbs_.size(); i != 0; --i) {
            if (limbs_[i - 1] != rhs.limbs_[i - 1]) return limbs_[i - 1] < rhs.limbs_[i - 1] ? -1 : 1;
        }
        return 0;
    }
};
} // namespace detail

// Return the sample median without modifying values. The same-sign difference
// form avoids overflow while preserving subnormal values; opposite-sign values
// can be added safely before halving.
inline double median(const std::vector<double>& values) {
    if (values.empty()) throw std::invalid_argument("values must not be empty");
    std::vector<double> sorted = values;
    for (double value : sorted) {
        if (!std::isfinite(value)) throw std::invalid_argument("values must be finite");
    }
    std::sort(sorted.begin(), sorted.end());
    const std::size_t middle = sorted.size() / 2;
    if (sorted.size() % 2 != 0) return sorted[middle];
    const double low = sorted[middle - 1], high = sorted[middle];
    // Add before halving so adjacent subnormals use ties-to-even correctly;
    // the difference form can underflow before the final rounding. If the sum
    // overflows for large same-sign values, halve each operand first.
    const double sum = low + high;
    return std::isfinite(sum) ? sum / 2.0 : low / 2.0 + high / 2.0;
}

// Exact interval cutoff. The 10,000-sample ceiling matches VER07's maximum
// observation count and bounds the O(n^2)-bit recurrence for hostile inputs.
inline std::optional<Interval> median_interval(const std::vector<double>& values,
                                                unsigned confidence_percent) {
    if (values.empty()) throw std::invalid_argument("values must not be empty");
    if (values.size() > 10'000) throw std::length_error("sample exceeds 10000 observations");
    if (confidence_percent < 1 || confidence_percent > 99)
        throw std::invalid_argument("confidence must be from 1 through 99");
    std::vector<double> sorted = values;
    for (double value : sorted) {
        if (!std::isfinite(value)) throw std::invalid_argument("values must be finite");
    }
    std::sort(sorted.begin(), sorted.end());

    detail::BigNat choose(1), cumulative, power_of_two(1);
    // Private limbs intentionally stay behind the API; exact comparisons never
    // narrow a binomial coefficient to a machine integer.
    const std::size_t n = sorted.size();
    for (std::size_t bit = 0; bit < n; ++bit) power_of_two.multiply(2);
    std::size_t best_rank = 0;
    for (std::size_t j = 0; j < n; ++j) {
        if (j != 0) {
            choose.multiply(static_cast<std::uint32_t>(n - j + 1));
            choose.divide_exact(static_cast<std::uint32_t>(j));
        }
        cumulative.add(choose);
        detail::BigNat left = cumulative;
        left.multiply(200);
        detail::BigNat right = power_of_two;
        right.multiply(100 - confidence_percent);
        if (left.compare(right) > 0) break;
        best_rank = j + 1;
    }
    if (best_rank == 0) return std::nullopt;
    return Interval{sorted[best_rank - 1], sorted[n - best_rank]};
}

}} // namespace ca::rank_statistics
#endif
