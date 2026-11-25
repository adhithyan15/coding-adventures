#include "ca/array_workload.hpp"
#include <stdexcept>

namespace ca { namespace microbenchmark {
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
        for (std::size_t i = 0; i < values.size(); ++i) sum += values[i];
    return sum;
}

Int64 iterator_batch(const std::array<Int64, array_length>& values, Int64 operations) {
    if (operations < 0) throw std::invalid_argument("operations must not be negative");
    Int64 sum = 0;
    for (Int64 op = 0; op < operations; ++op)
        for (const Int64 value : values) sum += value;
    return sum;
}
}}
