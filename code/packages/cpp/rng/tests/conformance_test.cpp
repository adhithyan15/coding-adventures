// conformance_test.cpp — the shared rng corpus (spec rng.md, "Conformance
// corpus"), read from its flat form code/specs/fixtures/rng-v1/cases.txt.
//
// The corpus pins exact outputs every language's rng package must reproduce;
// its results come from the independent reference
// code/scripts/rng_conformance.py, which also generates the flat form so a
// lane with no JSON reader can use it. Each line is whitespace-separated:
//
//     case <id> <generator> <seed>
//     next_u32 <result>
//     next_u64 <result>
//     next_float <16 hex digits of the IEEE-754 bits>
//     next_int_in_range <min> <max> <result | error>
//     end
//
// The test runs from the package directory (tools/run.sh cds there).
#include "iso_test.h"

#include <cstdint>
#include <cstring>
#include <fstream>
#include <stdexcept>
#include <string>

#include "rng.hpp"

namespace {

// One generator of any of the three kinds, so the driver can be generic.
struct AnyRng {
    int kind = 0; // 0 lcg, 1 xorshift64, 2 pcg32
    ca::rng::Lcg lcg{0};
    ca::rng::Xorshift64 xs{0};
    ca::rng::Pcg32 pcg{0};

    std::uint32_t u32() {
        return kind == 0 ? lcg.next_u32() : kind == 1 ? xs.next_u32() : pcg.next_u32();
    }
    std::uint64_t u64() {
        return kind == 0 ? lcg.next_u64() : kind == 1 ? xs.next_u64() : pcg.next_u64();
    }
    double f64() {
        return kind == 0 ? lcg.next_float() : kind == 1 ? xs.next_float() : pcg.next_float();
    }
    std::int64_t in_range(std::int64_t min, std::int64_t max) {
        return kind == 0   ? lcg.next_int_in_range(min, max)
               : kind == 1 ? xs.next_int_in_range(min, max)
                           : pcg.next_int_in_range(min, max);
    }
};

bool start(AnyRng &g, const std::string &name, std::uint64_t seed) {
    if (name == "lcg") {
        g.kind = 0;
        g.lcg = ca::rng::Lcg(seed);
    } else if (name == "xorshift64") {
        g.kind = 1;
        g.xs = ca::rng::Xorshift64(seed);
    } else if (name == "pcg32") {
        g.kind = 2;
        g.pcg = ca::rng::Pcg32(seed);
    } else {
        return false;
    }
    return true;
}

} // namespace

int main() {
    std::ifstream in("../../../specs/fixtures/rng-v1/cases.txt");
    ISO_CHECK_MSG(in.good(), "open the rng-v1 flat corpus");
    std::string line, op, id, name, a, b, c;
    // Skip the '#' comment lines at the top.
    while (in.peek() == '#') {
        std::getline(in, line);
    }
    int cases = 0, calls = 0;
    AnyRng g;
    while (in >> op) {
        ISO_CHECK_STR_EQ(op.c_str(), "case");
        if (!(in >> id >> name >> a) || !start(g, name, std::stoull(a))) {
            ISO_CHECK_MSG(0, "malformed case line");
            break;
        }
        cases++;
        while (in >> op && op != "end") {
            calls++;
            if (op == "next_u32" && in >> a) {
                ISO_CHECK_MSG(g.u32() == std::stoull(a), id.c_str());
            } else if (op == "next_u64" && in >> a) {
                ISO_CHECK_MSG(g.u64() == std::stoull(a), id.c_str());
            } else if (op == "next_float" && in >> a) {
                double d = g.f64();
                std::uint64_t bits = 0;
                std::memcpy(&bits, &d, sizeof bits);
                ISO_CHECK_MSG(bits == std::stoull(a, nullptr, 16), id.c_str());
            } else if (op == "next_int_in_range" && in >> a >> b >> c) {
                const std::int64_t min = std::stoll(a), max = std::stoll(b);
                if (c == "error") {
                    // Thrown before any draw, so the sequence carries on.
                    bool threw = false;
                    try {
                        g.in_range(min, max);
                    } catch (const std::invalid_argument &) {
                        threw = true;
                    }
                    ISO_CHECK_MSG(threw, id.c_str());
                } else {
                    ISO_CHECK_MSG(g.in_range(min, max) == std::stoll(c), id.c_str());
                }
            } else {
                ISO_CHECK_MSG(0, "malformed call line");
                break;
            }
        }
    }
    // Guard against a truncated or empty file passing vacuously.
    ISO_CHECK_MSG(cases >= 27, "the corpus has at least 27 cases");
    ISO_CHECK_MSG(calls >= 150, "the corpus has at least 150 calls");
    return ISO_TEST_RESULT();
}
