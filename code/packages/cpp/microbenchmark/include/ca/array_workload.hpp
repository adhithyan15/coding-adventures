#ifndef CA_ARRAY_WORKLOAD_HPP
#define CA_ARRAY_WORKLOAD_HPP

#include "ca/microbenchmark.hpp"
#include <array>

namespace ca { namespace microbenchmark {
inline constexpr std::size_t array_length = 4096;
inline constexpr Int64 array_checksum = 511533;
std::array<Int64, array_length> make_indexed_array();
Int64 indexed_batch(const std::array<Int64, array_length>& values, Int64 operations);
Int64 iterator_batch(const std::array<Int64, array_length>& values, Int64 operations);
}}
#endif
