// conformance_test.cpp — bit-exact conformance of the C++ `trig` port against
// the shared TRIG-REF-1 vectors (PHY00a section 6).
// ===========================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as one line
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 BIT PATTERNS (16 lowercase hex digits), never
// decimals. This test follows PHY00a 6.5 to the letter:
//
//   (a) decode x from hex by a bit cast (memcpy), never by parsing a decimal;
//   (b) call the port's PUBLIC function, not an internal one;
//   (c) encode the result back to bits and compare the two INTEGERS. Printed
//       floats hide the last digits, so they are never compared. The one
//       relaxation is NaN: y = 7ff8000000000000 matches ANY NaN (VER04p `same`),
//       and y = `error` means "sqrt of a negative", which this port signals by
//       throwing std::domain_error;
//   (d) the BEAM-only flags (nonfinite=..., beam-raises) do not apply: C++
//       represents every double and never raises on overflow;
//   (e) the constants the header embeds are asserted bit for bit.
//
// A second pass checks ACCURACY against the `cr` column (the correctly rounded
// value of the mathematical function), measured in representable steps
// (VER04p 5.4 `ulp_dist`): sin and cos within 3, radians and degrees within 1.
// `tan` carries a `cr` on most rows but PHY00a 3.8 makes NO accuracy claim for
// it, so it is named explicitly and skipped; any OTHER op with a `cr` value and
// no limit fails the test, so a new op cannot slip through unmeasured.
//
// The test runs from the package directory (tools/run.sh and run.ps1 cd there),
// and falls back to walking up from the working directory to find the corpus.
#include "iso_test.h"

#include <cstdint>
#include <cstring>
#include <fstream>
#include <stdexcept>
#include <string>
#include <vector>

#include "trig.hpp"

namespace t = ca::trig;

namespace {

typedef std::uint64_t u64;

const u64 QNAN_BITS = 0x7FF8000000000000ULL;

double from_bits(u64 b) {
    double d;
    std::memcpy(&d, &b, sizeof d);
    return d;
}

u64 to_bits(double d) {
    u64 b;
    std::memcpy(&b, &d, sizeof b);
    return b;
}

// A NaN has an all-ones exponent and a non-zero fraction. Decided on the bits
// so the test never depends on a comparison the compiler might fold.
bool bits_are_nan(u64 b) {
    return (b & 0x7FF0000000000000ULL) == 0x7FF0000000000000ULL &&
           (b & 0x000FFFFFFFFFFFFFULL) != 0;
}

// Sixteen lowercase hex digits to an integer. Anything else is a corpus error.
bool parse_hex16(const std::string &s, u64 &out) {
    if (s.size() != 16) return false;
    u64 v = 0;
    for (std::size_t i = 0; i < 16; i++) {
        char c = s[i];
        unsigned d;
        if (c >= '0' && c <= '9') {
            d = static_cast<unsigned>(c - '0');
        } else if (c >= 'a' && c <= 'f') {
            d = static_cast<unsigned>(c - 'a') + 10U;
        } else {
            return false;
        }
        v = (v << 4) | d;
    }
    out = v;
    return true;
}

// VER04p 5.4: map the bit pattern onto a line where adjacent doubles are
// adjacent integers (sign-magnitude to two's complement), so +0 and -0 are both
// 0 and the distance counts representable steps.
long long ordered(u64 b) {
    long long mag = static_cast<long long>(b & 0x7FFFFFFFFFFFFFFFULL);
    return (b >> 63) ? -mag : mag;
}

long long ulp_steps(u64 a, u64 b) {
    long long d = ordered(a) - ordered(b);
    return d < 0 ? -d : d;
}

struct Row {
    std::string op, x, y, cr, flags;
};

// Locate the corpus directory.
std::string fixture_dir() {
    const std::string tail = "specs/fixtures/trig-reference-v1/";
    std::string prefix;
    for (int depth = 0; depth < 8; depth++) {
        std::string candidate = prefix + "code/" + tail;
        std::ifstream probe((candidate + "manifest.json").c_str());
        if (probe) return candidate;
        // Run from code/packages/cpp/trig: ../../../specs/...
        candidate = prefix + "../../../" + tail;
        std::ifstream probe2((candidate + "manifest.json").c_str());
        if (probe2) return candidate;
        prefix += "../";
    }
    return std::string();
}

bool load_group(const std::string &dir, const std::string &group,
                std::vector<Row> &rows) {
    std::ifstream in((dir + group + ".tsv").c_str(), std::ios::binary);
    if (!in) return false;
    std::string line;
    while (std::getline(in, line)) {
        std::vector<std::string> f;
        std::string cur;
        for (std::size_t i = 0; i < line.size(); i++) {
            if (line[i] == '\t') {
                f.push_back(cur);
                cur.clear();
            } else {
                cur += line[i];
            }
        }
        f.push_back(cur);
        if (f.size() != 5) return false;
        Row r;
        r.op = f[0];
        r.x = f[1];
        r.y = f[2];
        r.cr = f[3];
        r.flags = f[4];
        rows.push_back(r);
    }
    return true;
}

// What the port returned: either bits, or "threw domain_error".
struct Result {
    u64 bits;
    bool threw;
};

bool call(const Row &r, Result &out) {
    out.threw = false;
    out.bits = 0;
    if (r.op == "atan2") {
        std::size_t comma = r.x.find(',');
        u64 yb, xb;
        if (comma == std::string::npos ||
            !parse_hex16(r.x.substr(0, comma), yb) ||
            !parse_hex16(r.x.substr(comma + 1), xb))
            return false;
        out.bits = to_bits(t::atan2(from_bits(yb), from_bits(xb)));
        return true;
    }
    u64 xb;
    if (!parse_hex16(r.x, xb)) return false;
    double v = from_bits(xb);
    if (r.op == "sin") out.bits = to_bits(t::sin(v));
    else if (r.op == "cos") out.bits = to_bits(t::cos(v));
    else if (r.op == "tan") out.bits = to_bits(t::tan(v));
    else if (r.op == "radians") out.bits = to_bits(t::radians(v));
    else if (r.op == "degrees") out.bits = to_bits(t::degrees(v));
    else if (r.op == "atan") out.bits = to_bits(t::atan(v));
    else if (r.op == "sqrt") {
        try {
            out.bits = to_bits(t::sqrt(v));
        } catch (const std::domain_error &) {
            out.threw = true;
        }
    } else {
        return false;
    }
    return true;
}

// Per-op tallies, so a failing run prints "sin: 1234 of 11712 rows differ".
struct Tally {
    std::string op;
    long total, failed;
};

Tally &tally_for(std::vector<Tally> &v, const std::string &op) {
    for (std::size_t i = 0; i < v.size(); i++)
        if (v[i].op == op) return v[i];
    Tally t0;
    t0.op = op;
    t0.total = 0;
    t0.failed = 0;
    v.push_back(t0);
    return v.back();
}

}  // namespace

int main() {
    const std::string dir = fixture_dir();
    ISO_CHECK_MSG(!dir.empty(), "trig-reference-v1 corpus not found");
    if (dir.empty()) return ISO_TEST_RESULT();

    const char *groups[] = {"edge", "sweep-binade", "sweep-cw", "sweep-prng"};
    std::vector<Row> rows;
    for (int g = 0; g < 4; g++) {
        ISO_CHECK_MSG(load_group(dir, groups[g], rows), "cannot read a group");
    }
    ISO_CHECK(rows.size() == 62154U);  // 2854 + 19520 + 19300 + 20480

    // ── Pass 1: every row, bit for bit ───────────────────────────────────
    std::vector<Tally> tallies;
    int shown = 0;
    for (std::size_t i = 0; i < rows.size(); i++) {
        const Row &r = rows[i];
        Tally &ty = tally_for(tallies, r.op);
        ty.total++;
        Result got;
        bool decoded = call(r, got);
        ISO_CHECK_MSG(decoded, "row did not decode");
        bool ok;
        if (r.y == "error") {
            ok = got.threw;
        } else {
            u64 want = 0;
            ISO_CHECK_MSG(parse_hex16(r.y, want), "bad y field");
            if (got.threw) {
                ok = false;
            } else if (want == QNAN_BITS) {
                ok = bits_are_nan(got.bits);
            } else {
                ok = got.bits == want;
            }
        }
        iso_test_checks_run++;
        if (!ok) {
            ty.failed++;
            iso_test_checks_failed++;
            if (shown < 12) {
                shown++;
                printf("  FAIL %s %s: want %s, got %016llx%s\n", r.op.c_str(),
                       r.x.c_str(), r.y.c_str(),
                       static_cast<unsigned long long>(got.bits),
                       got.threw ? " (threw)" : "");
            }
        }
    }
    for (std::size_t i = 0; i < tallies.size(); i++) {
        printf("  %-8s %6ld rows, %6ld differ from the reference\n",
               tallies[i].op.c_str(), tallies[i].total, tallies[i].failed);
    }

    // ── Pass 2: accuracy against the correctly rounded column ────────────
    // PHY00a 4.1 / 6.4: sin, cos <= 3 steps; radians, degrees <= 1 step.
    // tan: no claim (3.8), skipped BY NAME. Any other op with a `cr` and no
    // limit fails closed.
    long worst_sin = 0, worst_cos = 0, worst_rad = 0, worst_deg = 0;
    for (std::size_t i = 0; i < rows.size(); i++) {
        const Row &r = rows[i];
        if (r.cr == "-" || r.op == "tan") continue;
        long *worst = 0;
        long limit = 0;
        if (r.op == "sin") { worst = &worst_sin; limit = 3; }
        else if (r.op == "cos") { worst = &worst_cos; limit = 3; }
        else if (r.op == "radians") { worst = &worst_rad; limit = 1; }
        else if (r.op == "degrees") { worst = &worst_deg; limit = 1; }
        ISO_CHECK_MSG(worst != 0, "an op has a cr value but no accuracy limit");
        if (worst == 0) continue;
        u64 cr = 0;
        ISO_CHECK_MSG(parse_hex16(r.cr, cr), "bad cr field");
        Result got;
        if (!call(r, got)) continue;
        long steps = static_cast<long>(ulp_steps(got.bits, cr));
        if (steps > *worst) *worst = steps;
        iso_test_checks_run++;
        if (steps > limit) {
            iso_test_checks_failed++;
            printf("  FAIL accuracy %s %s: %ld steps from cr (limit %ld)\n",
                   r.op.c_str(), r.x.c_str(), steps, limit);
        }
    }
    printf("  worst steps from cr: sin %ld cos %ld radians %ld degrees %ld\n",
           worst_sin, worst_cos, worst_rad, worst_deg);

    // ── Contract, asserted directly (PHY00a 3.3, 3.8, 3.9) ───────────────
    ISO_CHECK(to_bits(t::sin(-0.0)) == 0x8000000000000000ULL);
    ISO_CHECK(to_bits(t::sin(0.0)) == 0);
    ISO_CHECK(t::cos(0.0) == 1.0 && t::cos(-0.0) == 1.0);
    {
        double tiny = from_bits(0x3E3FFFFFFFFFFFFFULL);  // just below 2^-27
        ISO_CHECK(t::sin(tiny) == tiny);
        ISO_CHECK(t::cos(tiny) == 1.0);
        double sub = from_bits(1);  // the smallest subnormal
        ISO_CHECK(t::sin(sub) == sub);
    }
    {
        // NaN, both infinities and everything beyond 2^29 are outside the
        // domain: sin, cos and tan return NaN.
        const u64 outside[] = {0x7FF8000000000000ULL, 0x7FF0000000000000ULL,
                               0xFFF0000000000000ULL,
                               0x44415AF1D78B58C4ULL,  // 1e20
                               0x41C0000000000001ULL,  // 2^29 + 1 ulp
                               0x7FEFFFFFFFFFFFFFULL};
        for (std::size_t i = 0; i < sizeof outside / sizeof outside[0]; i++) {
            double x = from_bits(outside[i]);
            ISO_CHECK(bits_are_nan(to_bits(t::sin(x))));
            ISO_CHECK(bits_are_nan(to_bits(t::cos(x))));
            ISO_CHECK(bits_are_nan(to_bits(t::tan(x))));
        }
    }
    // The last in-domain value is finite and exact to the pinned bits.
    ISO_CHECK(to_bits(t::sin(536870912.0)) == 0x3FD4E67C0E2622DFULL);
    ISO_CHECK(to_bits(t::cos(536870912.0)) == 0xBFEE3EDD2DFEEF90ULL);
    // atan2 of a NaN is NaN, not 0.0 (3.9).
    {
        double nan = from_bits(QNAN_BITS);
        ISO_CHECK(bits_are_nan(to_bits(t::atan2(nan, 1.0))));
        ISO_CHECK(bits_are_nan(to_bits(t::atan2(1.0, nan))));
        ISO_CHECK(bits_are_nan(to_bits(t::atan2(-1.0, nan))));
        ISO_CHECK(bits_are_nan(to_bits(t::atan2(nan, nan))));
    }
    // The pinned, non-IEEE conventions.
    ISO_CHECK(t::atan2(-0.0, -1.0) == t::PI);
    ISO_CHECK(to_bits(t::atan2(-0.0, -0.0)) == 0);
    ISO_CHECK(t::tan(t::HALF_PI) == 1e308);
    ISO_CHECK(t::tan(-t::HALF_PI) == -1e308);
    // radians/degrees are one multiply each.
    ISO_CHECK(t::radians(180.0) == t::PI);
    ISO_CHECK(t::degrees(t::PI) == 180.0);
    ISO_CHECK(to_bits(t::radians(-0.0)) == 0x8000000000000000ULL);
    ISO_CHECK(to_bits(t::degrees(-0.0)) == 0x8000000000000000ULL);
    ISO_CHECK(to_bits(t::degrees(1.7e308)) == 0x7FF0000000000000ULL);

    // ── Constants, bit for bit (PHY00a 3.2, 6.5 e) ───────────────────────
    ISO_CHECK(to_bits(t::PI) == 0x400921FB54442D18ULL);
    ISO_CHECK(to_bits(t::TWO_PI) == 0x401921FB54442D18ULL);
    ISO_CHECK(to_bits(t::HALF_PI) == 0x3FF921FB54442D18ULL);
    ISO_CHECK(to_bits(t::detail::INV_PIO2) == 0x3FE45F306DC9C883ULL);
    ISO_CHECK(to_bits(t::detail::PIO2_1) == 0x3FF921FB40000000ULL);
    ISO_CHECK(to_bits(t::detail::PIO2_2) == 0x3E74442D00000000ULL);
    ISO_CHECK(to_bits(t::detail::PIO2_3) == 0x3CF8469880000000ULL);
    ISO_CHECK(to_bits(t::detail::PIO2_4) == 0x3B78CC5160000000ULL);
    ISO_CHECK(to_bits(t::detail::PIO2_5) == 0x39F01B839A25204AULL);
    ISO_CHECK(to_bits(t::detail::MAGIC) == 0x4338000000000000ULL);
    ISO_CHECK(to_bits(t::detail::LIMIT) == 0x41C0000000000000ULL);
    ISO_CHECK(to_bits(t::detail::TINY) == 0x3E40000000000000ULL);
    ISO_CHECK(to_bits(t::detail::RAD_PER_DEG) == 0x3F91DF46A2529D39ULL);
    ISO_CHECK(to_bits(t::detail::DEG_PER_RAD) == 0x404CA5DC1A63C1F8ULL);
    {
        const u64 s[9] = {0xBFC5555555555555ULL, 0x3F81111111111111ULL,
                          0xBF2A01A01A01A01AULL, 0x3EC71DE3A556C734ULL,
                          0xBE5AE64567F544E4ULL, 0x3DE6124613A86D09ULL,
                          0xBD6AE7F3E733B81FULL, 0x3CE952C77030AD4AULL,
                          0xBC62F49B46814157ULL};
        const u64 c[8] = {0x3FA5555555555555ULL, 0xBF56C16C16C16C17ULL,
                          0x3EFA01A01A01A01AULL, 0xBE927E4FB7789F5CULL,
                          0x3E21EED8EFF8D898ULL, 0xBDA93974A8C07C9DULL,
                          0x3D2AE7F3E733B81FULL, 0xBCA6827863B97D97ULL};
        for (int n = 0; n < 9; n++) ISO_CHECK(to_bits(t::detail::S[n]) == s[n]);
        for (int n = 0; n < 8; n++) ISO_CHECK(to_bits(t::detail::C[n]) == c[n]);
    }
    // The decimal literals Part II and tan write in code.
    ISO_CHECK(to_bits(1e-15) == 0x3CD203AF9EE75616ULL);
    ISO_CHECK(to_bits(1.0e308) == 0x7FE1CCF385EBC8A0ULL);
    ISO_CHECK(to_bits(1e-17) == 0x3C670EF54646D497ULL);
    ISO_CHECK(to_bits(1e-300) == 0x01A56E1FC2F8F359ULL);
    ISO_CHECK(t::radians(1.0) == t::PI / 180.0);
    ISO_CHECK(t::degrees(1.0) == 180.0 / t::PI);

    return ISO_TEST_RESULT();
}
