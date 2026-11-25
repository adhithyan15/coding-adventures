// Reusable paired timing, analysis, and JSON report replay for VER07.
#ifndef CA_MICROBENCHMARK_HPP
#define CA_MICROBENCHMARK_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace ca { namespace microbenchmark {

inline constexpr std::size_t max_pairs = 10'000;
using Int64 = std::int64_t;
using Batch = std::function<Int64(Int64 operations)>;
using Clock = std::function<Int64()>;

struct Config {
    std::size_t pairs = 40;
    unsigned confidence_percent = 95;
    double minimum_factor = 1.05;
    Int64 seed = 0;
    Int64 target_ns = 100'000;
    Int64 max_batch = 65'536;
};
struct Pair { Int64 baseline_ns; Int64 candidate_ns; std::string order; };
struct Analysis {
    double ratio;
    std::optional<std::array<double, 2>> interval;
    std::string verdict;
};
struct Metadata {
    std::optional<std::string> workload, revision, runtime, os, arch, profile;
    std::optional<std::vector<std::string>> invocation;
};
struct Report {
    Config config;
    Int64 batch_size;
    std::vector<Pair> pairs;
    Analysis analysis;
    Metadata metadata;
};

void validate_config(const Config& config);
Analysis analyze(const std::vector<Pair>& pairs, unsigned confidence_percent = 95,
                 double minimum_factor = 1.05);
Report collect(const Batch& baseline, const Batch& candidate, Int64 expected_checksum,
               const Config& config = Config{}, const Clock& clock = Clock{});
std::string to_json(const Report& report);
Report replay_json(const std::string& json);
Int64 monotonic_nanoseconds();

}} // namespace ca::microbenchmark
#endif
