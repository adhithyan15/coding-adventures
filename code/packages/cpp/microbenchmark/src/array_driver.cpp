#include "ca/array_workload.hpp"
#include "cli_output.hpp"

#include <cerrno>
#include <charconv>
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mb = ca::microbenchmark;

namespace {
struct Arguments { mb::Config config; std::string revision = "unknown"; bool help = false; };

mb::Int64 parse_integer(const std::string& text, const char* flag) {
    mb::Int64 result = 0;
    const auto parsed = std::from_chars(text.data(), text.data() + text.size(), result);
    if (parsed.ec != std::errc{} || parsed.ptr != text.data() + text.size())
        throw std::invalid_argument(std::string("invalid value for ") + flag);
    return result;
}

double parse_factor(const std::string& text) {
    errno = 0;
    char* end = nullptr;
    const double value = std::strtod(text.c_str(), &end);
    if (errno == ERANGE || end != text.c_str() + text.size() || !std::isfinite(value))
        throw std::invalid_argument("invalid value for --factor");
    return value;
}

Arguments parse_arguments(const std::vector<std::string>& args) {
    Arguments result;
    for (std::size_t i = 0; i < args.size(); ++i) {
        const std::string& flag = args[i];
        if (flag == "--help" || flag == "-h") { result.help = true; continue; }
        if (i + 1 == args.size()) throw std::invalid_argument(flag + " requires a value");
        const std::string& value = args[++i];
        if (flag == "--pairs") {
            const auto parsed = parse_integer(value, "--pairs");
            if (parsed < 1 || parsed > static_cast<mb::Int64>(mb::max_pairs)) throw std::invalid_argument("--pairs must be from 1 through 10000");
            result.config.pairs = static_cast<std::size_t>(parsed);
        } else if (flag == "--confidence") {
            const auto parsed = parse_integer(value, "--confidence");
            if (parsed < 1 || parsed > 99) throw std::invalid_argument("--confidence must be from 1 through 99");
            result.config.confidence_percent = static_cast<unsigned>(parsed);
        } else if (flag == "--factor") result.config.minimum_factor = parse_factor(value);
        else if (flag == "--seed") result.config.seed = parse_integer(value, "--seed");
        else if (flag == "--target-ns") result.config.target_ns = parse_integer(value, "--target-ns");
        else if (flag == "--max-batch") result.config.max_batch = parse_integer(value, "--max-batch");
        else if (flag == "--revision") result.revision = value;
        else throw std::invalid_argument("unknown option: " + flag);
    }
    mb::validate_config(result.config);
    return result;
}

std::string runtime_name() {
#if defined(__clang__)
    return std::string("Clang ") + __clang_version__;
#elif defined(_MSC_VER)
    return "MSVC " + std::to_string(_MSC_VER);
#elif defined(__GNUC__)
    return std::string("GCC ") + __VERSION__;
#else
    return "C++17 compiler";
#endif
}

std::string operating_system() {
#if defined(_WIN32)
    return "windows";
#elif defined(__APPLE__)
    return "darwin";
#elif defined(__linux__)
    return "linux";
#else
    return "unknown";
#endif
}

std::string architecture() {
#if defined(_M_X64) || defined(__x86_64__)
    return "x86_64";
#elif defined(_M_IX86) || defined(__i386__)
    return "x86";
#elif defined(_M_ARM64) || defined(__aarch64__)
    return "aarch64";
#elif defined(_M_ARM) || defined(__arm__)
    return "arm";
#else
    return "unknown";
#endif
}

std::string build_profile() {
#if defined(NDEBUG)
    return "release";
#else
    return "debug";
#endif
}

const char* usage() {
    return "Usage: array-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]\n";
}
} // namespace

int main(int argc, char** argv) {
    try {
        std::vector<std::string> args;
        std::vector<std::string> invocation;
        args.reserve(static_cast<std::size_t>(argc > 1 ? argc - 1 : 0));
        invocation.reserve(static_cast<std::size_t>(argc));
        if (argc > 0) invocation.emplace_back(argv[0]);
        for (int i = 1; i < argc; ++i) { args.emplace_back(argv[i]); invocation.emplace_back(argv[i]); }
        const Arguments parsed = parse_arguments(args);
        if (parsed.help) {
            ca::microbenchmark::detail::write_and_flush(std::cout, usage());
            return 0;
        }
        const auto values = mb::make_indexed_array();
        mb::Report report = mb::collect(
            [&](mb::Int64 count) { return mb::indexed_batch(values, count); },
            [&](mb::Int64 count) { return mb::iterator_batch(values, count); },
            mb::array_checksum, parsed.config);
        report.metadata.workload = "indexed-array-sum-4096";
        report.metadata.invocation = std::move(invocation);
        report.metadata.revision = parsed.revision;
        report.metadata.runtime = runtime_name();
        report.metadata.os = operating_system();
        report.metadata.arch = architecture();
        report.metadata.profile = build_profile();
        const std::string json = mb::to_json(report);
        ca::microbenchmark::detail::write_and_flush(std::cout, json);
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "array-microbenchmark: " << error.what() << '\n';
        return 1;
    }
}
