#include <ca/microbenchmark.hpp>

#include <cstdint>

int main() {
    ca::microbenchmark::Config config;
    config.pairs = 1;
    config.target_ns = 1;
    config.max_batch = 1;
    std::int64_t tick = 0;
    const ca::microbenchmark::Batch baseline = [](std::int64_t count) { return count; };
    const ca::microbenchmark::Batch candidate = [](std::int64_t count) { return count; };
    const ca::microbenchmark::Clock clock = [&]() { return tick++; };
    const auto report = ca::microbenchmark::collect(baseline, candidate, 1, config, clock);
    return report.pairs.size() == 1 && report.pairs.front().baseline_ns == 1 ? 0 : 1;
}
