// test_fuzz_exceptions.cpp -- the exception boundary of the fuzz binding (VER06k section 3).
//
// A C++ exception must never unwind through a C frame.  Every scenario here throws from user code
// that the C engine called, and requires: the process continues, the engine classified the throw
// BY TYPE (never by what()), nothing leaked (the C allocation counter and the handle ring are back
// at their baselines) and no message text reaches any rendering.  Under ASan this also proves no
// use-after-free on unwind.
#include <cstdio>
#include <new>
#include <stdexcept>
#include <string>
#include <vector>

#include "fuzz_bootstrap.hpp"
#include "iso_test.h"

extern "C" {
#include "fuzz_internal.h"
#include "pbt_hooks.h"
}

namespace {

namespace fz = ca::fuzz;

const char* const kSecret = "SECRET-PAYLOAD-0123456789";

std::string everything(const fz::Report& r) {
    std::string all = r.summary();
    for (const std::string& l : r.tap_failures("t")) all += l;
    for (const fz::CrashRecord& c : r.crashes()) {
        all += c.signature() + c.kind();
        for (const std::string& f : c.frames()) all += f;
    }
    return all;
}

// Run a scenario (whose Reports die at its closing brace), then require the C allocation counter and the
// handle ring to be exactly where they were.
void clean_scenario(const char* name, void (*scenario)()) {
    long allocs = pbt_hooks.live_allocs;
    std::size_t handles = fz_handles_live();
    scenario();
    if (pbt_hooks.live_allocs != allocs || fz_handles_live() != handles) std::printf("  %s leaked\n", name);
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, allocs);
    ISO_CHECK_EQ_UINT(fz_handles_live(), handles);
}

fz::Options opts(std::size_t iters) {
    fz::Options o;
    o.max_iterations = iters;
    return o;
}

// Every seed crashes at once, so iterations == 0 and the report is exactly the triage of the seeds.
fz::Report run_seeds(const std::vector<fz::Bytes>& seeds, fz::Target t, std::size_t iters = 0) {
    return fz::run(seeds, opts(iters), std::move(t));
}

// A type whose what() must never be called.
bool g_what_called = false;
struct Sneaky : std::exception {
    const char* what() const noexcept override {
        g_what_called = true;
        return kSecret;
    }
};
struct Plain {};  // not derived from std::exception

void test_std_exception_is_a_crash_by_type() {
    // Two seeds throw the SAME type with DIFFERENT messages: one signature, two hits, no message anywhere.
    fz::Report r = run_seeds({fz::Bytes{1}, fz::Bytes{2}}, [](fz::ByteView d, fz::Handle&) {
        throw std::runtime_error(std::string(kSecret) + std::to_string(d[0]));
    });
    std::vector<fz::CrashRecord> c = r.crashes();
    ISO_CHECK_EQ_UINT(c.size(), 1);
    if (c.size() == 1) {
        ISO_CHECK(c[0].kind().find("runtime_error") != std::string::npos);
        ISO_CHECK_EQ_UINT(c[0].hits(), 2);
        ISO_CHECK(c[0].frames().empty());
        ISO_CHECK(c[0].signature() == fz::crash_signature(c[0].kind(), {}));
    }
    ISO_CHECK(everything(r).find("SECRET") == std::string::npos);
    ISO_CHECK(!r.is_clean());
}

void test_distinct_types_are_distinct_crashes() {
    g_what_called = false;
    fz::Report r = run_seeds({fz::Bytes{0}, fz::Bytes{1}, fz::Bytes{2}, fz::Bytes{3}, fz::Bytes{4}},
                             [](fz::ByteView d, fz::Handle&) {
                                 switch (d[0]) {
                                     case 0: throw std::runtime_error("x");
                                     case 1: throw std::logic_error("x");
                                     case 2: throw 42;               // not a class at all
                                     case 3: throw Plain();          // a class, not a std::exception
                                     default: throw Sneaky();
                                 }
                             });
    ISO_CHECK_EQ_UINT(r.crashes().size(), 4);  // `throw 42` and `throw Plain()` share one signature
    std::vector<std::string> kinds;
    for (const fz::CrashRecord& c : r.crashes()) kinds.push_back(c.kind());
    bool saw_nonstd = false;
    int nonstd = 0;
    for (const std::string& k : kinds) {
        if (k == "non-std-exception") ++nonstd;
        saw_nonstd = saw_nonstd || k == "non-std-exception";
    }
    ISO_CHECK(saw_nonstd);
    ISO_CHECK_EQ_INT(nonstd, 1);  // neither carries a usable type: one fixed kind, two hits
    ISO_CHECK_EQ_UINT(r.unique_crashes(), 4);
    ISO_CHECK(!g_what_called);  // what() is NEVER read, not even for a std::exception subclass
    ISO_CHECK(everything(r).find("SECRET") == std::string::npos);
}

void test_crash_declares_kind_and_frames() {
    fz::Report r = run_seeds({fz::Bytes{1}}, [](fz::ByteView, fz::Handle&) {
        throw fz::Crash("boom", {"inner", "outer"});
    });
    std::vector<fz::CrashRecord> c = r.crashes();
    ISO_CHECK_EQ_UINT(c.size(), 1);
    if (c.size() == 1) {
        ISO_CHECK(c[0].kind() == "boom");
        ISO_CHECK((c[0].frames() == fz::Frames{"inner", "outer"}));
        ISO_CHECK(c[0].signature() == fz::crash_signature("boom", {"inner", "outer"}));
    }
    // the engine's normalisation applies: odd characters, an empty kind, hostile lengths
    fz::Report n = run_seeds({fz::Bytes{1}}, [](fz::ByteView, fz::Handle&) {
        throw fz::Crash("bad:kind with spaces\n", {"f:g", "f:g", "", std::string(500, 'f')});
    });
    std::vector<fz::CrashRecord> nc = n.crashes();
    ISO_CHECK_EQ_UINT(nc.size(), 1);
    if (nc.size() == 1) {
        ISO_CHECK(nc[0].kind() == "bad_kind_with_spaces_");
        ISO_CHECK_EQ_UINT(nc[0].frames().size(), 2);  // repeat collapsed, empty dropped
        ISO_CHECK(nc[0].frames()[0] == "f_g");
        ISO_CHECK_EQ_UINT(nc[0].frames()[1].size(), FZ_FRAME_MAX);
    }
    fz::Report e = run_seeds({fz::Bytes{1}}, [](fz::ByteView, fz::Handle&) { throw fz::Crash(""); });
    ISO_CHECK(e.crashes()[0].kind() == "unspecified");
    // Crash::what() is a fixed string
    ISO_CHECK(std::string(fz::Crash("secret").what()) == "ca::fuzz::Crash");
}

void test_bad_alloc_is_an_oom() {
    for (int which = 0; which < 2; ++which) {
        fz::Report r = run_seeds({fz::Bytes{1}}, [which](fz::ByteView, fz::Handle&) {
            if (which == 0) throw std::bad_alloc();
            throw std::bad_array_new_length();
        });
        ISO_CHECK_EQ_UINT(r.ooms(), 1);
        ISO_CHECK_EQ_UINT(r.timeouts(), 0);
        ISO_CHECK_EQ_UINT(r.crashes().size(), 0);  // never a crash signature
        ISO_CHECK(!r.is_clean() || r.iterations() == 0);
        std::vector<std::string> tap = r.tap_failures("t");
        ISO_CHECK_EQ_UINT(tap.size(), 1);
        if (tap.size() == 1)
            ISO_CHECK(tap[0] == "not ok 1 - t [fuzz-oom:" + fz::crash_signature("oom", {}) + "]");
    }
    // in the middle of a campaign too: oom is counted per execution and the campaign goes on
    fz::Report m = fz::run({fz::Bytes{'A', 'B'}}, opts(200), [](fz::ByteView d, fz::Handle&) {
        if (d.size() > 0 && d[0] == 'A') throw std::bad_alloc();
    });
    ISO_CHECK(m.ooms() >= 1);
    ISO_CHECK_EQ_UINT(m.crashes().size(), 0);
}

void test_first_exhausted_budget_wins() {
    // exhaust the step budget, THEN throw: a timeout, never a crash
    fz::Options o = opts(0);
    o.step_budget = 3;
    fz::Report t = fz::run({fz::Bytes{1}}, o, [](fz::ByteView, fz::Handle& h) {
        while (!h.tick()) {
        }
        throw std::runtime_error("after the budget broke");
    });
    ISO_CHECK_EQ_UINT(t.timeouts(), 1);
    ISO_CHECK_EQ_UINT(t.crashes().size(), 0);
    // exhaust the step budget, then throw bad_alloc: still the timeout (first budget wins)
    fz::Report t2 = fz::run({fz::Bytes{1}}, o, [](fz::ByteView, fz::Handle& h) {
        while (!h.tick()) {
        }
        throw std::bad_alloc();
    });
    ISO_CHECK_EQ_UINT(t2.timeouts(), 1);
    ISO_CHECK_EQ_UINT(t2.ooms(), 0);
    // exceed memory, then return normally: an oom, never a pass
    fz::Options m = opts(0);
    m.mem_budget = 100;
    fz::Report r = fz::run({fz::Bytes{1}}, m, [](fz::ByteView, fz::Handle& h) {
        ISO_CHECK(!h.reserve(60));
        ISO_CHECK(h.reserve(60));
        ISO_CHECK(h.tick());  // sticky and shared: steps are exhausted too
    });
    ISO_CHECK_EQ_UINT(r.ooms(), 1);
}

void test_minimizer_exceptions() {
    // a campaign finds the crash, a throwing MinimizeTest reproduces the SAME signature by type
    fz::Report r = run_seeds({fz::Bytes{1, 2, 3, 4, 5, 6}}, [](fz::ByteView d, fz::Handle&) {
        for (std::size_t i = 0; i < d.size(); ++i)
            if (d[i] == 3) throw std::runtime_error("target");
    }, 50);  // minimisation spends at most max_iterations executions in total
    std::vector<fz::CrashRecord> c = r.crashes();
    ISO_CHECK_EQ_UINT(c.size(), 1);
    if (c.size() != 1) return;
    ISO_CHECK(c[0].minimal_input().has_value());
    ISO_CHECK((c[0].minimal_input().value() == fz::Bytes{3}));
    fz::Bytes shrunk = fz::minimize(fz::Bytes{1, 2, 3, 4, 5, 6}, c[0].signature(), [](fz::ByteView d) {
        for (std::size_t i = 0; i < d.size(); ++i)
            if (d[i] == 3) throw std::runtime_error("a different message");
        return fz::Outcome::pass();
    }, 100);
    ISO_CHECK((shrunk == fz::Bytes{3}));
    // bad_alloc in the test is the oom outcome: it never reproduces a crash signature
    shrunk = fz::minimize(fz::Bytes{1, 2, 3}, c[0].signature(), [](fz::ByteView) -> fz::Outcome { throw std::bad_alloc(); },
                          100);
    ISO_CHECK((shrunk == fz::Bytes{1, 2, 3}));
    // OOM never reproduces ANY crash signature.  A crash whose fz_crash is empty normalises to kind
    // "unspecified" (signature e2f83322, pinned in test_fuzz_pinned), which a test that wrongly mapped
    // bad_alloc to an empty CRASH would reproduce and so shrink; the oom outcome must not.
    ISO_CHECK(fz::crash_signature("unspecified", {}) == "e2f83322");
    shrunk = fz::minimize(fz::Bytes{1, 2, 3, 4}, "e2f83322", [](fz::ByteView) -> fz::Outcome { throw std::bad_alloc(); },
                          100);
    ISO_CHECK((shrunk == fz::Bytes{1, 2, 3, 4}));
    // control: a real empty-kind crash outcome DOES reproduce that signature and shrinks to empty
    shrunk = fz::minimize(fz::Bytes{1, 2, 3, 4}, "e2f83322", [](fz::ByteView) { return fz::Outcome::crash(""); }, 100);
    ISO_CHECK(shrunk.empty());
    // a different type is a different crash
    shrunk = fz::minimize(fz::Bytes{1, 2, 3}, c[0].signature(), [](fz::ByteView) -> fz::Outcome {
        throw std::logic_error("other");
    }, 100);
    ISO_CHECK((shrunk == fz::Bytes{1, 2, 3}));
}

void test_nested_campaigns_and_binding_errors() {
    // an inner campaign inside a target: each has its own handle, the outer stays clean
    std::size_t inner_crashes = 0;
    fz::Report outer = fz::run({fz::Bytes{1}}, opts(5), [&](fz::ByteView, fz::Handle& h) {
        h.edge(7);
        fz::Report inner = run_seeds({fz::Bytes{9}}, [](fz::ByteView, fz::Handle&) { throw fz::Crash("inner"); });
        inner_crashes += inner.crashes().size();
        ISO_CHECK(h.alive());  // the outer handle survives the inner campaign
    });
    ISO_CHECK(outer.is_clean());
    ISO_CHECK_EQ_UINT(inner_crashes, 6);
    // an Error thrown BY the binding inside a target (an invalid inner option) is just another exception
    fz::Report bad = run_seeds({fz::Bytes{1}}, [](fz::ByteView, fz::Handle&) {
        fz::Options o;
        o.max_iterations = FZ_MAX_ITERATIONS + 1;
        (void)fz::run({}, o, [](fz::ByteView, fz::Handle&) {});
    });
    ISO_CHECK_EQ_UINT(bad.crashes().size(), 1);
    if (bad.crashes().size() == 1) ISO_CHECK(bad.crashes()[0].kind().find("Error") != std::string::npos);
}

void test_a_throwing_target_in_a_long_campaign_leaks_nothing() {
    // roughly one input in 3 throws; thousands of unwinds through the engine's frames
    fz::Report r = fz::run({fz::Bytes{'A', 'B', 'C', 'D'}}, opts(3000), [](fz::ByteView d, fz::Handle& h) {
        h.edge(1);
        if (d.size() > 0 && (d[0] % 3) == 0) throw std::runtime_error("x");
    });
    ISO_CHECK(r.unique_crashes() >= 1);
}

}  // namespace

int main() {
    pbt_hooks_reset();
    clean_scenario("std exception by type", test_std_exception_is_a_crash_by_type);
    clean_scenario("distinct types", test_distinct_types_are_distinct_crashes);
    clean_scenario("Crash kind and frames", test_crash_declares_kind_and_frames);
    clean_scenario("bad_alloc", test_bad_alloc_is_an_oom);
    clean_scenario("budget precedence", test_first_exhausted_budget_wins);
    clean_scenario("minimizer", test_minimizer_exceptions);
    clean_scenario("nested", test_nested_campaigns_and_binding_errors);
    clean_scenario("long campaign", test_a_throwing_target_in_a_long_campaign_leaks_nothing);
    return ISO_TEST_RESULT();
}
