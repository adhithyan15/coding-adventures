#include "ca/rank_statistics.hpp"
#include "bounded_json.hpp"
#include "iso_test.h"

#include <cmath>
#include <fstream>
#include <iterator>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

namespace rs = ca::rank_statistics;
namespace bj = ca::bounded_json;

static const bj::Value& field(const bj::Value& object, std::string_view name) {
    for (std::size_t i = 0; i < object.size(); ++i) {
        const auto entry = object.member(i);
        if (entry.first == name) return *entry.second;
    }
    throw std::runtime_error("missing fixture field");
}

static int bounded_oracle_rank(unsigned n, unsigned confidence) {
    const unsigned assignments = 1u << n;
    int best = 0;
    for (unsigned rank = 1; rank <= n; ++rank) {
        unsigned below = 0;
        for (unsigned mask = 0; mask < assignments; ++mask) {
            unsigned count = 0;
            for (unsigned bit = 0; bit < n; ++bit) count += (mask >> bit) & 1u;
            if (count <= rank - 1) ++below;
        }
        if (200u * below <= (100u - confidence) * assignments) best = static_cast<int>(rank);
    }
    return best;
}

static void medians() {
    const std::vector<double> values{9, 1, 5, 3};
    ISO_CHECK(rs::median(values) == 4.0);
    ISO_CHECK(values == std::vector<double>({9, 1, 5, 3}));
    ISO_CHECK(rs::median({7}) == 7.0);
    ISO_CHECK(rs::median({std::numeric_limits<double>::max(), std::numeric_limits<double>::max()}) == std::numeric_limits<double>::max());
    ISO_CHECK(rs::median({-std::numeric_limits<double>::max(), std::numeric_limits<double>::max()}) == 0.0);
    ISO_CHECK(rs::median({std::numeric_limits<double>::denorm_min(), std::numeric_limits<double>::denorm_min()}) == std::numeric_limits<double>::denorm_min());
    const double denorm = std::numeric_limits<double>::denorm_min();
    ISO_CHECK(rs::median({denorm, 2 * denorm}) == 2 * denorm);
    ISO_CHECK(rs::median({3 * denorm, 4 * denorm}) == 4 * denorm);
    ISO_CHECK(rs::median({-3 * denorm, -2 * denorm}) == -2 * denorm);
    const double min_normal = std::numeric_limits<double>::min();
    ISO_CHECK(rs::median({std::nextafter(min_normal, 0.0), min_normal}) == min_normal);
    const double maximum = std::numeric_limits<double>::max();
    ISO_CHECK(rs::median({maximum / 2.0, maximum}) == maximum * 0.75);
    bool empty = false, nonfinite = false;
    try { (void)rs::median({}); } catch (const std::invalid_argument&) { empty = true; }
    try { (void)rs::median({1, std::numeric_limits<double>::infinity()}); } catch (const std::invalid_argument&) { nonfinite = true; }
    ISO_CHECK(empty && nonfinite);
}

static void exact_oracle_and_wide_cutoffs() {
    for (unsigned n = 1; n <= 10; ++n) {
        std::vector<double> sample;
        for (unsigned i = 0; i < n; ++i) sample.push_back(static_cast<double>(i % 3));
        for (unsigned confidence = 1; confidence <= 99; ++confidence) {
            const int rank = bounded_oracle_rank(n, confidence);
            const auto interval = rs::median_interval(sample, confidence);
            ISO_CHECK((rank == 0) == !interval.has_value());
            if (rank != 0) {
                auto sorted = sample;
                std::sort(sorted.begin(), sorted.end());
                ISO_CHECK(interval->low == sorted[static_cast<std::size_t>(rank - 1)]);
                ISO_CHECK(interval->high == sorted[n - static_cast<std::size_t>(rank)]);
            }
        }
    }
    // Frozen differential vectors from Python rank-statistics (not an oracle).
    struct Wide { unsigned n, confidence; std::size_t low, high; };
    const Wide vectors[] = {{64,95,23,40}, {65,99,21,43}, {127,95,51,75},
            {128,95,52,75}, {256,99,106,149}, {1000,95,468,531},
            {10000,50,4965,5034}, {10000,95,4901,5098}, {10000,99,4870,5129}};
    for (const Wide value : vectors) {
        std::vector<double> sample(value.n);
        for (std::size_t i = 0; i < sample.size(); ++i) sample[i] = static_cast<double>(i);
        const auto interval = rs::median_interval(sample, value.confidence);
        ISO_CHECK(interval.has_value());
        ISO_CHECK(interval->low == static_cast<double>(value.low));
        ISO_CHECK(interval->high == static_cast<double>(value.high));
    }
    bool bad_confidence = false, oversized = false;
    try { (void)rs::median_interval({1}, 100); } catch (const std::invalid_argument&) { bad_confidence = true; }
    try { (void)rs::median_interval(std::vector<double>(10001, 1), 95); } catch (const std::length_error&) { oversized = true; }
    ISO_CHECK(bad_confidence && oversized);
}

static void shared_fixture_and_drift() {
    std::ifstream local_file("testdata/microbenchmark-pairs-v1.json", std::ios::binary);
    const std::string local((std::istreambuf_iterator<char>(local_file)), {});
    ISO_CHECK(local_file.good() || local_file.eof());
    std::ifstream canonical_file("../../../../code/specs/fixtures/microbenchmark-pairs-v1.json", std::ios::binary);
    const std::string canonical((std::istreambuf_iterator<char>(canonical_file)), {});
    ISO_CHECK(canonical_file.good() || canonical_file.eof());
    ISO_CHECK(local == canonical);
    const auto parsed = bj::parse(local);
    ISO_CHECK(parsed);
    const auto& root = parsed.document->root();
    const auto& cases = field(root, "cases");
    ISO_CHECK(cases.size() == 6);
    const unsigned confidence = static_cast<unsigned>(field(root, "confidence_percent").integer());
    for (std::size_t i = 0; i < cases.size(); ++i) {
        const auto& example = cases.at(i);
        const double baseline = static_cast<double>(field(example, "baseline_ns").integer());
        std::vector<double> ratios;
        const auto& candidate = field(example, "candidate_ns");
        for (std::size_t j = 0; j < candidate.size(); ++j)
            ratios.push_back(static_cast<double>(candidate.at(j).integer()) / baseline);
        ISO_CHECK(std::abs(rs::median(ratios) - field(example, "ratio").number()) <= 1e-12);
        const auto expected = rs::median_interval(ratios, confidence);
        const auto& wanted = field(example, "interval");
        ISO_CHECK((wanted.kind() == bj::Kind::null) == !expected.has_value());
        if (expected) {
            ISO_CHECK(expected->low == wanted.at(0).number());
            ISO_CHECK(expected->high == wanted.at(1).number());
        }
    }
}

int main() {
    medians();
    exact_oracle_and_wide_cutoffs();
    shared_fixture_and_drift();
    return ISO_TEST_RESULT();
}
