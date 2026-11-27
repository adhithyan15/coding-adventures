#include "ca/array_workload.hpp"
#include "ca/microbenchmark.hpp"
#include "ca/rank_statistics.hpp"
#include "bounded_json.hpp"
#include "../src/cli_output.hpp"
#include "iso_test.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <fstream>
#include <iterator>
#include <limits>
#include <stdexcept>
#include <sstream>
#include <string>
#include <vector>

namespace mb = ca::microbenchmark;
namespace bj = ca::bounded_json;

template <typename Function>
static bool throws_overflow(Function function) {
    try { function(); }
    catch (const std::overflow_error&) { return true; }
    return false;
}

static const bj::Value& field(const bj::Value& object, std::string_view name) {
    for (std::size_t i = 0; i < object.size(); ++i) {
        const auto item = object.member(i);
        if (item.first == name) return *item.second;
    }
    throw std::runtime_error("missing fixture field");
}

static mb::Pair pair(mb::Int64 baseline, mb::Int64 candidate, std::string order = "baseline-first") {
    return {baseline, candidate, std::move(order)};
}

static std::string replace_top_level_value(std::string json, std::string_view key,
                                           std::string_view replacement) {
    const std::string prefix = "\"" + std::string(key) + "\": ";
    const auto member = json.find(prefix);
    if (member == std::string::npos) throw std::runtime_error("missing report member");
    const auto begin = member + prefix.size();
    std::size_t end = begin;
    if (json[begin] == '"') {
        ++end;
        bool escaped = false;
        while (end < json.size()) {
            const char current = json[end++];
            if (escaped) escaped = false;
            else if (current == '\\') escaped = true;
            else if (current == '"') break;
        }
    } else if (json[begin] == '[') {
        int depth = 0;
        bool in_string = false, escaped = false;
        while (end < json.size()) {
            const char current = json[end++];
            if (in_string) {
                if (escaped) escaped = false;
                else if (current == '\\') escaped = true;
                else if (current == '"') in_string = false;
            } else if (current == '"') in_string = true;
            else if (current == '[') ++depth;
            else if (current == ']' && --depth == 0) break;
        }
    } else {
        while (end < json.size() && json[end] != ',' && json[end] != '\n' && json[end] != '}') ++end;
    }
    if (end > json.size() || end == begin) throw std::runtime_error("malformed report member");
    json.replace(begin, end - begin, replacement);
    return json;
}

static std::string remove_top_level_member(std::string json, std::string_view key) {
    const std::string prefix = "\"" + std::string(key) + "\": ";
    const auto member = json.find(prefix);
    if (member == std::string::npos) throw std::runtime_error("missing report member");
    const auto newline_before = json.rfind('\n', member);
    const auto begin = newline_before == std::string::npos ? 0 : newline_before + 1;
    const auto newline_after = json.find('\n', member);
    const auto end = newline_after == std::string::npos ? json.size() : newline_after + 1;
    json.erase(begin, end - begin);
    return json;
}

static void fixture_replay_and_package_drift() {
    std::ifstream local_file("testdata/microbenchmark-pairs-v1.json", std::ios::binary);
    const std::string local((std::istreambuf_iterator<char>(local_file)), {});
    ISO_CHECK(local_file.good() || local_file.eof());
    std::ifstream canonical_file("../../../../code/specs/fixtures/microbenchmark-pairs-v1.json", std::ios::binary);
    const std::string canonical((std::istreambuf_iterator<char>(canonical_file)), {});
    ISO_CHECK(canonical_file.good() || canonical_file.eof());
    ISO_CHECK(local == canonical);
    const auto parsed = bj::parse(local);
    ISO_CHECK(parsed);
    if (!parsed) return;
    const auto& root = parsed.document->root();
    const auto& cases = field(root, "cases");
    ISO_CHECK(cases.size() == 6);
    const unsigned confidence = static_cast<unsigned>(field(root, "confidence_percent").integer());
    const double factor = field(root, "minimum_factor").number();
    for (std::size_t i = 0; i < cases.size(); ++i) {
        const auto& example = cases.at(i);
        const auto baseline = field(example, "baseline_ns").integer();
        const auto& candidates = field(example, "candidate_ns");
        std::vector<mb::Pair> observations;
        for (std::size_t j = 0; j < candidates.size(); ++j)
            observations.push_back(pair(baseline, candidates.at(j).integer(), j % 2 ? "candidate-first" : "baseline-first"));
        const auto result = mb::analyze(observations, confidence, factor);
        ISO_CHECK(std::abs(result.ratio - field(example, "ratio").number()) <= 1e-12);
        ISO_CHECK(result.verdict == field(example, "verdict").string());
        const auto& wanted = field(example, "interval");
        ISO_CHECK((wanted.kind() == bj::Kind::null) == !result.interval.has_value());
        if (result.interval) {
            ISO_CHECK((*result.interval)[0] == wanted.at(0).number());
            ISO_CHECK((*result.interval)[1] == wanted.at(1).number());
        }
    }
}

static void collection_replays_balanced_and_seeded_pairs() {
    mb::Config config;
    config.pairs = 7;
    config.target_ns = 1;
    config.max_batch = 1;
    config.seed = -71;
    std::int64_t tick = 0;
    std::size_t baseline_calls = 0, candidate_calls = 0;
    const mb::Batch baseline = [&](mb::Int64 n) { ++baseline_calls; return 11 * n; };
    const mb::Batch candidate = [&](mb::Int64 n) { ++candidate_calls; return 11 * n; };
    const mb::Clock clock = [&]() { return tick++; };
    auto report = mb::collect(baseline, candidate, 11, config, clock);
    ISO_CHECK(report.batch_size == 1);
    ISO_CHECK(report.pairs.size() == 7);
    ISO_CHECK(baseline_calls == 9 && candidate_calls == 9);
    const auto count = std::count_if(report.pairs.begin(), report.pairs.end(), [](const auto& item) { return item.order == "baseline-first"; });
    ISO_CHECK(count == 4);
    const std::vector<std::string> expected_orders{
        "baseline-first", "candidate-first", "baseline-first", "candidate-first",
        "baseline-first", "candidate-first", "baseline-first",
    };
    for (std::size_t i = 0; i < expected_orders.size(); ++i)
        ISO_CHECK(report.pairs[i].order == expected_orders[i]);
    const auto encoded = mb::to_json(report);
    const auto replayed = mb::replay_json(encoded);
    ISO_CHECK(replayed.pairs.size() == report.pairs.size());
    for (std::size_t i = 0; i < report.pairs.size(); ++i) {
        ISO_CHECK(replayed.pairs[i].order == report.pairs[i].order);
        ISO_CHECK(replayed.pairs[i].baseline_ns == 1 && replayed.pairs[i].candidate_ns == 1);
    }
    report.metadata.workload = "indexed-array-sum-4096";
    report.metadata.invocation = std::vector<std::string>{"bench", "--revision", "a\"b"};
    report.metadata.revision = "a\"b";
    report.metadata.runtime = "test C++17";
    report.metadata.os = "test";
    report.metadata.arch = "x86_64";
    report.metadata.profile = "debug";
    const auto with_metadata = mb::to_json(report);
    ISO_CHECK(with_metadata.find("\\\"b") != std::string::npos);
    ISO_CHECK(mb::replay_json(with_metadata).metadata.profile == std::optional<std::string>("debug"));
    report.metadata.revision = std::string("bad\xff", 4);
    bool invalid_utf8 = false;
    try { (void)mb::to_json(report); } catch (const std::invalid_argument&) { invalid_utf8 = true; }
    ISO_CHECK(invalid_utf8);

    std::int64_t repeated_tick = 0;
    const mb::Clock repeat_clock = [&]() { return repeated_tick++; };
    const auto again = mb::collect(baseline, candidate, 11, config, repeat_clock);
    for (std::size_t i = 0; i < report.pairs.size(); ++i)
        ISO_CHECK(again.pairs[i].order == report.pairs[i].order);
}

static void failures_and_replay_validation() {
    mb::Config config;
    config.pairs = 1; config.target_ns = 1; config.max_batch = 1;
    const mb::Batch correct = [](mb::Int64 n) { return 2 * n; };
    std::int64_t t = 0;
    const mb::Clock clock = [&]() { return t++; };
    bool warmup = false, workload = false, overflow = false, calibration = false;
    try { (void)mb::collect([](mb::Int64) { return 0; }, correct, 2, config, clock); } catch (const std::runtime_error&) { warmup = true; }
    try { (void)mb::collect([](mb::Int64) -> mb::Int64 { throw std::runtime_error("batch failure"); }, correct, 2, config, clock); } catch (const std::runtime_error&) { workload = true; }
    mb::Config maxbatch = config; maxbatch.max_batch = 2;
    maxbatch.target_ns = 2;
    const mb::Int64 maximum = (std::numeric_limits<mb::Int64>::max)();
    const mb::Batch huge = [](mb::Int64 n) { return n == 1 ? (std::numeric_limits<mb::Int64>::max)() : 0; };
    try { (void)mb::collect(huge, huge, maximum, maxbatch, clock); } catch (const std::overflow_error&) { overflow = true; }
    std::int64_t stuck_tick = 0;
    try { (void)mb::collect([](mb::Int64 n) { return n; }, [](mb::Int64 n) { return n; }, 1, config, [&]() { return stuck_tick; }); } catch (const std::runtime_error&) { calibration = true; }
    ISO_CHECK(warmup && workload && overflow && calibration);

    auto good = mb::collect([](mb::Int64 n) { return n; }, [](mb::Int64 n) { return n; }, 1, config, clock);
    good.metadata.workload = "dependent-link-order";
    good.metadata.invocation = std::vector<std::string>{"cache-experiment", "--seed", "19"};
    good.metadata.revision = "experiment-revision";
    good.metadata.runtime = "go1.26.4";
    good.metadata.os = "windows";
    good.metadata.arch = "amd64";
    good.metadata.profile = "release";
    auto text = mb::to_json(good);
    const auto generic = mb::replay_json(text);
    ISO_CHECK(generic.metadata.workload == std::optional<std::string>("dependent-link-order"));
    ISO_CHECK(generic.metadata.revision == std::optional<std::string>("experiment-revision"));
    ISO_CHECK(generic.metadata.invocation == good.metadata.invocation);

    const auto replace_workload = [&](std::string replacement) {
        std::string changed = text;
        const std::string token = "\"dependent-link-order\"";
        const auto position = changed.find(token);
        if (position == std::string::npos) throw std::runtime_error("test report lacks workload metadata");
        changed.replace(position, token.size(), replacement);
        return changed;
    };
    for (const auto& invalid : {"\"\"", "\" \t\r\n\"", "\"bad\\u0000id\"", "null", "7", "true", "[]", "{}"}) {
        bool rejected = false;
        try { (void)mb::replay_json(replace_workload(invalid)); }
        catch (const std::invalid_argument&) { rejected = true; }
        ISO_CHECK(rejected);
    }
    std::string invalid_utf8 = replace_workload("\"bad\xFFid\"");
    bool rejected_utf8 = false;
    try { (void)mb::replay_json(invalid_utf8); }
    catch (const std::invalid_argument&) { rejected_utf8 = true; }
    ISO_CHECK(rejected_utf8);

    const auto reject_report = [](const std::string& candidate) {
        bool rejected = false;
        try { (void)mb::replay_json(candidate); }
        catch (const std::invalid_argument&) { rejected = true; }
        ISO_CHECK(rejected);
    };
    for (const auto* field_name : {"workload", "invocation", "revision", "runtime", "os", "arch"})
        reject_report(remove_top_level_member(text, field_name));

    auto core_report = good;
    core_report.metadata = {};
    auto core_json = mb::to_json(core_report);
    const auto final_object = core_json.rfind("\n}");
    ISO_CHECK(final_object != std::string::npos);
    core_json.insert(final_object, ",\n  \"profile\": \"release\"");
    reject_report(core_json);

    for (const auto* field_name : {"revision", "runtime", "os", "arch", "profile"}) {
        reject_report(replace_top_level_value(text, field_name, "\"\""));
        reject_report(replace_top_level_value(text, field_name, "\"bad\\u0000value\""));
        reject_report(replace_top_level_value(text, field_name, "null"));
    }
    for (const auto* invalid : {"\"\"", "\" \\t\\n\\r\\f\\u000b\"", "\"bad\\u0000value\""})
        reject_report(replace_top_level_value(text, "workload", invalid));
    for (const auto* invalid : {"null", "[1]", "[\"bad\\u0000arg\"]"})
        reject_report(replace_top_level_value(text, "invocation", invalid));
    ISO_CHECK(mb::replay_json(replace_top_level_value(text, "invocation", "[]"))
                  .metadata.invocation == std::optional<std::vector<std::string>>({}));
    ISO_CHECK(mb::replay_json(replace_top_level_value(text, "invocation", "[\"\"]"))
                  .metadata.invocation == std::optional<std::vector<std::string>>({""}));
    ISO_CHECK(mb::replay_json(replace_top_level_value(text, "revision", "\" \""))
                  .metadata.revision == std::optional<std::string>(" "));
    ISO_CHECK(mb::replay_json(replace_top_level_value(text, "workload", "\"\\u2003\""))
                  .metadata.workload == std::optional<std::string>("\xe2\x80\x83"));

    auto incomplete = good;
    incomplete.metadata.arch.reset();
    bool rejected_incomplete_encoder = false;
    try { (void)mb::to_json(incomplete); }
    catch (const std::invalid_argument&) { rejected_incomplete_encoder = true; }
    ISO_CHECK(rejected_incomplete_encoder);
    auto nul_argument = good;
    nul_argument.metadata.invocation =
        std::vector<std::string>{std::string("bad\0arg", 7)};
    bool rejected_nul_encoder = false;
    try { (void)mb::to_json(nul_argument); }
    catch (const std::invalid_argument&) { rejected_nul_encoder = true; }
    ISO_CHECK(rejected_nul_encoder);

    const auto order = text.find("baseline-first");
    ISO_CHECK(order != std::string::npos);
    const auto order_end = text.find('"', order + 1);
    text.replace(order, order_end - order, "candidate-first");
    bool unbalanced = false;
    try { (void)mb::replay_json(text); } catch (const std::invalid_argument&) { unbalanced = true; }
    ISO_CHECK(unbalanced);

    bool duplicate = false, unknown = false, out_of_range = false;
    try { (void)mb::replay_json("{\"x\":1,\"x\":2}"); } catch (const std::invalid_argument&) { duplicate = true; }
    try { (void)mb::replay_json("{\"unexpected\":true}"); } catch (const std::invalid_argument&) { unknown = true; }
    const std::string too_large = "{\"config\":{\"pairs\":1,\"confidence_percent\":95,\"minimum_factor\":1.05,\"seed\":0,\"target_ns\":1,\"max_batch\":9223372036854775808},\"batch_size\":1,\"pairs\":[],\"analysis\":{\"ratio\":1,\"interval\":null,\"verdict\":\"inconclusive\"}}";
    try { (void)mb::replay_json(too_large); } catch (const std::invalid_argument&) { out_of_range = true; }
    ISO_CHECK(duplicate && unknown && out_of_range);
}

static void signed_64_bit_round_trip_and_maximum_report() {
    const std::string json = "{\"config\":{\"pairs\":1,\"confidence_percent\":95,\"minimum_factor\":1.05,\"seed\":-9223372036854775808,\"target_ns\":1,\"max_batch\":9223372036854775807},\"batch_size\":1,\"pairs\":[{\"baseline_ns\":9223372036854775807,\"candidate_ns\":9223372036854775807,\"order\":\"baseline-first\"}],\"analysis\":{\"ratio\":1,\"interval\":null,\"verdict\":\"inconclusive\"}}";
    auto report = mb::replay_json(json);
    ISO_CHECK(report.config.seed == (std::numeric_limits<mb::Int64>::min)());
    ISO_CHECK(report.config.max_batch == (std::numeric_limits<mb::Int64>::max)());
    ISO_CHECK(report.pairs[0].baseline_ns == (std::numeric_limits<mb::Int64>::max)());
    const auto encoded = mb::to_json(report);
    ISO_CHECK(encoded.find("-9223372036854775808") != std::string::npos);
    ISO_CHECK(encoded.find("9223372036854775807") != std::string::npos);

    mb::Config config; config.pairs = 10'000; config.target_ns = 1; config.max_batch = 1;
    std::vector<mb::Pair> pairs; pairs.reserve(config.pairs);
    for (std::size_t i = 0; i < config.pairs; ++i)
        pairs.push_back(pair(100, 100, i < 5000 ? "baseline-first" : "candidate-first"));
    mb::Report large{config, 1, pairs, mb::analyze(pairs, config.confidence_percent, config.minimum_factor), {}};
    const auto large_json = mb::to_json(large);
    ISO_CHECK(large_json.size() < bj::max_bytes);
    const auto replayed = mb::replay_json(large_json);
    ISO_CHECK(replayed.pairs.size() == mb::max_pairs);
}

static void indexed_array_workload() {
    const auto values = mb::make_indexed_array();
    ISO_CHECK(values.size() == 4096);
    ISO_CHECK(values[0] == 3 && values[1] == 20 && values[2] == 37 && values[3] == 54);
    ISO_CHECK(mb::indexed_batch(values, 3) == mb::array_checksum * 3);
    ISO_CHECK(mb::iterator_batch(values, 3) == mb::array_checksum * 3);

    auto custom = values;
    custom.fill(0);
    custom[0] = std::numeric_limits<mb::Int64>::max();
    ISO_CHECK(mb::indexed_batch(custom, 1) == std::numeric_limits<mb::Int64>::max());
    ISO_CHECK(mb::iterator_batch(custom, 1) == std::numeric_limits<mb::Int64>::max());
    ISO_CHECK(throws_overflow([&] { (void)mb::indexed_batch(custom, 2); }));
    ISO_CHECK(throws_overflow([&] { (void)mb::iterator_batch(custom, 2); }));

    custom.fill(0);
    custom[0] = std::numeric_limits<mb::Int64>::min();
    ISO_CHECK(mb::indexed_batch(custom, 1) == std::numeric_limits<mb::Int64>::min());
    ISO_CHECK(mb::iterator_batch(custom, 1) == std::numeric_limits<mb::Int64>::min());
    ISO_CHECK(throws_overflow([&] { (void)mb::indexed_batch(custom, 2); }));
    ISO_CHECK(throws_overflow([&] { (void)mb::iterator_batch(custom, 2); }));

    custom.fill(0);
    custom[0] = std::numeric_limits<mb::Int64>::max();
    custom[1] = 1;
    ISO_CHECK(throws_overflow([&] { (void)mb::indexed_batch(custom, 1); }));
    ISO_CHECK(throws_overflow([&] { (void)mb::iterator_batch(custom, 1); }));

    custom.fill(0);
    custom[0] = std::numeric_limits<mb::Int64>::min();
    custom[1] = -1;
    ISO_CHECK(throws_overflow([&] { (void)mb::indexed_batch(custom, 1); }));
    ISO_CHECK(throws_overflow([&] { (void)mb::iterator_batch(custom, 1); }));

    custom.fill(0);
    custom[0] = std::numeric_limits<mb::Int64>::max();
    custom[1] = -std::numeric_limits<mb::Int64>::max();
    custom[2] = std::numeric_limits<mb::Int64>::min();
    custom[3] = std::numeric_limits<mb::Int64>::max();
    custom[4] = 1;
    ISO_CHECK(mb::indexed_batch(custom, 2) == 0);
    ISO_CHECK(mb::iterator_batch(custom, 2) == 0);
}

static void cli_output_failures_are_reported() {
    std::ostringstream output;
    output.setstate(std::ios::badbit);
    bool failure_reported = false;
    try { ca::microbenchmark::detail::write_and_flush(output, "{}\n"); }
    catch (const std::runtime_error&) { failure_reported = true; }
    ISO_CHECK(failure_reported);
}

static void generated_properties() {
    std::uint32_t state = UINT32_C(0x4d454449);
    for (unsigned sample = 0; sample < 2048; ++sample) {
        state ^= state << 13; state ^= state >> 17; state ^= state << 5;
        const std::size_t length = state % 36 + 1;
        std::vector<double> values;
        for (std::size_t i = 0; i < length; ++i) {
            state ^= state << 13; state ^= state >> 17; state ^= state << 5;
            values.push_back(static_cast<double>(static_cast<int>(state % 200001) - 100000));
        }
        auto reversed = values;
        std::reverse(reversed.begin(), reversed.end());
        ISO_CHECK(mb::analyze({pair(1, 1)}).ratio == 1.0);
        ISO_CHECK(ca::rank_statistics::median(values) == ca::rank_statistics::median(reversed));
        const auto interval = ca::rank_statistics::median_interval(values, static_cast<unsigned>(state % 99 + 1));
        if (interval) {
            ISO_CHECK(std::find(values.begin(), values.end(), interval->low) != values.end());
            ISO_CHECK(std::find(values.begin(), values.end(), interval->high) != values.end());
        }
    }
}

static void seeded_mutational_replay() {
    mb::Config config; config.pairs = 3; config.target_ns = 1; config.max_batch = 1;
    std::int64_t tick = 0;
    const mb::Batch identity = [](mb::Int64 n) { return n; };
    const auto original = mb::collect(identity, identity, 1, config, [&]() { return tick++; });
    const std::string seed = mb::to_json(original);
    std::uint32_t state = UINT32_C(0x4d555441);
    for (unsigned attempt = 0; attempt < 1024; ++attempt) {
        state ^= state << 13; state ^= state >> 17; state ^= state << 5;
        std::string changed = seed;
        const std::size_t position = state % changed.size();
        state ^= state << 13; state ^= state >> 17; state ^= state << 5;
        changed[position] = static_cast<char>(32 + state % 95);
        try {
            const auto accepted = mb::replay_json(changed);
            const auto round_trip = mb::replay_json(mb::to_json(accepted));
            ISO_CHECK(round_trip.pairs.size() == config.pairs);
            ISO_CHECK(round_trip.analysis.verdict == accepted.analysis.verdict);
        } catch (const std::exception&) {
            // Mutation may make invalid JSON or a report that violates policy.
        }
    }
}

int main() {
    fixture_replay_and_package_drift();
    collection_replays_balanced_and_seeded_pairs();
    failures_and_replay_validation();
    signed_64_bit_round_trip_and_maximum_report();
    indexed_array_workload();
    cli_output_failures_are_reported();
    generated_properties();
    seeded_mutational_replay();
    return ISO_TEST_RESULT();
}
