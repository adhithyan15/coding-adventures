// test_fuzz_pinned.cpp -- the cross-lane values fuzz-v1 does not pin, checked THROUGH THE C++ API
// (VER06k section 6), plus unit tests of the pure pieces and of the API edges.
//
// The numbers come from the C lane's tests (test_fuzz_pinned.c, test_fuzz_engine.c), which took them
// from the independent Python oracle scripts/fuzz_conformance.py.  The C++ binding must reproduce
// them exactly: a wrapper that altered a signature, an energy tie, a TAP token or a minimiser step
// would pass fuzz-v1 and fail here.
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

#include "fuzz_bootstrap.hpp"
#include "iso_test.h"

extern "C" {
#include "pbt_hooks.h"
}

namespace {

namespace fz = ca::fuzz;

fz::Bytes hex(const char* h) {
    std::string s(h);
    fz::Bytes b(s.size() / 2);
    for (std::size_t i = 0; i < b.size(); ++i) {
        auto v = [](char c) { return c >= '0' && c <= '9' ? c - '0' : c - 'a' + 10; };
        b[i] = static_cast<std::uint8_t>(v(s[2 * i]) * 16 + v(s[2 * i + 1]));
    }
    return b;
}

// ---- signatures --------------------------------------------------------------------------------
void check_sig(const char* kind, const fz::Frames& frames, const char* want) {
    std::string got = fz::crash_signature(kind, frames);
    if (got != want) std::printf("  signature of %s is %s, oracle says %s\n", kind, got.c_str(), want);
    ISO_CHECK(got == want);
}

void test_signatures() {
    check_sig("panic", {"eval", "step"}, "4eca7ae8");
    check_sig("segv", {"alloc", "write"}, "a0aa3aa2");
    check_sig("timeout", {}, "4e61ea76");
    check_sig("oom", {}, "7146a410");
    check_sig("invalid", {}, "43a76e9e");
    check_sig("unspecified", {}, "e2f83322");
    check_sig("oob", {"parse", "main"}, "54a9c7a9");
    check_sig("oob", {"parse"}, "c151f40e");
    check_sig("assert", {"a", "b", "c"}, "0bf0dbc1");
    check_sig("", {}, "8601adad");
    check_sig("x", {"y"}, "7f6b2d3a");
    // frames are not escaped: one frame "x:y" and two frames "x", "y" hash alike (a documented limit)
    ISO_CHECK(fz::crash_signature("k", {"x:y"}) == fz::crash_signature("k", {"x", "y"}));
    ISO_CHECK(fz::crash_signature("panic", {"eval", "step"}) != fz::crash_signature("panic", {"alloc", "write"}));
    // embedded NULs are part of the text (the corpus strings may hold them)
    ISO_CHECK(fz::crash_signature(std::string("a\0b", 3), {}) != fz::crash_signature("a", {}));
    // duplicate detection
    ISO_CHECK(fz::is_duplicate_crash("panic", {"eval", "step"}, "panic", {"eval", "step"}));
    ISO_CHECK(!fz::is_duplicate_crash("panic", {"eval", "step"}, "segv", {"alloc", "write"}));
    ISO_CHECK(!fz::is_duplicate_crash("panic", {"eval", "step"}, "panic", {"eval"}));
}

// ---- the power schedule ------------------------------------------------------------------------
void check_energy(std::size_t base, std::size_t len, std::size_t nw, std::size_t tot, std::size_t want) {
    std::size_t got = fz::calculate_energy(base, len, nw, tot);
    if (got != want)
        std::printf("  energy(%lu,%lu,%lu,%lu) = %lu, oracle says %lu\n", (unsigned long)base, (unsigned long)len,
                    (unsigned long)nw, (unsigned long)tot, (unsigned long)got, (unsigned long)want);
    ISO_CHECK_EQ_UINT(got, want);
}

void test_energy() {
    // the *tie* rows have x exactly k + 0.5 and round UP
    check_energy(10, 4, 3, 6, 8);
    check_energy(5, 4, 0, 0, 3);
    check_energy(3, 4, 0, 0, 2);
    check_energy(7, 4, 0, 0, 4);
    check_energy(9, 36, 0, 0, 2);
    check_energy(25, 100, 0, 0, 3);
    check_energy(1, 16, 1, 1, 1);
    check_energy(10, 1, 0, 0, 10);
    check_energy(10, 100, 1, 2, 2);
    check_energy(0, 1, 0, 0, 1);
    check_energy(1, 1000000, 5, 5, 1);
    check_energy(10, 0, 0, 0, 10);
    check_energy(1000000, 1, 5, 5, 1000000);
    check_energy(2, 4, 0, 0, 1);
    check_energy(1, 4, 0, 0, 1);
    check_energy(15, 9, 0, 0, 5);
    check_energy(10, 9, 2, 4, 5);
    check_energy(11, 4, 1, 1, 11);
    check_energy(50, 4, 1, 2, 38);
    check_energy(10, 4, 9, 0, 5);
}

// ---- constants ---------------------------------------------------------------------------------
void test_constants() {
    ISO_CHECK((fz::interest_values(1) == std::vector<std::uint64_t>{0, 1, 127, 128, 255}));
    ISO_CHECK((fz::interest_values(2) == std::vector<std::uint64_t>{0, 1, 255, 256, 32767, 32768, 65535}));
    ISO_CHECK((fz::interest_values(4) ==
               std::vector<std::uint64_t>{0, 1, 65535, 65536, 0x7FFFFFFFu, 0x80000000u, 0xFFFFFFFFu}));
    ISO_CHECK((fz::interest_values(8) == std::vector<std::uint64_t>{0, 1, 0xFFFFFFFFu, 0x100000000u,
                                                                    0x7FFFFFFFFFFFFFFFu, 0x8000000000000000u,
                                                                    0xFFFFFFFFFFFFFFFFu}));
    ISO_CHECK(fz::interest_values(3).empty());
    ISO_CHECK(fz::interest_values(0).empty());
    ISO_CHECK(fz::interest_values(16).empty());
    ISO_CHECK((fz::interest8() == std::vector<std::uint8_t>{0, 1, 127, 128, 255}));
    ISO_CHECK((fz::deltas8() == std::vector<int>{-35, -8, -1, 1, 8, 35}));
    ISO_CHECK_EQ_INT(FZ_COVERAGE_MAP_SIZE, 65536);
    ISO_CHECK_EQ_INT(FZ_MAX_ITERATIONS, 1000000);
    ISO_CHECK_EQ_INT(FZ_MAX_TARGET_LEN, 128);
}

// ---- splice ------------------------------------------------------------------------------------
void check_splice(const char* t, const char* d, std::int64_t ts, std::int64_t ds, const char* want) {
    fz::Bytes got = fz::splice(hex(t), hex(d), ts, ds);
    if (got != hex(want)) std::printf("  splice(%s,%s,%ld,%ld) differs from oracle %s\n", t, d, (long)ts, (long)ds, want);
    ISO_CHECK(got == hex(want));
    // the BigInt path agrees
    ISO_CHECK(fz::splice(hex(t), hex(d), fz::BigInt(ts), fz::BigInt(ds)) == hex(want));
}

void test_splice() {
    const char* t = "0102030405";
    const char* d = "a0b1c2d3";
    check_splice(t, d, 1, 3, "01d3");
    check_splice(d, t, 3, 1, "a0b1c202030405");
    check_splice(t, d, 3, 1, "010203b1c2d3");
    check_splice(d, t, 1, 3, "a00405");
    check_splice(t, d, 0, 0, "a0b1c2d3");
    check_splice(d, t, 0, 0, "0102030405");
    check_splice(t, d, 5, 4, "0102030405");
    check_splice(t, d, 2, -1, "0102a0b1c2d3");
    check_splice(d, t, -1, 2, "030405");
    check_splice(t, d, -3, 2, "c2d3");
    check_splice(t, d, 9, 9, "0102030405");
    check_splice(t, d, 2, 2, "0102c2d3");
    check_splice(d, t, 2, 2, "a0b1030405");
}

// ---- mutator edges and limits -------------------------------------------------------------------
void test_mutator_edges() {
    const fz::Bytes two{0, 0};
    const fz::Bytes three{0x12, 0x34, 0x56};
    ISO_CHECK(fz::bit_flip(two, 0, INT64_MAX).size() == 2);  // parity: finishes at once
    ISO_CHECK(fz::bit_flip(two, 0, 17) == hex("feff"));
    ISO_CHECK(fz::byte_flip(three, -1, 1) == hex("1234a9"));  // floor modulo
    ISO_CHECK(fz::byte_flip(three, 0, 0) == three);
    ISO_CHECK(fz::byte_flip(fz::ByteView(), 0, 5).empty());
    ISO_CHECK(fz::byte_arithmetic(three, 0, 1, 3, fz::Endian::Little) == three);  // unsupported width
    ISO_CHECK(fz::byte_arithmetic(three, 0, 1, 4, fz::Endian::Little) == three);  // short buffer
    ISO_CHECK(fz::boundary_insert(three, 0, 0, 1, fz::Endian::Big) == three);
    const fz::Bytes ff{0xff, 0xff};
    ISO_CHECK(fz::byte_arithmetic(ff, 0, 1, 2, fz::Endian::Big) == hex("0000"));
    ISO_CHECK(fz::byte_arithmetic(ff, 0, -1, 2, fz::Endian::Little) == hex("feff"));
    ISO_CHECK(fz::block_insert(fz::ByteView(), 5, hex("0708")) == hex("0708"));
    ISO_CHECK(fz::block_duplicate(three, 1, 5, 3) == hex("1234563456"));
    ISO_CHECK(fz::block_delete(three, 2, 99) == hex("1234"));
    // NULL data with length 0 is legal for every mutator
    ISO_CHECK(fz::block_insert(fz::ByteView(), 0, fz::ByteView()).empty());
    ISO_CHECK(fz::block_delete(fz::ByteView(), 0, 3).empty());
    ISO_CHECK(fz::mutate_sequence(fz::ByteView(), 9, 20, 64).size() <= 64);
    // text bytes: a string_view is a byte view
    ISO_CHECK(fz::block_insert(std::string_view("ab"), 0, std::string_view("x")) == hex("786162"));
    // limits surface as Error carrying the engine status
    try {
        (void)fz::mutate_sequence(three, 1, FZ_MAX_ITERATIONS + 1, 100);
        ISO_CHECK_MSG(false, "an oversized step count must throw");
    } catch (const fz::Error& e) {
        ISO_CHECK_EQ_INT(e.status(), FZ_E_LIMIT);
    }
    try {
        (void)fz::bit_flip(fz::ByteView(three.data(), FZ_MAX_MUTATE_BYTES + 1), 0, 1);
        ISO_CHECK_MSG(false, "an oversized input must throw");
    } catch (const fz::Error& e) {
        ISO_CHECK_EQ_INT(e.status(), FZ_E_LIMIT);
    }
    // the mutators never touch their input
    fz::Bytes keep{1, 2, 3, 4};
    (void)fz::byte_flip(keep, 0, 4);
    ISO_CHECK((keep == fz::Bytes{1, 2, 3, 4}));
    // BigInt parsing: too wide / malformed text is an Error, a wide but legal value is accepted
    ISO_CHECK_MSG(fz::BigInt::parse(std::string(300, '9')).fits_i64() == false, "300 digits parse, and do not fit int64");
    bool threw = false;
    try {
        (void)fz::BigInt::parse(std::string(301, '9'));
    } catch (const fz::Error&) {
        threw = true;
    }
    ISO_CHECK(threw);
    threw = false;
    try {
        (void)fz::BigInt::parse("12a");
    } catch (const fz::Error& e) {
        threw = e.status() == FZ_E_LIMIT;
    }
    ISO_CHECK(threw);
    // 64-bit compare: ISO_CHECK_EQ_UINT goes through unsigned long, which is
    // 32 bits on Windows (LLP64) and would truncate this constant.
    ISO_CHECK(fz::BigInt(-1).mod_u64() == UINT64_MAX);
}

void test_sequence_work() {
    fz::Bytes in{1, 2, 3, 4, 5, 6, 7, 8};
    fz::SequenceResult r = fz::mutate_sequence_budget(in, 42, 4, 100, 7);
    ISO_CHECK_EQ_INT(r.status, FZ_E_WORK);
    ISO_CHECK_EQ_UINT(r.work_done, 0);
    ISO_CHECK(r.bytes.empty());
    r = fz::mutate_sequence_budget(in, 42, 0, 100, 8);  // zero steps cost exactly the initial copy
    ISO_CHECK_EQ_INT(r.status, FZ_OK);
    ISO_CHECK_EQ_UINT(r.work_done, 8);
    r = fz::mutate_sequence_budget(in, 42, 4, 100, UINT64_MAX);
    ISO_CHECK_EQ_INT(r.status, FZ_OK);
    ISO_CHECK(r.work_done >= 8 + 4);
    std::uint64_t full = r.work_done;
    fz::Bytes full_out = r.bytes;
    for (std::uint64_t cap = 8; cap < full; ++cap) {
        r = fz::mutate_sequence_budget(in, 42, 4, 100, cap);
        ISO_CHECK_EQ_INT(r.status, FZ_E_WORK);
        ISO_CHECK(r.work_done <= cap);
        ISO_CHECK(r.bytes.empty());
    }
    r = fz::mutate_sequence_budget(in, 42, 4, 100, full);
    ISO_CHECK_EQ_INT(r.status, FZ_OK);
    ISO_CHECK(r.bytes == full_out);
    // the plain call equals the budgeted one
    ISO_CHECK(fz::mutate_sequence(in, 42, 4, 100) == full_out);
    r = fz::mutate_sequence_budget(in, 42, FZ_MAX_ITERATIONS + 1, 100, UINT64_MAX);
    ISO_CHECK_EQ_INT(r.status, FZ_E_LIMIT);
    ISO_CHECK_EQ_UINT(r.work_done, 0);
}

void test_corpus_admit() {
    auto n = fz::corpus_admit({1, 2, 3}, {2, 4});
    ISO_CHECK(n.admitted && n.new_edges == 1);
    n = fz::corpus_admit({1, 2, 3}, {1, 2});
    ISO_CHECK(!n.admitted && n.new_edges == 0);
    n = fz::corpus_admit({1, 2, 3}, {4, 4, 4, 5});
    ISO_CHECK(n.admitted && n.new_edges == 2);
    n = fz::corpus_admit({}, {2, 4});
    ISO_CHECK(n.admitted && n.new_edges == 2);
    n = fz::corpus_admit({1, 2, 3}, {});
    ISO_CHECK(!n.admitted && n.new_edges == 0);
    n = fz::corpus_admit({1, 2, 3}, {65537});  // ids are compared exactly here (no masking)
    ISO_CHECK(n.admitted && n.new_edges == 1);
}

// ---- failure ids and TAP -----------------------------------------------------------------------
void expect_arg(int test_num, const char* target, const char* id, const char* sig) {
    try {
        (void)fz::format_tap_failure(test_num, target, id, sig);
        ISO_CHECK_MSG(false, "this TAP request must be refused");
    } catch (const fz::Error& e) {
        ISO_CHECK_EQ_INT(e.status(), FZ_E_ARG);
    }
}

void test_tap() {
    for (const char* id : {"fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"})
        ISO_CHECK(fz::is_failure_id(id));
    ISO_CHECK(!fz::is_failure_id("fuzz-arbitrary-error"));
    ISO_CHECK(!fz::is_failure_id(std::string("fuzz-crash\0x", 12)));
    ISO_CHECK(!fz::is_failure_id("fuzz-crash "));
    ISO_CHECK(!fz::is_failure_id(""));
    ISO_CHECK(!fz::is_failure_id("FUZZ-CRASH"));
    ISO_CHECK(!fz::is_failure_id("property-counterexample"));
    // the pinned corpus line
    ISO_CHECK(fz::format_tap_failure(1, "ct_compare", "fuzz-crash", "3a8f19b2") ==
              "not ok 1 - ct_compare [fuzz-crash:3a8f19b2]");
    // TAP tokens: '#' becomes '_', and so does everything outside [A-Za-z0-9._/-]
    ISO_CHECK(fz::format_tap_failure(1, "a#b", "fuzz-crash", "3a8f19b2") == "not ok 1 - a_b [fuzz-crash:3a8f19b2]");
    ISO_CHECK(fz::format_tap_failure(7, std::string("a\nnot ok 9 - x\r\t b\x01", 19), "fuzz-oom", "00ff00ff") ==
              "not ok 7 - a_not_ok_9_-_x___b_ [fuzz-oom:00ff00ff]");
    ISO_CHECK(fz::format_tap_failure(1, std::string("a\0b\xc3\xa9", 5), "fuzz-oom", "00ff00ff") ==
              "not ok 1 - a_b__ [fuzz-oom:00ff00ff]");
    // the empty target
    ISO_CHECK(fz::format_tap_failure(1, "", "fuzz-crash", "3a8f19b2") == "not ok 1 -  [fuzz-crash:3a8f19b2]");
    // the 128-byte cut
    std::string line = fz::format_tap_failure(1, std::string(300, 'x'), "fuzz-oom", "00ff00ff");
    ISO_CHECK_EQ_UINT(line.size(), std::string("not ok 1 -  [fuzz-oom:00ff00ff]").size() + 128);
    ISO_CHECK(line == "not ok 1 - " + std::string(128, 'x') + " [fuzz-oom:00ff00ff]");
    // refusals are Error(FZ_E_ARG): unregistered id; uppercase / short / long / non-hex signature
    expect_arg(1, "t", "fuzz-nope", "00ff00ff");
    expect_arg(1, "t", "fuzz-oom", "00FF00FF");
    expect_arg(1, "t", "fuzz-oom", "00ff00f");
    expect_arg(1, "t", "fuzz-oom", "00ff00ff0");
    expect_arg(1, "t", "fuzz-oom", "00ff00fg");
}

void test_edge_id() {
    ISO_CHECK(fz::edge_id("") == 0xcbf29ce484222325u);
    ISO_CHECK(fz::edge_id("a") != fz::edge_id("b"));
    ISO_CHECK(fz::edge_id("parse") == fz::edge_id(std::string("parse")));
    ISO_CHECK(fz::edge_id(std::string("par\0se", 6)) == fz::edge_id("par"));  // cut at the first NUL
}

// ---- the minimiser's call sequence (values from the C engine) -----------------------------------
void test_minimizer_trace() {
    const std::string sig = fz::crash_signature("k", {});
    ISO_CHECK(sig == "b5621210");
    std::vector<std::string> seen;
    fz::MinimizeTest test = [&](fz::ByteView d) {
        static const char digits[] = "0123456789abcdef";
        std::string h;
        bool has3 = false;
        for (std::size_t i = 0; i < d.size(); ++i) {
            h.push_back(digits[d[i] >> 4]);
            h.push_back(digits[d[i] & 15]);
            has3 = has3 || d[i] == 3;
        }
        seen.push_back(h);
        return has3 ? fz::Outcome::crash("k") : fz::Outcome::pass();
    };
    fz::Bytes out = fz::minimize(fz::Bytes{1, 2, 3, 4, 5, 6}, sig, test, 100);
    ISO_CHECK(out == fz::Bytes{3});
    const std::vector<std::string> want = {"040506", "010203", "0203", "03", "", "00"};
    if (seen != want) {
        std::printf("  minimiser trace differs:");
        for (const std::string& s : seen) std::printf(" [%s]", s.c_str());
        std::printf("\n");
    }
    ISO_CHECK(seen == want);

    // budgets: every REJECTED candidate counts; budget 0 runs nothing; an always-reproducing test reaches empty
    std::size_t calls = 0;
    fz::Bytes data(100);
    for (std::size_t i = 0; i < data.size(); ++i) data[i] = static_cast<std::uint8_t>(i + 1);
    fz::MinimizeTest never = [&](fz::ByteView) {
        ++calls;
        return fz::Outcome::pass();
    };
    ISO_CHECK(fz::minimize(data, fz::crash_signature("k", {}), never, 10) == data);
    ISO_CHECK_EQ_UINT(calls, 10);
    calls = 0;
    ISO_CHECK(fz::minimize(data, fz::crash_signature("k", {}), never, 0) == data);
    ISO_CHECK_EQ_UINT(calls, 0);
    fz::MinimizeTest always = [&](fz::ByteView) {
        ++calls;
        return fz::Outcome::crash("k");
    };
    calls = 0;
    ISO_CHECK(fz::minimize(data, fz::crash_signature("k", {}), always, 100000).empty());
    ISO_CHECK(calls < 200);
    // a different signature is not "the same crash"
    calls = 0;
    ISO_CHECK(fz::minimize(data, fz::crash_signature("other", {}), always, 50) == data);
    // argument validation
    for (const char* bad : {"ZZZZZZZZ", "abc", "b56212100"}) {
        bool threw = false;
        try {
            (void)fz::minimize(data, bad, always, 1);
        } catch (const fz::Error& e) {
            threw = e.status() == FZ_E_ARG;
        }
        ISO_CHECK(threw);
    }
    bool threw = false;
    try {
        (void)fz::minimize(data, sig, always, FZ_MAX_MINIMIZE_BUDGET + 1);
    } catch (const fz::Error& e) {
        threw = e.status() == FZ_E_ARG;
    }
    ISO_CHECK(threw);
}

// ---- Options / Report / Handle basics ------------------------------------------------------------
void test_options_and_report() {
    fz::Options o;
    fz_options d = fz_options_default();
    ISO_CHECK(o.seed == d.seed && o.max_iterations == d.max_iterations && o.max_input_len == d.max_input_len);
    ISO_CHECK(o.base_energy == d.base_energy && o.minimize_budget == d.minimize_budget);
    ISO_CHECK(o.step_budget == d.step_budget && o.mem_budget == d.mem_budget);
    // the engine validates: an option above its ceiling is Error(FZ_E_ARG)
    o.max_iterations = FZ_MAX_ITERATIONS + 1;
    bool threw = false;
    try {
        (void)fz::run({}, o, [](fz::ByteView, fz::Handle&) {});
    } catch (const fz::Error& e) {
        threw = e.status() == FZ_E_ARG;
    }
    ISO_CHECK(threw);

    // a clean campaign: tier black-box, counts, summary without bytes
    fz::Options opts;
    opts.max_iterations = 100;
    std::size_t calls = 0;
    fz::Report r = fz::run({fz::Bytes{'A', 'B', 'C'}}, opts, [&](fz::ByteView, fz::Handle&) { ++calls; });
    ISO_CHECK_EQ_UINT(r.iterations(), 100);
    ISO_CHECK_EQ_UINT(calls, 101);
    ISO_CHECK(r.is_clean());
    ISO_CHECK(r.tier() == FZ_TIER_BLACKBOX);
    ISO_CHECK(r.summary().find("iterations=100") != std::string::npos);
    ISO_CHECK(r.tap_failures("t").empty());
    ISO_CHECK_EQ_UINT(r.corpus().size(), 1);

    // Report is move-only: a moved-from report is the empty one
    fz::Report moved = std::move(r);
    ISO_CHECK_EQ_UINT(moved.iterations(), 100);
    ISO_CHECK_EQ_UINT(r.iterations(), 0);  // NOLINT: moved-from is specified to be empty
    static_assert(!std::is_copy_constructible<fz::Report>::value, "Report must not be copyable");
    static_assert(!std::is_copy_assignable<fz::Report>::value, "Report must not be copyable");
    static_assert(std::is_nothrow_move_constructible<fz::Report>::value, "Report moves are noexcept");

    // the staircase: coverage feedback finds the crash (for at least 5 of 6 seeds); the minimal input is
    // private, TAP is opaque and the campaign is deterministic
    auto stairs = [](fz::ByteView d, fz::Handle& h) {
        h.edge(1);
        if (d.size() > 0 && d[0] == 0xFF) {
            h.edge(2);
            if (d.size() > 1 && d[1] == 0x80) {
                h.edge(3);
                if (d.size() > 2 && d[2] == 0x7F) throw fz::Crash("staircase", {"stage3"});
            }
        }
    };
    int cracked = 0;
    for (std::uint64_t seed = 1; seed <= 6; ++seed) {
        fz::Options so;
        so.seed = seed;
        so.max_iterations = 2000;
        fz::Report s = fz::run({fz::Bytes{'A', 'B', 'C'}}, so, stairs);
        if (s.unique_crashes() != 1) continue;
        ++cracked;
        if (cracked > 1) continue;
        ISO_CHECK(s.tier() == FZ_TIER_CALLER);
        std::vector<fz::CrashRecord> cr = s.crashes();
        ISO_CHECK_EQ_UINT(cr.size(), 1);
        if (cr.size() != 1) return;
        ISO_CHECK(cr[0].kind() == "staircase");
        ISO_CHECK((cr[0].frames() == fz::Frames{"stage3"}));
        ISO_CHECK(cr[0].signature() == fz::crash_signature("staircase", {"stage3"}));
        ISO_CHECK(cr[0].minimized() && !cr[0].input_dropped());
        ISO_CHECK(cr[0].minimal_input().has_value());
        if (cr[0].minimal_input().has_value()) ISO_CHECK((*cr[0].minimal_input() == fz::Bytes{0xFF, 0x80, 0x7F}));
        std::vector<std::string> tap = s.tap_failures("stair#case", 3);
        ISO_CHECK_EQ_UINT(tap.size(), 1);
        if (tap.size() == 1) ISO_CHECK(tap[0] == "not ok 3 - stair_case [fuzz-crash:" + cr[0].signature() + "]");
        ISO_CHECK(s.summary().find("clean=0") != std::string::npos);
        fz::Report s2 = fz::run({fz::Bytes{'A', 'B', 'C'}}, so, stairs);
        ISO_CHECK(s.iterations() == s2.iterations() && s.total_edges() == s2.total_edges() &&
                  s.admitted_seeds() == s2.admitted_seeds() && s2.unique_crashes() == 1);
    }
    std::printf("  staircase: %d/6 seeds cracked\n", cracked);
    ISO_CHECK(cracked >= 5);
}

}  // namespace

int main() {
    pbt_hooks_reset();
    long baseline = pbt_hooks.live_allocs;
    test_signatures();
    test_energy();
    test_constants();
    test_splice();
    test_mutator_edges();
    test_sequence_work();
    test_corpus_admit();
    test_tap();
    test_edge_id();
    test_minimizer_trace();
    test_options_and_report();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
    return ISO_TEST_RESULT();
}
