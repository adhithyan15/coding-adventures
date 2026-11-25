#include "ca/microbenchmark.hpp"
#include "ca/rank_statistics.hpp"
#include "bounded_json.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <iomanip>
#include <limits>
#include <locale>
#include <sstream>
#include <stdexcept>
#include <string_view>

namespace ca { namespace microbenchmark {
namespace {
constexpr Int64 min_i64 = (std::numeric_limits<Int64>::min)();
constexpr Int64 max_i64 = (std::numeric_limits<Int64>::max)();
namespace bj = ca::bounded_json;

void require_positive(Int64 value, const char* name) {
    if (value <= 0) throw std::invalid_argument(std::string(name) + " must be positive");
}

Int64 expected_batch(Int64 checksum, Int64 operations) {
    require_positive(operations, "batch size");
    if ((checksum > 0 && checksum > max_i64 / operations) ||
        (checksum < 0 && checksum < min_i64 / operations))
        throw std::overflow_error("batch checksum arithmetic exceeds signed 64-bit range");
    return checksum * operations;
}

class SplitMix64 {
    std::uint64_t state_;
public:
    explicit SplitMix64(Int64 seed) : state_(static_cast<std::uint64_t>(seed)) {}
    std::uint64_t next() noexcept {
        std::uint64_t z = (state_ += UINT64_C(0x9e3779b97f4a7c15));
        z = (z ^ (z >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)) * UINT64_C(0x94d049bb133111eb);
        return z ^ (z >> 31);
    }
    std::size_t bounded(std::size_t n) {
        const auto bound = static_cast<std::uint64_t>(n);
        const auto threshold = (std::uint64_t{0} - bound) % bound;
        for (;;) {
            const auto value = next();
            if (value >= threshold) return static_cast<std::size_t>(value % bound);
        }
    }
};

std::vector<std::string> schedule(std::size_t count, Int64 seed) {
    std::vector<std::string> result;
    result.reserve(count);
    for (std::size_t i = 0; i < (count + 1) / 2; ++i) result.emplace_back("baseline-first");
    for (std::size_t i = 0; i < count / 2; ++i) result.emplace_back("candidate-first");
    SplitMix64 random(seed);
    for (std::size_t i = result.size(); i > 1; --i) {
        const std::size_t j = random.bounded(i);
        std::swap(result[i - 1], result[j]);
    }
    return result;
}

std::string quote(std::string_view value) {
    static constexpr char hex[] = "0123456789abcdef";
    for (std::size_t i = 0; i < value.size();) {
        const unsigned char lead = static_cast<unsigned char>(value[i]);
        if (lead < 0x80) { ++i; continue; }
        std::size_t width = 0;
        unsigned char second_min = 0x80, second_max = 0xbf;
        if (lead >= 0xc2 && lead <= 0xdf) width = 2;
        else if (lead >= 0xe0 && lead <= 0xef) {
            width = 3;
            if (lead == 0xe0) second_min = 0xa0;
            if (lead == 0xed) second_max = 0x9f;
        } else if (lead >= 0xf0 && lead <= 0xf4) {
            width = 4;
            if (lead == 0xf0) second_min = 0x90;
            if (lead == 0xf4) second_max = 0x8f;
        } else throw std::invalid_argument("JSON strings must be valid UTF-8");
        if (i + width > value.size()) throw std::invalid_argument("JSON strings must be valid UTF-8");
        const unsigned char second = static_cast<unsigned char>(value[i + 1]);
        if (second < second_min || second > second_max) throw std::invalid_argument("JSON strings must be valid UTF-8");
        for (std::size_t offset = 2; offset < width; ++offset) {
            const unsigned char next = static_cast<unsigned char>(value[i + offset]);
            if (next < 0x80 || next > 0xbf) throw std::invalid_argument("JSON strings must be valid UTF-8");
        }
        i += width;
    }
    std::string result = "\"";
    for (unsigned char byte : value) {
        switch (byte) {
            case '"': result += "\\\""; break;
            case '\\': result += "\\\\"; break;
            case '\b': result += "\\b"; break;
            case '\f': result += "\\f"; break;
            case '\n': result += "\\n"; break;
            case '\r': result += "\\r"; break;
            case '\t': result += "\\t"; break;
            default:
                if (byte < 0x20) {
                    result += "\\u00";
                    result.push_back(hex[byte >> 4]);
                    result.push_back(hex[byte & 15]);
                } else result.push_back(static_cast<char>(byte));
        }
    }
    result.push_back('"');
    return result;
}

std::string number(double value) {
    if (!std::isfinite(value)) throw std::invalid_argument("JSON report floats must be finite");
    std::ostringstream output;
    output.imbue(std::locale::classic());
    output << std::setprecision(std::numeric_limits<double>::max_digits10) << value;
    return output.str();
}

const bj::Value* optional_member(const bj::Value& object, std::string_view name) {
    if (object.kind() != bj::Kind::object) throw std::invalid_argument("expected JSON object");
    const bj::Value* found = nullptr;
    for (std::size_t i = 0; i < object.size(); ++i) {
        const auto item = object.member(i);
        if (item.first == name) {
            if (found != nullptr) throw std::invalid_argument("duplicate JSON object member");
            found = item.second;
        }
    }
    return found;
}

const bj::Value& required_member(const bj::Value& object, std::string_view name) {
    const auto* value = optional_member(object, name);
    if (value == nullptr) throw std::invalid_argument("missing JSON report member: " + std::string(name));
    return *value;
}

void allowed_members(const bj::Value& object, const std::vector<std::string_view>& allowed) {
    if (object.kind() != bj::Kind::object) throw std::invalid_argument("expected JSON object");
    std::vector<std::string_view> seen;
    for (std::size_t i = 0; i < object.size(); ++i) {
        const auto item = object.member(i);
        if (std::find(allowed.begin(), allowed.end(), item.first) == allowed.end())
            throw std::invalid_argument("unknown JSON report member");
        if (std::find(seen.begin(), seen.end(), item.first) != seen.end())
            throw std::invalid_argument("duplicate JSON object member");
        seen.push_back(item.first);
    }
}

Int64 integer(const bj::Value& value, const char* name) {
    if (value.kind() != bj::Kind::integer) throw std::invalid_argument(std::string(name) + " must be a signed 64-bit JSON integer");
    return value.integer();
}

double finite_number(const bj::Value& value, const char* name) {
    if (value.kind() != bj::Kind::integer && value.kind() != bj::Kind::floating)
        throw std::invalid_argument(std::string(name) + " must be a number");
    const double result = value.number();
    if (!std::isfinite(result)) throw std::invalid_argument(std::string(name) + " must be finite");
    return result;
}

std::string string_value(const bj::Value& value, const char* name) {
    if (value.kind() != bj::Kind::string) throw std::invalid_argument(std::string(name) + " must be a string");
    return std::string(value.string());
}

void optional_string(const bj::Value& object, std::string_view key, std::optional<std::string>& target) {
    if (const auto* value = optional_member(object, key)) target = string_value(*value, "metadata");
}

void validate_workload_identifier(std::string_view value) {
    const auto ascii_space = [](unsigned char ch) {
        return ch == 0x20 || (ch >= 0x09 && ch <= 0x0d);
    };
    if (value.empty() || value.find('\0') != std::string_view::npos ||
        std::all_of(value.begin(), value.end(), [&](char ch) {
            return ascii_space(static_cast<unsigned char>(ch));
        }))
        throw std::invalid_argument("workload must be a nonblank UTF-8 identifier without NUL");
}

void append_optional(std::string& output, std::string_view key, const std::optional<std::string>& value) {
    if (value) output += ",\n  " + quote(key) + ": " + quote(*value);
}

} // namespace

void validate_config(const Config& config) {
    if (config.pairs == 0 || config.pairs > max_pairs) throw std::invalid_argument("pairs must be from 1 through 10000");
    if (config.confidence_percent < 1 || config.confidence_percent > 99) throw std::invalid_argument("confidence must be from 1 through 99");
    if (!std::isfinite(config.minimum_factor) || config.minimum_factor <= 1.0)
        throw std::invalid_argument("minimum_factor must be finite and greater than 1");
    require_positive(config.target_ns, "target_ns");
    require_positive(config.max_batch, "max_batch");
}

Analysis analyze(const std::vector<Pair>& pairs, unsigned confidence_percent, double minimum_factor) {
    if (pairs.empty()) throw std::invalid_argument("pairs must not be empty");
    if (pairs.size() > max_pairs) throw std::length_error("pair count exceeds 10000 observations");
    if (confidence_percent < 1 || confidence_percent > 99) throw std::invalid_argument("confidence must be from 1 through 99");
    if (!std::isfinite(minimum_factor) || minimum_factor <= 1.0)
        throw std::invalid_argument("minimum_factor must be finite and greater than 1");
    std::vector<double> ratios;
    ratios.reserve(pairs.size());
    for (const auto& pair : pairs) {
        require_positive(pair.baseline_ns, "baseline_ns");
        require_positive(pair.candidate_ns, "candidate_ns");
        if (pair.order != "baseline-first" && pair.order != "candidate-first")
            throw std::invalid_argument("pair order must be baseline-first or candidate-first");
        // Conversion precedes division to match the specified binary64 profile.
        ratios.push_back(static_cast<double>(pair.candidate_ns) / static_cast<double>(pair.baseline_ns));
    }
    Analysis result{rank_statistics::median(ratios), std::nullopt, "inconclusive"};
    const auto interval = rank_statistics::median_interval(ratios, confidence_percent);
    if (!interval) return result;
    result.interval = std::array<double, 2>{interval->low, interval->high};
    const double fast_boundary = 1.0 / minimum_factor;
    if (interval->high < fast_boundary) result.verdict = "faster";
    else if (interval->low > minimum_factor) result.verdict = "slower";
    else if (interval->low >= fast_boundary && interval->high <= minimum_factor) result.verdict = "equivalent";
    return result;
}

Int64 monotonic_nanoseconds() {
    using namespace std::chrono;
    static const auto start = steady_clock::now();
    const auto elapsed = duration<long double, std::nano>(steady_clock::now() - start).count();
    if (!(elapsed >= 0.0L) || elapsed > static_cast<long double>(max_i64)) return -1;
    return static_cast<Int64>(elapsed);
}

Report collect(const Batch& baseline, const Batch& candidate, Int64 expected_checksum,
               const Config& config, const Clock& supplied_clock) {
    validate_config(config);
    if (!baseline || !candidate) throw std::invalid_argument("baseline and candidate callbacks are required");
    const Clock clock = supplied_clock ? supplied_clock : Clock(monotonic_nanoseconds);
    auto verify = [&](const Batch& batch) {
        if (batch(1) != expected_checksum) throw std::runtime_error("checksum mismatch during warmup");
    };
    verify(baseline);
    verify(candidate);

    std::optional<Int64> previous;
    auto tick = [&]() {
        const Int64 now = clock();
        if (now < 0 || (previous && now < *previous)) throw std::runtime_error("clock is negative or regressed");
        previous = now;
        return now;
    };
    auto measure = [&](const Batch& batch, Int64 operations) {
        const Int64 expected = expected_batch(expected_checksum, operations);
        const Int64 start = tick();
        const Int64 observed = batch(operations);
        const Int64 end = tick();
        if (observed != expected) throw std::runtime_error("checksum mismatch in timed batch");
        return end - start;
    };
    Int64 batch_size = 1;
    for (;;) {
        const Int64 first = measure(baseline, batch_size);
        const Int64 second = measure(candidate, batch_size);
        if (first >= config.target_ns && second >= config.target_ns) break;
        if (batch_size >= config.max_batch) throw std::runtime_error("calibration limit exhausted");
        batch_size = batch_size > config.max_batch / 2 ? config.max_batch : batch_size * 2;
    }

    std::vector<Pair> pairs;
    pairs.reserve(config.pairs);
    for (const auto& order : schedule(config.pairs, config.seed)) {
        Int64 baseline_ns, candidate_ns;
        if (order == "baseline-first") {
            baseline_ns = measure(baseline, batch_size);
            candidate_ns = measure(candidate, batch_size);
        } else {
            candidate_ns = measure(candidate, batch_size);
            baseline_ns = measure(baseline, batch_size);
        }
        if (baseline_ns <= 0 || candidate_ns <= 0) throw std::runtime_error("measured durations must be positive");
        pairs.push_back({baseline_ns, candidate_ns, order});
    }
    return {config, batch_size, pairs, analyze(pairs, config.confidence_percent, config.minimum_factor), {}};
}

std::string to_json(const Report& report) {
    validate_config(report.config);
    std::string output = "{\n  \"config\": {\n    \"pairs\": " + std::to_string(report.config.pairs) +
        ",\n    \"confidence_percent\": " + std::to_string(report.config.confidence_percent) +
        ",\n    \"minimum_factor\": " + number(report.config.minimum_factor) +
        ",\n    \"seed\": " + std::to_string(report.config.seed) +
        ",\n    \"target_ns\": " + std::to_string(report.config.target_ns) +
        ",\n    \"max_batch\": " + std::to_string(report.config.max_batch) + "\n  },\n  \"batch_size\": " +
        std::to_string(report.batch_size) + ",\n  \"pairs\": [";
    for (std::size_t i = 0; i < report.pairs.size(); ++i) {
        const auto& pair = report.pairs[i];
        output += (i == 0 ? "\n" : ",\n") + std::string("    {\"baseline_ns\": ") +
            std::to_string(pair.baseline_ns) + ", \"candidate_ns\": " + std::to_string(pair.candidate_ns) +
            ", \"order\": " + quote(pair.order) + "}";
    }
    if (!report.pairs.empty()) output += "\n  ";
    output += "],\n  \"analysis\": {\n    \"ratio\": " + number(report.analysis.ratio) + ",\n    \"interval\": ";
    if (report.analysis.interval) output += "[" + number((*report.analysis.interval)[0]) + ", " + number((*report.analysis.interval)[1]) + "]";
    else output += "null";
    output += ",\n    \"verdict\": " + quote(report.analysis.verdict) + "\n  }";
    append_optional(output, "workload", report.metadata.workload);
    if (report.metadata.invocation) {
        output += ",\n  \"invocation\": [";
        for (std::size_t i = 0; i < report.metadata.invocation->size(); ++i)
            output += (i == 0 ? "" : ", ") + quote((*report.metadata.invocation)[i]);
        output += "]";
    }
    append_optional(output, "revision", report.metadata.revision);
    append_optional(output, "runtime", report.metadata.runtime);
    append_optional(output, "os", report.metadata.os);
    append_optional(output, "arch", report.metadata.arch);
    append_optional(output, "profile", report.metadata.profile);
    output += "\n}\n";
    return output;
}

Report replay_json(const std::string& json) {
    const auto parsed = bj::parse(json);
    if (!parsed) throw std::invalid_argument("invalid or over-limit JSON report");
    const auto& root = parsed.document->root();
    allowed_members(root, {"config", "batch_size", "pairs", "analysis", "workload", "invocation", "revision", "runtime", "os", "arch", "profile"});

    Report report{};
    const auto& config = required_member(root, "config");
    allowed_members(config, {"pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch"});
    const Int64 count = integer(required_member(config, "pairs"), "pairs");
    const Int64 confidence = integer(required_member(config, "confidence_percent"), "confidence_percent");
    if (count < 1 || count > static_cast<Int64>(max_pairs) || confidence < 1 || confidence > 99)
        throw std::invalid_argument("invalid report configuration range");
    report.config.pairs = static_cast<std::size_t>(count);
    report.config.confidence_percent = static_cast<unsigned>(confidence);
    report.config.minimum_factor = finite_number(required_member(config, "minimum_factor"), "minimum_factor");
    report.config.seed = integer(required_member(config, "seed"), "seed");
    report.config.target_ns = integer(required_member(config, "target_ns"), "target_ns");
    report.config.max_batch = integer(required_member(config, "max_batch"), "max_batch");
    validate_config(report.config);
    report.batch_size = integer(required_member(root, "batch_size"), "batch_size");
    require_positive(report.batch_size, "batch_size");
    if (report.batch_size > report.config.max_batch) throw std::invalid_argument("batch_size exceeds max_batch");

    const auto& raw_pairs = required_member(root, "pairs");
    if (raw_pairs.kind() != bj::Kind::array || raw_pairs.size() != report.config.pairs)
        throw std::invalid_argument("report pair count does not match config");
    report.pairs.reserve(raw_pairs.size());
    for (std::size_t i = 0; i < raw_pairs.size(); ++i) {
        const auto& raw_pair = raw_pairs.at(i);
        allowed_members(raw_pair, {"baseline_ns", "candidate_ns", "order"});
        report.pairs.push_back({integer(required_member(raw_pair, "baseline_ns"), "baseline_ns"),
            integer(required_member(raw_pair, "candidate_ns"), "candidate_ns"),
            string_value(required_member(raw_pair, "order"), "order")});
    }
    const auto& raw_analysis = required_member(root, "analysis");
    allowed_members(raw_analysis, {"ratio", "interval", "verdict"});
    const double claimed_ratio = finite_number(required_member(raw_analysis, "ratio"), "ratio");
    const auto& claimed_interval = required_member(raw_analysis, "interval");
    std::optional<std::array<double, 2>> interval;
    if (claimed_interval.kind() != bj::Kind::null) {
        if (claimed_interval.kind() != bj::Kind::array || claimed_interval.size() != 2)
            throw std::invalid_argument("analysis interval must be null or a pair");
        interval = std::array<double, 2>{finite_number(claimed_interval.at(0), "interval low"),
            finite_number(claimed_interval.at(1), "interval high")};
    }
    const std::string verdict = string_value(required_member(raw_analysis, "verdict"), "verdict");
    const auto& raw_observations = required_member(root, "pairs");
    std::size_t baseline_first = 0;
    for (std::size_t i = 0; i < raw_observations.size(); ++i)
        if (report.pairs[i].order == "baseline-first") ++baseline_first;
    if (baseline_first != (report.config.pairs + 1) / 2) throw std::invalid_argument("report pair order counts are not balanced");
    const Analysis expected = analyze(report.pairs, report.config.confidence_percent, report.config.minimum_factor);
    if (claimed_ratio != expected.ratio || interval != expected.interval || verdict != expected.verdict)
        throw std::invalid_argument("reported analysis does not match raw observations");
    report.analysis = expected;

    if (const auto* workload = optional_member(root, "workload")) {
        report.metadata.workload = string_value(*workload, "workload");
        validate_workload_identifier(*report.metadata.workload);
    }
    if (const auto* invocation = optional_member(root, "invocation")) {
        if (invocation->kind() != bj::Kind::array) throw std::invalid_argument("invocation must be an array of strings");
        report.metadata.invocation.emplace();
        for (std::size_t i = 0; i < invocation->size(); ++i)
            report.metadata.invocation->push_back(string_value(invocation->at(i), "invocation item"));
    }
    optional_string(root, "revision", report.metadata.revision);
    optional_string(root, "runtime", report.metadata.runtime);
    optional_string(root, "os", report.metadata.os);
    optional_string(root, "arch", report.metadata.arch);
    optional_string(root, "profile", report.metadata.profile);
    return report;
}

}} // namespace ca::microbenchmark
