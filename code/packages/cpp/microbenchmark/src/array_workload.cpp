#include "ca/array_workload.hpp"
#include <limits>
#include <stdexcept>

namespace ca { namespace microbenchmark {
static Int64 checked_add(Int64 left, Int64 right) {
    if ((right > 0 && left > std::numeric_limits<Int64>::max() - right) ||
        (right < 0 && left < std::numeric_limits<Int64>::min() - right))
        throw std::overflow_error("array batch checksum exceeds signed 64-bit range");
    return left + right;
}

std::array<Int64, array_length> make_indexed_array() {
    std::array<Int64, array_length> values{};
    Int64 sum = 0;
    for (std::size_t i = 0; i < values.size(); ++i) {
        values[i] = static_cast<Int64>((17 * i + 3) % 251);
        sum += values[i];
    }
    if (sum != array_checksum) throw std::logic_error("indexed-array checksum mismatch");
    return values;
}

Int64 indexed_batch(const std::array<Int64, array_length>& values, Int64 operations) {
    if (operations < 0) throw std::invalid_argument("operations must not be negative");
    Int64 sum = 0;
    for (Int64 op = 0; op < operations; ++op)
        for (std::size_t i = 0; i < values.size(); ++i) sum = checked_add(sum, values[i]);
    return sum;
}

Int64 iterator_batch(const std::array<Int64, array_length>& values, Int64 operations) {
    if (operations < 0) throw std::invalid_argument("operations must not be negative");
    Int64 sum = 0;
    for (Int64 op = 0; op < operations; ++op)
        for (const Int64 value : values) sum = checked_add(sum, value);
    return sum;
}
}}
