// test_exceptions.cpp -- exception safety and out-of-memory discipline of the
// binding (VER05j sections 4 and 7).
//
// The rule under test: NO EXCEPTION THROWN BY A USER LAMBDA CROSSES A C FRAME.
// For every place a lambda can run (generation, a property, a filter predicate, a
// custom shrinker) and for three kinds of exception (a std::exception, a type
// that is not a std::exception, and std::bad_alloc) the result must be a VERDICT
// with the documented failure id, and afterwards the C engine's live-allocation
// counter must be back at its baseline: RAII released everything, including on
// unwind.  Under ASan the same runs also prove there is no use-after-free.
//
// The second half drives the C engine's out-of-memory seam (pbt_hooks.fail_after)
// THROUGH the binding: for N = 1, 2, ... the N-th engine allocation fails, and
// every run must either throw a pbt::Error or return a verdict, never claim
// success once an allocation failed, and leak nothing.
#include <cstdio>
#include <cstdint>
#include <new>
#include <stdexcept>
#include <string>
#include <vector>

#include "iso_test.h"
#include "test_bootstrap.hpp"

extern "C" {
#include "pbt_hooks.h"
}

namespace pbt = ca::pbt;

namespace {

// ---- the three exceptions ------------------------------------------------------------------
struct NotStd {};  // deliberately not derived from std::exception

enum class Boom { Runtime, NotStd, BadAlloc };
const Boom kBooms[] = {Boom::Runtime, Boom::NotStd, Boom::BadAlloc};
const char* name(Boom b) { return b == Boom::Runtime ? "runtime_error" : (b == Boom::NotStd ? "non-std type" : "bad_alloc"); }

[[noreturn]] void explode(Boom b) {
    switch (b) {
        case Boom::Runtime: throw std::runtime_error("boom");
        case Boom::NotStd: throw NotStd();
        default: throw std::bad_alloc();
    }
}

pbt::Options unredacted() {
    pbt::Options o;
    o.ci = pbt::CiMode::Off;
    o.num_tests = 20;
    return o;
}

// Run one scenario: it must not throw, it must leak nothing, and it yields a verdict.
template <class F>
pbt::Verdict contained(const char* what, Boom b, F f, bool* escaped = nullptr) {
    long baseline = pbt_hooks.live_allocs;
    bool threw = false;
    pbt::Verdict v = [&]() {
        try {
            return f();
        } catch (...) {
            threw = true;
            return pbt::check_property(0, pbt::gen::boolean(), [](pbt::ValueView) { return false; });
        }
    }();
    if (threw) std::printf("  ESCAPED: %s thrown from %s reached the caller\n", name(b), what);
    ISO_CHECK(!threw);
    if (escaped != nullptr) *escaped = threw;
    if (pbt_hooks.live_allocs != baseline)
        std::printf("  LEAK: %s / %s: %ld live, baseline %ld\n", what, name(b), pbt_hooks.live_allocs, baseline);
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
    return v;
}

// ---- generation ---------------------------------------------------------------------------------
void test_throw_in_generation() {
    for (Boom b : kBooms) {
        // map
        pbt::Verdict m = contained("map", b, [&] {
            return pbt::check_property(1, pbt::gen::integer(0, 9).map([b](pbt::ValueView) -> pbt::Value { explode(b); }),
                                       [](pbt::ValueView) { return true; }, unredacted());
        });
        ISO_CHECK(!m.passed());
        ISO_CHECK(m.failure_id() == PBT_FAIL_EXHAUSTED);
        ISO_CHECK(m.message().find("generator error") != std::string::npos);
        // bind
        pbt::Verdict bd = contained("bind", b, [&] {
            return pbt::check_property(1, pbt::gen::integer(0, 9).bind([b](pbt::ValueView) -> pbt::Gen { explode(b); }),
                                       [](pbt::ValueView) { return true; }, unredacted());
        });
        ISO_CHECK(bd.failure_id() == PBT_FAIL_EXHAUSTED);
        // a custom sampler
        pbt::Verdict cs = contained("sampler", b, [&] {
            return pbt::check_property(1, pbt::Gen::custom([b](pbt::Run&) -> pbt::Value { explode(b); }),
                                       [](pbt::ValueView) { return true; }, unredacted());
        });
        ISO_CHECK(cs.failure_id() == PBT_FAIL_EXHAUSTED);
        // a throw deep inside a composite: list(tuple(u8, map(throws)))
        pbt::Verdict deep = contained("composite", b, [&] {
            pbt::Gen bad = pbt::gen::u8().map([b](pbt::ValueView) -> pbt::Value { explode(b); });
            return pbt::check_property(1, pbt::gen::list(pbt::gen::tuple({pbt::gen::u8(), bad}), 1, 3),
                                       [](pbt::ValueView) { return true; }, unredacted());
        });
        ISO_CHECK(deep.failure_id() == PBT_FAIL_EXHAUSTED);
        // a throw that happens only on the 3rd draw (earlier values must be released)
        pbt::Verdict late = contained("late map", b, [&] {
            int calls = 0;
            return pbt::check_property(
                1, pbt::gen::integer(0, 9).map([&calls, b](pbt::ValueView v) -> pbt::Value {
                    if (++calls == 3) explode(b);
                    return pbt::Value::integer(v.as_i64());
                }),
                [](pbt::ValueView) { return true; }, unredacted());
        });
        ISO_CHECK(late.failure_id() == PBT_FAIL_EXHAUSTED);
        ISO_CHECK_EQ_INT(late.cases_run(), 2);  // two cases ran before generation failed: never a pass
    }
}

// Direct use: Gen::sample throws a StatusError carrying the mapped status (this pins the table in the spec).
void test_status_mapping_through_sample() {
    for (Boom b : kBooms) {
        pbt::Gen g = pbt::gen::integer(0, 9).map([b](pbt::ValueView) -> pbt::Value { explode(b); });
        pbt::Rng rng(1);
        int status = 0;
        try {
            (void)g.sample(rng);
        } catch (const pbt::StatusError& e) {
            status = e.status();
        }
        ISO_CHECK_EQ_INT(status, b == Boom::BadAlloc ? PBT_E_NOMEM : PBT_E_GEN);
        // the same for a sampler and for bind
        pbt::Gen s = pbt::Gen::custom([b](pbt::Run&) -> pbt::Value { explode(b); });
        status = 0;
        try {
            (void)s.sample(rng);
        } catch (const pbt::StatusError& e) {
            status = e.status();
        }
        ISO_CHECK_EQ_INT(status, b == Boom::BadAlloc ? PBT_E_NOMEM : PBT_E_GEN);
        pbt::Gen bd = pbt::gen::integer(0, 9).bind([b](pbt::ValueView) -> pbt::Gen { explode(b); });
        status = 0;
        try {
            (void)bd.sample(rng);
        } catch (const pbt::StatusError& e) {
            status = e.status();
        }
        ISO_CHECK_EQ_INT(status, b == Boom::BadAlloc ? PBT_E_NOMEM : PBT_E_GEN);
    }
    // A StatusError thrown by the user carries its own code, and Run::sample propagates an inner failure's code.
    pbt::Gen limit = pbt::Gen::custom([](pbt::Run&) -> pbt::Value { throw pbt::StatusError(PBT_E_LIMIT); });
    pbt::Rng rng(1);
    int status = 0;
    try {
        (void)limit.sample(rng);
    } catch (const pbt::StatusError& e) {
        status = e.status();
    }
    ISO_CHECK_EQ_INT(status, PBT_E_LIMIT);
    pbt::Gen inner_never = pbt::gen::u8().filter([](pbt::ValueView) { return false; }, 3);
    pbt::Gen outer = pbt::Gen::custom([inner_never](pbt::Run& run) { return run.sample(inner_never); });
    pbt::Verdict v = contained("inner exhaustion", Boom::Runtime, [&] {
        return pbt::check_property(1, outer, [](pbt::ValueView) { return true; }, unredacted());
    });
    ISO_CHECK(v.failure_id() == PBT_FAIL_EXHAUSTED);
    ISO_CHECK(v.message().find("filter precondition") != std::string::npos);  // PBT_E_EXHAUSTED passed through
}

// ---- the property ---------------------------------------------------------------------------------
void test_throw_in_property() {
    for (Boom b : kBooms) {
        pbt::Verdict v = contained("property", b, [&] {
            return pbt::check_property(1, pbt::gen::integer(0, 9), [b](pbt::ValueView) -> bool { explode(b); }, unredacted());
        });
        ISO_CHECK(!v.passed());
        ISO_CHECK(v.failure_id() == PBT_FAIL_PANIC);
        ISO_CHECK(v.seed().has_value());  // non-redacted panic carries the seed for replay
        // A property that throws only on the 5th case: four held, then a panic verdict (never a pass).
        pbt::Verdict late = contained("late property", b, [&] {
            int calls = 0;
            return pbt::check_property(
                1, pbt::gen::list(pbt::gen::u8(), 0, 8),
                [&calls, b](pbt::ValueView) -> bool {
                    if (++calls == 5) explode(b);
                    return true;
                },
                unredacted());
        });
        ISO_CHECK(late.failure_id() == PBT_FAIL_PANIC);
        ISO_CHECK_EQ_INT(late.iteration(), 5);
        // A property that throws only WHILE SHRINKING: the engine treats it as "no longer fails" and still
        // reports the counterexample.
        pbt::Verdict shrinking = contained("property during shrink", b, [&] {
            return pbt::check_property(
                1, pbt::gen::integer(0, 1000),
                [b](pbt::ValueView x) -> bool {
                    std::int64_t v = x.as_i64();
                    if (v > 100 && v < 700) explode(b);
                    return v <= 100;
                },
                unredacted());
        });
        ISO_CHECK(!shrinking.passed());
        ISO_CHECK(shrinking.failure_id() == PBT_FAIL_COUNTEREXAMPLE || shrinking.failure_id() == PBT_FAIL_PANIC);
        // A nested check whose inner property throws yields an inner panic verdict, not an outer crash.
        pbt::Verdict nested = contained("nested property", b, [&] {
            return pbt::check_property(
                1, pbt::gen::boolean(),
                [b](pbt::ValueView, pbt::Scope scope) {
                    pbt::Verdict iv = pbt::check_property(scope, 2, pbt::gen::boolean(), [b](pbt::ValueView) -> bool { explode(b); },
                                                          unredacted());
                    return iv.failure_id() == PBT_FAIL_PANIC;
                },
                unredacted());
        });
        ISO_CHECK(nested.passed());
    }
}

// ---- the filter predicate -----------------------------------------------------------------------------
void test_throw_in_filter() {
    for (Boom b : kBooms) {
        pbt::Verdict v = contained("filter", b, [&] {
            return pbt::check_property(1, pbt::gen::integer(0, 9).filter([b](pbt::ValueView) -> bool { explode(b); }),
                                       [](pbt::ValueView) { return true; }, unredacted());
        });
        ISO_CHECK(v.failure_id() == PBT_FAIL_EXHAUSTED);
        // A predicate that throws only once shrinking has begun (the property flips the flag).
        pbt::Verdict shrinking = contained("filter during shrink", b, [&] {
            bool shrinking_started = false;
            return pbt::check_property(
                1, pbt::gen::integer(0, 1000).filter([&shrinking_started, b](pbt::ValueView) -> bool {
                    if (shrinking_started) explode(b);
                    return true;
                }),
                [&shrinking_started](pbt::ValueView x) {
                    bool holds = x.as_i64() < 0;  // always fails
                    if (!holds) shrinking_started = true;
                    return holds;
                },
                unredacted());
        });
        ISO_CHECK(shrinking.failure_id() == PBT_FAIL_COUNTEREXAMPLE);
    }
}

// ---- the custom shrinker ----------------------------------------------------------------------------------
void test_throw_in_shrinker() {
    for (Boom b : kBooms) {
        // An override shrinker that throws: the original witness is reported with zero steps.
        pbt::ShrinkFn bad = [b](pbt::Value& value, const pbt::Falsifies&, int) -> int {
            value = pbt::Value::integer(-999);  // scribble first, then throw: the engine must not believe the scribble
            explode(b);
        };
        pbt::Verdict v = contained("override shrinker", b, [&] {
            return pbt::check_property(1, pbt::gen::integer(0, 1000), [](pbt::ValueView x) { return x.as_i64() < 0; }, unredacted(), bad);
        });
        ISO_CHECK(v.failure_id() == PBT_FAIL_COUNTEREXAMPLE);
        ISO_CHECK(v.minimal().has_value() && v.counterexample().has_value());
        ISO_CHECK(*v.minimal() == *v.counterexample());
        ISO_CHECK_EQ_INT(v.shrink_steps(), 0);
        // The same shrinker installed on a custom generator.
        pbt::Verdict g = contained("generator shrinker", b, [&] {
            pbt::Gen cg = pbt::Gen::custom([](pbt::Run& run) { return pbt::Value::integer(static_cast<std::int64_t>(run.bounded(1000))); }, bad);
            return pbt::check_property(1, cg, [](pbt::ValueView x) { return x.as_i64() < 0; }, unredacted());
        });
        ISO_CHECK(g.failure_id() == PBT_FAIL_COUNTEREXAMPLE);
        ISO_CHECK(*g.minimal() == *g.counterexample());
        // A shrinker that throws from inside the still-fails predicate it was handed.
        pbt::ShrinkFn inner = [b](pbt::Value& value, const pbt::Falsifies& still_fails, int) -> int {
            (void)still_fails(value);
            explode(b);
        };
        pbt::Verdict i = contained("shrinker after predicate", b, [&] {
            return pbt::check_property(1, pbt::gen::integer(0, 1000), [](pbt::ValueView x) { return x.as_i64() < 0; }, unredacted(), inner);
        });
        ISO_CHECK(i.failure_id() == PBT_FAIL_COUNTEREXAMPLE);
    }
}

// ---- a lambda destroyed mid-flight --------------------------------------------------------------------------
// The closures' captured state must be released exactly once, throwing or not.
void test_captured_state_released() {
    std::shared_ptr<int> witness = std::make_shared<int>(0);
    for (Boom b : kBooms) {
        long baseline = pbt_hooks.live_allocs;
        {
            pbt::Gen g = pbt::gen::integer(0, 9).map([witness, b](pbt::ValueView) -> pbt::Value { explode(b); });
            pbt::Verdict v = pbt::check_property(1, g, [witness](pbt::ValueView) { return true; }, unredacted());
            ISO_CHECK(!v.passed());
        }
        ISO_CHECK_EQ_INT(witness.use_count(), 1);
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
    }
}

// ---- OOM injection through the binding -------------------------------------------------------------------------
long sweep_runs = 0;

// A scenario returns 1 when it CLAIMS success.  A strict scenario may never claim success once an allocation
// failed; a non-strict one may degrade (a lost message does not change the verdict) but may not leak or crash.
template <class F>
void sweep(const char* what, F scenario, bool strict) {
    ISO_CHECK_MSG(scenario() == 1, what);  // without injection it succeeds
    for (long n = 1; n < 20000; ++n) {
        long baseline = pbt_hooks.live_allocs;
        long before = pbt_hooks.injected;
        int claims = 0;
        pbt_hooks.fail_after = n;
        try {
            claims = scenario();
        } catch (const pbt::Error&) {
            claims = 0;
        } catch (const std::bad_alloc&) {
            claims = 0;
        } catch (...) {
            pbt_hooks.fail_after = 0;
            ISO_CHECK_MSG(false, "an unexpected exception type escaped the binding");
            return;
        }
        pbt_hooks.fail_after = 0;
        bool fired = pbt_hooks.injected != before;
        ++sweep_runs;
        if (strict && fired && claims) std::printf("  %s: claimed success although allocation %ld failed\n", what, n);
        ISO_CHECK(!(strict && fired && claims));
        if (pbt_hooks.live_allocs != baseline)
            std::printf("  %s: leak when allocation %ld failed (%ld live, baseline %ld)\n", what, n, pbt_hooks.live_allocs, baseline);
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        if (!fired) return;  // n exceeded the scenario's allocations: the sweep is complete
    }
    ISO_CHECK_MSG(false, "the sweep did not terminate");
}

int scenario_values() {
    std::vector<pbt::Value> items;
    items.push_back(pbt::Value::integer(1));
    items.push_back(pbt::Value::bytes(std::vector<std::uint8_t>{1, 2, 3}));
    items.push_back(pbt::Value::string("abc"));
    items.push_back(pbt::Value::list({pbt::Value::integer_dec("340282366920938463463374607431768211455")}));
    pbt::Value l = pbt::Value::list(std::move(items));
    pbt::Value copy = l;
    pbt::Value t = pbt::Value::tuple({copy, l});
    std::string c = t.canonical();
    return (!c.empty() && copy == l) ? 1 : 0;
}

int scenario_passing_check() {
    pbt::Gen elem = pbt::gen::tuple({pbt::gen::u8(), pbt::gen::bytes(0, 6)});
    pbt::Gen g = pbt::gen::list(elem, 0, 4).map([](pbt::ValueView v) { return pbt::Value::from_c(*v.raw()); });
    pbt::Gen f = g.filter([](pbt::ValueView v) { return v.size() < 100; });
    pbt::Options o = unredacted();
    o.num_tests = 5;
    o.max_shrink_steps = 50;
    pbt::Verdict v = pbt::check_property(7, f, [](pbt::ValueView) { return true; }, o);
    return v.passed() ? 1 : 0;
}

int scenario_combinators() {
    pbt::Gen a = pbt::gen::one_of({pbt::gen::u8(), pbt::gen::bytes(1, 3)});
    pbt::Gen b = pbt::gen::frequency({{2, pbt::gen::boolean()}, {3, a}});
    pbt::Gen c = pbt::gen::integer(1, 3).bind([](pbt::ValueView n) { return pbt::gen::list(pbt::gen::u8(), 0, static_cast<std::size_t>(n.as_i64())); });
    pbt::Rng rng(4);
    for (int i = 0; i < 3; ++i) {
        (void)b.sample(rng);
        (void)c.sample(rng);
    }
    return 1;
}

int scenario_failing_check() {
    // A falsified check with shrinking, a custom shrinker and a nested check.
    pbt::ShrinkFn jump = [](pbt::Value& value, const pbt::Falsifies& still_fails, int) {
        pbt::Value cand = pbt::Value::integer(7);
        if (still_fails(cand)) {
            value = cand;
            return 1;
        }
        return 0;
    };
    pbt::Options o = unredacted();
    o.num_tests = 5;
    pbt::Verdict v = pbt::check_property(5, pbt::gen::integer(0, 1000), [](pbt::ValueView x) { return x.as_i64() < 0; }, o, jump);
    pbt::Verdict w = pbt::check_property(5, pbt::gen::list(pbt::gen::integer(0, 50), 0, 6),
                                         [](pbt::ValueView l) {
                                             std::int64_t s = 0;
                                             for (std::size_t i = 0; i < l.size(); ++i) s += l.at(i).as_i64();
                                             return s < 30;
                                         },
                                         o);
    return (!v.passed() && !w.passed()) ? 1 : 0;  // "claims success" is a PASS: a failing check never claims it
}

int scenario_standalone() {
    pbt::Shrunk s = pbt::shrink_int(pbt::Value::integer(500), pbt::Value::integer(0), pbt::Value::integer(0), pbt::Value::integer(1000),
                                    [](pbt::ValueView x) { return x.cmp_i64(100) <= 0; });
    pbt::Shrunk b = pbt::shrink_bytes(pbt::Value::bytes(std::vector<std::uint8_t>{1, 2, 3, 4, 5, 6}), [](pbt::ValueView v) { return v.size() < 5; });
    pbt::Replay r = pbt::simulate_replay(42, [](pbt::ValueView x) { return x.cmp_i64(100) <= 0; });
    return (s.minimal.as_i64() == 101 && b.minimal.size() == 5 && r.minimal.as_i64() == 101) ? 1 : 0;
}

void test_oom_sweep() {
    sweep("values", scenario_values, true);
    sweep("passing check", scenario_passing_check, true);
    sweep("combinators", scenario_combinators, true);
    sweep("standalone shrinkers", scenario_standalone, true);
    sweep("failing check", scenario_failing_check, false);
    std::printf("  oom sweep: %ld injected runs\n", sweep_runs);
    ISO_CHECK(sweep_runs > 100);
}

}  // namespace

int main() {
    pbt_hooks_reset();
    long baseline = pbt_hooks.live_allocs;
    test_throw_in_generation();
    test_status_mapping_through_sample();
    test_throw_in_property();
    test_throw_in_filter();
    test_throw_in_shrinker();
    test_captured_state_released();
    test_oom_sweep();
    pbt_hooks_reset();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);  // nothing leaked by the whole file
    return ISO_TEST_RESULT();
}
