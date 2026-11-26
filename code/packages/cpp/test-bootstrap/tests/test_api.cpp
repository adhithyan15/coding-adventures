// test_api.cpp -- unit tests for the C++ binding surface (VER05j section 3).
//
// These tests pin the BINDING's behaviour: ownership, conversions, builders,
// verdict shape and redaction.  Engine semantics (PRNG stream, shrink order,
// step counts) are pinned by the corpus test, not re-derived here.
#include <cstdint>
#include <string>
#include <vector>

#include "iso_test.h"
#include "test_bootstrap.hpp"

namespace pbt = ca::pbt;

namespace {

pbt::Options unredacted() {
    pbt::Options o;
    o.ci = pbt::CiMode::Off;
    return o;
}

// Run `f`, return true iff it throws a pbt::Error of type E.
template <class E, class F>
bool throws(F f) {
    try {
        f();
    } catch (const E&) {
        return true;
    }
    return false;
}

// ---- Value ---------------------------------------------------------------------------------
void test_value_scalars() {
    ISO_CHECK(pbt::Value().kind() == pbt::Kind::Nil);
    ISO_CHECK(pbt::Value::boolean(true).as_bool());
    ISO_CHECK(!pbt::Value::boolean(false).as_bool());
    ISO_CHECK_EQ_INT(pbt::Value::integer(-42).as_i64(), -42);
    ISO_CHECK(pbt::Value::integer_u64(18446744073709551615ull).as_u64() == 18446744073709551615ull);
    ISO_CHECK(pbt::Value::integer(-1).to_u64_wrapping() == 18446744073709551615ull);
    ISO_CHECK(pbt::Value::integer(5).cmp_i64(5) == 0);
    ISO_CHECK(pbt::Value::integer(5).cmp_i64(6) < 0);
    ISO_CHECK(pbt::Value::integer(-5).cmp_i64(-6) > 0);
    ISO_CHECK(pbt::Value::string("hi").as_string() == "hi");
    ISO_CHECK(pbt::Value::string("").as_string().empty());
    pbt::Value two_nul = pbt::Value::string(std::string_view("a\0b", 3));
    ISO_CHECK_EQ_UINT(two_nul.size(), 3);
    ISO_CHECK_EQ_UINT(two_nul.as_string().size(), 3);
}

void test_value_wide_integers() {
    const std::string big = "340282366920938463463374607431768211455";  // 2^128 - 1
    pbt::Value v = pbt::Value::integer_dec(big);
    ISO_CHECK(v.decimal() == big);
    ISO_CHECK(throws<pbt::Error>([&] { (void)v.as_u64(); }));
    ISO_CHECK(throws<pbt::Error>([&] { (void)v.as_i64(); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::Value::integer_dec("12x"); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::Value::integer_dec(""); }));
    ISO_CHECK(pbt::Value::integer_dec("-0").decimal() == "0");
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::Value::integer_u64(1).as_bool(); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::Value::integer(-1).as_u64(); }));
}

void test_value_containers() {
    std::vector<std::uint8_t> raw = {1, 2, 3};
    pbt::Value b = pbt::Value::bytes(raw);
    ISO_CHECK(b.as_bytes() == raw);
    ISO_CHECK_EQ_UINT(b.size(), 3);
    ISO_CHECK(pbt::Value::bytes(nullptr, 0).as_bytes().empty());
    ISO_CHECK(pbt::Value::bytes(nullptr, 2).as_bytes() == (std::vector<std::uint8_t>{0, 0}));

    std::vector<pbt::Value> items;
    items.push_back(pbt::Value::integer(1));
    items.push_back(pbt::Value::boolean(true));
    items.push_back(pbt::Value::bytes(raw));
    pbt::Value l = pbt::Value::list(std::move(items));
    ISO_CHECK(l.kind() == pbt::Kind::List);
    ISO_CHECK_EQ_UINT(l.size(), 3);
    ISO_CHECK_EQ_INT(l.at(0).as_i64(), 1);
    ISO_CHECK(l.at(1).as_bool());
    ISO_CHECK(l.at(2).as_bytes() == raw);
    ISO_CHECK(throws<pbt::Error>([&] { (void)l.at(3); }));
    ISO_CHECK(throws<pbt::Error>([&] { (void)b.at(0); }));
    ISO_CHECK(throws<pbt::Error>([&] { (void)pbt::Value::integer(1).size(); }));
    std::vector<pbt::Value> back = l.items();
    ISO_CHECK_EQ_UINT(back.size(), 3);
    ISO_CHECK(back[0] == pbt::Value::integer(1));

    pbt::Value t = pbt::Value::tuple({pbt::Value::integer(1), pbt::Value::integer(2)});
    ISO_CHECK(t.kind() == pbt::Kind::Tuple);
    ISO_CHECK(t != l);
    ISO_CHECK(pbt::Value::list({}).size() == 0);
    ISO_CHECK(l.canonical() == "[1,true,0x010203]");
}

void test_value_copy_move_equality() {
    pbt::Value a = pbt::Value::list({pbt::Value::integer(1), pbt::Value::integer(2)});
    pbt::Value copy = a;  // deep clone
    ISO_CHECK(copy == a);
    pbt::Value moved = std::move(copy);
    ISO_CHECK(moved == a);
    ISO_CHECK(copy.kind() == pbt::Kind::Nil);  // a moved-from value is nil and freeable
    pbt::Value assigned;
    assigned = a;
    ISO_CHECK(assigned == a);
    assigned = pbt::Value::integer(9);
    ISO_CHECK(assigned != a);
    pbt::Value& alias = assigned;
    assigned = alias;  // self-assignment is safe
    ISO_CHECK_EQ_INT(assigned.as_i64(), 9);
    // Kind-sensitive equality: the int 1 is not the bool true.
    ISO_CHECK(pbt::Value::integer(1) != pbt::Value::boolean(true));
    // A raw C value can be adopted and released.
    pbt_val raw;
    pbt_val_nil(&raw);
    pbt_val_set_i64(&raw, 77);
    pbt::Value adopted = pbt::Value::adopt(raw);
    ISO_CHECK(raw.kind == PBT_V_NIL);
    ISO_CHECK_EQ_INT(adopted.as_i64(), 77);
    pbt_val out = adopted.release();
    ISO_CHECK(adopted.kind() == pbt::Kind::Nil);
    pbt::Value from_c = pbt::Value::from_c(out);
    pbt_val_free(&out);
    ISO_CHECK_EQ_INT(from_c.as_i64(), 77);
    pbt::ValueView view = from_c;  // implicit window
    ISO_CHECK_EQ_INT(view.as_i64(), 77);
}

// ---- Rng and pure helpers -----------------------------------------------------------------
void test_rng_and_helpers() {
    pbt::Rng a(0), b(0), c(1);
    std::uint64_t first = a.next();
    ISO_CHECK(first == 16294208416658607535ull);  // the corpus' u64_seed0 first draw
    ISO_CHECK(b.next() == first);
    ISO_CHECK(c.next() != first);
    pbt::Rng d(5);
    for (int i = 0; i < 50; ++i) ISO_CHECK(d.bounded(7) < 7);
    ISO_CHECK(pbt::fnv1a64("") == 0xcbf29ce484222325ull);
    ISO_CHECK(pbt::sub_seed(1, "a") == pbt::sub_seed(1, "a"));
    ISO_CHECK(pbt::sub_seed(1, "a") != pbt::sub_seed(1, "b"));
    ISO_CHECK(pbt::is_failure_id("property-panic"));
    ISO_CHECK(!pbt::is_failure_id("property-panic "));
    ISO_CHECK(!pbt::is_failure_id(std::string_view("property-panic\0x", 16)));
    ISO_CHECK(!pbt::is_failure_id(""));
    ISO_CHECK(pbt::tfv_valid("tfv 1\nrec R1 Kop S706274\n"));
    std::string why;
    ISO_CHECK(!pbt::tfv_valid("bad header\n", &why));
    ISO_CHECK(!why.empty());
    ISO_CHECK(!pbt::ci_from_value(nullptr));
    ISO_CHECK(!pbt::ci_from_value("false"));
    ISO_CHECK(!pbt::ci_from_value(" 0 "));
    ISO_CHECK(pbt::ci_from_value("true"));
    ISO_CHECK(pbt::ci_from_value("1"));
}

// ---- generators ---------------------------------------------------------------------------
void test_primitive_generators() {
    pbt::Rng rng(11);
    for (int i = 0; i < 40; ++i) {
        std::int64_t x = pbt::gen::integer(-5, 5).sample(rng).as_i64();
        ISO_CHECK(x >= -5 && x <= 5);
        ISO_CHECK(pbt::gen::u8().sample(rng).as_i64() <= 255);
        ISO_CHECK(pbt::gen::boolean().sample(rng).kind() == pbt::Kind::Bool);
    }
    pbt::Value wide = pbt::gen::integer("-1000000000000000000000", "1000000000000000000000").sample(rng);
    ISO_CHECK(wide.decimal().size() >= 1);
    ISO_CHECK(pbt::gen::i128().sample(rng).kind() == pbt::Kind::Int);
    ISO_CHECK(pbt::gen::u64().sample(rng).kind() == pbt::Kind::Int);
    pbt::Value bytes = pbt::gen::bytes(2, 4).sample(rng);
    ISO_CHECK(bytes.size() >= 2 && bytes.size() <= 4);
    ISO_CHECK(pbt::gen::constant(pbt::Value::integer(3)).sample(rng).as_i64() == 3);
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::integer(5, 1); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::integer("1", "zz"); }));
}

void test_combinators() {
    pbt::Rng rng(3);
    pbt::Gen evens = pbt::gen::integer(0, 100).map([](pbt::ValueView v) { return pbt::Value::integer(v.as_i64() * 2); });
    for (int i = 0; i < 30; ++i) ISO_CHECK(evens.sample(rng).as_i64() % 2 == 0);

    pbt::Gen odd_only = pbt::gen::integer(0, 100).filter([](pbt::ValueView v) { return v.as_i64() % 2 == 1; });
    for (int i = 0; i < 30; ++i) ISO_CHECK(odd_only.sample(rng).as_i64() % 2 == 1);

    // bind: the length decides the range of the second stage.
    pbt::Gen bound = pbt::gen::integer(1, 3).bind([](pbt::ValueView n) {
        return pbt::gen::list(pbt::gen::integer(0, 9), static_cast<std::size_t>(n.as_i64()), static_cast<std::size_t>(n.as_i64()));
    });
    for (int i = 0; i < 20; ++i) {
        pbt::Value l = bound.sample(rng);
        ISO_CHECK(l.size() >= 1 && l.size() <= 3);
    }

    pbt::Gen either = pbt::gen::one_of({pbt::gen::constant(pbt::Value::integer(1)), pbt::gen::constant(pbt::Value::integer(2))});
    bool saw1 = false, saw2 = false;
    for (int i = 0; i < 60; ++i) {
        std::int64_t x = either.sample(rng).as_i64();
        saw1 = saw1 || x == 1;
        saw2 = saw2 || x == 2;
        ISO_CHECK(x == 1 || x == 2);
    }
    ISO_CHECK(saw1 && saw2);

    pbt::Gen freq = pbt::gen::frequency({{1000000, pbt::gen::constant(pbt::Value::integer(7))},
                                         {1, pbt::gen::constant(pbt::Value::integer(8))}});
    int sevens = 0;
    for (int i = 0; i < 50; ++i) sevens += freq.sample(rng).as_i64() == 7;
    ISO_CHECK(sevens >= 45);

    pbt::Gen pair = pbt::gen::tuple({pbt::gen::boolean(), pbt::gen::u8()});
    pbt::Value p = pair.sample(rng);
    ISO_CHECK(p.kind() == pbt::Kind::Tuple);
    ISO_CHECK_EQ_UINT(p.size(), 2);

    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::one_of({}); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::frequency({}); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::tuple(std::vector<pbt::Gen>(9, pbt::gen::boolean())); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::list(pbt::gen::boolean(), 3, 1); }));
}

// A generator must keep its closures alive after the scope that built them is gone.
pbt::Gen make_anchored(std::shared_ptr<int> witness) {
    int base = 1000;
    return pbt::gen::integer(0, 9)
        .map([base, witness](pbt::ValueView v) { return pbt::Value::integer(base + v.as_i64()); })
        .filter([witness](pbt::ValueView v) { return v.as_i64() >= 1000; });
}

void test_anchors_outlive_the_builder() {
    std::shared_ptr<int> witness = std::make_shared<int>(1);
    {
        pbt::Gen g = make_anchored(witness);
        ISO_CHECK(witness.use_count() > 1);  // the closures hold it
        pbt::Gen copy = g;                   // a copy keeps them too
        pbt::Rng rng(1);
        ISO_CHECK(copy.sample(rng).as_i64() >= 1000);
        pbt::Gen moved = std::move(g);
        ISO_CHECK(moved.sample(rng).as_i64() >= 1000);
        // composing keeps the children's closures alive
        pbt::Gen listed = pbt::gen::list(moved, 1, 3);
        pbt::Gen outer = listed;
        ISO_CHECK(outer.sample(rng).size() >= 1);
    }
    ISO_CHECK_EQ_INT(witness.use_count(), 1);  // and releases them when the last copy dies
}

void test_custom_generator() {
    int draws = 0;
    pbt::Gen g = pbt::Gen::custom([&draws](pbt::Run& run) {
        ++draws;
        pbt::Value inner = run.sample(pbt::gen::integer(0, 9));
        return pbt::Value::integer(inner.as_i64() + static_cast<std::int64_t>(run.bounded(1)));
    });
    pbt::Rng rng(2);
    for (int i = 0; i < 10; ++i) ISO_CHECK(g.sample(rng).as_i64() <= 9);
    ISO_CHECK_EQ_INT(draws, 10);
}

void test_gen_sample_and_shrink() {
    pbt::Gen g = pbt::gen::integer(0, 1000);
    pbt::Value v = pbt::Value::integer(937);
    int steps = g.shrink(v, [](pbt::ValueView x) { return x.as_i64() > 100; });
    ISO_CHECK_EQ_INT(v.as_i64(), 101);
    ISO_CHECK(steps > 0);
    // map drops shrinking: the value is returned untouched with zero steps.
    pbt::Gen mapped = g.map([](pbt::ValueView x) { return pbt::Value::integer(x.as_i64()); });
    pbt::Value w = pbt::Value::integer(937);
    ISO_CHECK_EQ_INT(mapped.shrink(w, [](pbt::ValueView x) { return x.as_i64() > 100; }), 0);
    ISO_CHECK_EQ_INT(w.as_i64(), 937);
    // An exception in the predicate surfaces to the caller after the C call returns.
    pbt::Value z = pbt::Value::integer(500);
    ISO_CHECK(throws<std::runtime_error>([&] {
        g.shrink(z, [](pbt::ValueView) -> bool { throw std::runtime_error("boom"); });
    }));
}

// ---- running properties -----------------------------------------------------------------------
void test_passing_property() {
    pbt::Verdict v = pbt::for_all(pbt::gen::integer(0, 10), [](pbt::ValueView x) { return x.as_i64() <= 10; }, 1);
    ISO_CHECK(v.passed());
    ISO_CHECK(static_cast<bool>(v));
    ISO_CHECK_EQ_INT(v.cases_run(), 100);
    ISO_CHECK(v.failure_id().empty());
    ISO_CHECK_EQ_INT(v.status(), 0);
    v.assert_passed();
    // A void property holds unless it throws.
    pbt::Verdict w = pbt::check_property(1, pbt::gen::boolean(), [](pbt::ValueView) {}, unredacted());
    ISO_CHECK(w.passed());
}

void test_failing_property_unredacted() {
    pbt::Gen g = pbt::gen::integer(0, 1000);
    pbt::Verdict v = pbt::check_property(7, g, [](pbt::ValueView x) { return x.as_i64() <= 100; }, unredacted());
    ISO_CHECK(!v.passed());
    ISO_CHECK(v.failure_id() == PBT_FAIL_COUNTEREXAMPLE);
    ISO_CHECK(v.seed().has_value() && *v.seed() == 7);
    ISO_CHECK(v.counterexample().has_value() && v.counterexample()->as_i64() > 100);
    ISO_CHECK(v.minimal().has_value() && v.minimal()->as_i64() == 101);
    ISO_CHECK(v.shrink_steps() > 0);
    ISO_CHECK(v.iteration() >= 1);
    ISO_CHECK(!v.message().empty());
    ISO_CHECK(throws<pbt::PropertyFailed>([&] { v.assert_passed(); }));
    // Replay: the same seed gives the same verdict.
    pbt::Verdict again = pbt::check_property(7, g, [](pbt::ValueView x) { return x.as_i64() <= 100; }, unredacted());
    ISO_CHECK(again.counterexample() == v.counterexample());
    ISO_CHECK(again.iteration() == v.iteration());
}

void test_redaction_fails_closed() {
    pbt::Gen g = pbt::gen::integer(0, 1000);
    auto prop = [](pbt::ValueView x) { return x.as_i64() <= 100; };
    // The default (Unset) and On both redact: no seed and no witness.
    pbt::Options on;
    on.ci = pbt::CiMode::On;
    for (const pbt::Options& o : {pbt::Options(), on}) {
        pbt::Verdict v = pbt::check_property(7, g, prop, o);
        ISO_CHECK(!v.passed());
        ISO_CHECK(v.failure_id() == PBT_FAIL_COUNTEREXAMPLE);
        ISO_CHECK(!v.seed().has_value());
        ISO_CHECK(!v.counterexample().has_value());
        ISO_CHECK(!v.minimal().has_value());
        ISO_CHECK_EQ_UINT(v.digest().size(), 16);
        ISO_CHECK(v.message().find("101") == std::string::npos);  // the witness is not in the text
        ISO_CHECK(v.message().find("seed=") == std::string::npos);
    }
}

void test_verdict_honesty() {
    pbt::Gen g = pbt::gen::boolean();
    auto holds = [](pbt::ValueView) { return true; };
    pbt::Options zero = unredacted();
    zero.num_tests = 0;  // a property that never ran proves nothing
    pbt::Verdict v = pbt::check_property(1, g, holds, zero);
    ISO_CHECK(!v.passed());
    ISO_CHECK(v.failure_id() == PBT_FAIL_EXHAUSTED);
    pbt::Options bad = unredacted();
    bad.num_tests = -1;
    pbt::Verdict b = pbt::check_property(1, g, holds, bad);
    ISO_CHECK(!b.passed());
    ISO_CHECK_EQ_INT(b.status(), PBT_E_ARG);
    pbt::Options huge = unredacted();
    huge.max_shrink_steps = PBT_MAX_SHRINK_STEPS + 1;
    ISO_CHECK(!pbt::check_property(1, g, holds, huge).passed());
    // A property that reports an error by throwing is a panic verdict.
    pbt::Verdict p = pbt::check_property(1, g, [](pbt::ValueView) -> bool { throw std::runtime_error("x"); }, unredacted());
    ISO_CHECK(!p.passed());
    ISO_CHECK(p.failure_id() == PBT_FAIL_PANIC);
    // A filter nothing satisfies is exhausted.
    pbt::Verdict e = pbt::check_property(1, g.filter([](pbt::ValueView) { return false; }, 5), holds, unredacted());
    ISO_CHECK(e.failure_id() == PBT_FAIL_EXHAUSTED);
}

void test_nested_check_uses_the_enclosing_scope() {
    pbt::Gen outer = pbt::gen::integer(0, 5);
    pbt::Gen inner = pbt::gen::integer(0, 5);
    int inner_runs = 0;
    pbt::Options small = unredacted();
    small.num_tests = 5;
    pbt::Verdict v = pbt::check_property(
        3, outer,
        [&](pbt::ValueView x, pbt::Scope scope) {
            pbt::Options io = unredacted();
            io.num_tests = 3;
            pbt::Verdict iv = pbt::check_property(scope, 9, inner, [&](pbt::ValueView y) { return y.as_i64() <= 5; }, io);
            ++inner_runs;
            return iv.passed() && x.as_i64() <= 5;
        },
        small);
    ISO_CHECK(v.passed());
    ISO_CHECK_EQ_INT(inner_runs, 5);
}

void test_custom_shrinker_override() {
    pbt::Gen g = pbt::gen::integer(0, 1000);
    // The shrinker jumps straight to 7; the engine re-verifies it and adopts it.
    pbt::ShrinkFn jump = [](pbt::Value& value, const pbt::Falsifies& still_fails, int) {
        pbt::Value cand = pbt::Value::integer(7);
        if (still_fails(cand)) {
            value = cand;
            return 1;
        }
        return 0;
    };
    pbt::Verdict v = pbt::check_property(5, g, [](pbt::ValueView x) { return x.as_i64() < 0; }, unredacted(), jump);
    ISO_CHECK(v.minimal().has_value());
    ISO_CHECK_EQ_INT(v.minimal()->as_i64(), 7);
    ISO_CHECK_EQ_INT(v.shrink_steps(), 1);
    // A shrinker that LIES (returns a value that passes) is not believed: the original is reported.
    pbt::ShrinkFn liar = [](pbt::Value& value, const pbt::Falsifies&, int) {
        value = pbt::Value::integer(-1);
        return 3;
    };
    pbt::Verdict l = pbt::check_property(5, g, [](pbt::ValueView x) { return x.as_i64() < 0; }, unredacted(), liar);
    ISO_CHECK(l.minimal().has_value());
    ISO_CHECK(l.minimal()->as_i64() >= 0);
    ISO_CHECK_EQ_INT(l.shrink_steps(), 0);
    // A custom GENERATOR with a shrinker is used by the engine too.
    pbt::Gen cg = pbt::Gen::custom([](pbt::Run& run) { return pbt::Value::integer(static_cast<std::int64_t>(run.bounded(1000)) + 500); },
                                   jump);
    pbt::Verdict c = pbt::check_property(5, cg, [](pbt::ValueView x) { return x.as_i64() < 0; }, unredacted());
    ISO_CHECK_EQ_INT(c.minimal()->as_i64(), 7);
}

void test_standalone_shrinkers() {
    auto le100 = [](pbt::ValueView x) { return x.cmp_i64(100) <= 0; };
    pbt::Shrunk s = pbt::shrink_int(pbt::Value::integer(54321), pbt::Value::integer(0), pbt::Value::integer(0),
                                    pbt::Value::integer(100000), le100);
    ISO_CHECK_EQ_INT(s.minimal.as_i64(), 101);
    ISO_CHECK(s.steps > 0);
    ISO_CHECK(throws<pbt::NotFalsifying>([&] {
        (void)pbt::shrink_int(pbt::Value::integer(5), pbt::Value::integer(0), pbt::Value::integer(0), pbt::Value::integer(10), le100);
    }));
    ISO_CHECK(throws<pbt::Error>([&] {  // initial outside its bounds
        (void)pbt::shrink_int(pbt::Value::integer(500), pbt::Value::integer(0), pbt::Value::integer(0), pbt::Value::integer(10), le100);
    }));
    ISO_CHECK(throws<pbt::Error>([&] {  // not an int
        (void)pbt::shrink_int(pbt::Value::boolean(true), pbt::Value::integer(0), pbt::Value::integer(0), pbt::Value::integer(10), le100);
    }));
    pbt::Shrunk l = pbt::shrink_list_ints(
        pbt::Value::list({pbt::Value::integer(10), pbt::Value::integer(5), pbt::Value::integer(8), pbt::Value::integer(4)}),
        pbt::Value::integer(0), [](pbt::ValueView v) {
            std::int64_t sum = 0;
            for (std::size_t i = 0; i < v.size(); ++i) sum += v.at(i).as_i64();
            return sum <= 20;
        });
    ISO_CHECK(l.minimal.kind() == pbt::Kind::List);
    ISO_CHECK(l.steps > 0);
    ISO_CHECK(throws<pbt::Error>([] {
        (void)pbt::shrink_list_ints(pbt::Value::list({pbt::Value::boolean(true)}), pbt::Value::integer(0), [](pbt::ValueView) { return false; });
    }));
    pbt::Shrunk b = pbt::shrink_bytes(pbt::Value::bytes(std::vector<std::uint8_t>{1, 2, 3, 4, 5, 6, 7, 8}),
                                      [](pbt::ValueView v) { return v.size() < 5; });
    ISO_CHECK_EQ_UINT(b.minimal.size(), 5);
    ISO_CHECK(throws<pbt::Error>([&] { (void)pbt::shrink_bytes(pbt::Value::integer(1), [](pbt::ValueView) { return false; }); }));
    pbt::Replay r = pbt::simulate_replay(42, le100);
    ISO_CHECK_EQ_INT(r.initial_fail.as_i64(), 413);
    ISO_CHECK_EQ_INT(r.minimal.as_i64(), 101);
}

// This TU is compiled WITHOUT CA_PBT_TEST_SEAMS: production has no seam global and the
// helpers are the identity (VER05j section 6).
#ifdef CA_PBT_TEST_SEAMS
#error "test_api must exercise the production configuration"
#endif
void test_production_has_no_seams() {
    ISO_CHECK(pbt::detail::seeded(5) == 5);
    ISO_CHECK_EQ_INT(pbt::detail::stepped(3), 3);
    ISO_CHECK(!pbt::detail::bounds_swapped());
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::gen::integer(5, 1); }));  // bounds are never swapped
}

// ---- views are call-scoped: the compiler refuses the dangling forms ----------------------------------
template <class V, class = void>
struct can_at : std::false_type {};
template <class V>
struct can_at<V, std::void_t<decltype(std::declval<V>().at(0))>> : std::true_type {};

static_assert(std::is_convertible_v<const pbt::Value&, pbt::ValueView>, "a named Value is viewable");
static_assert(!std::is_convertible_v<pbt::Value&&, pbt::ValueView>, "a temporary Value must not become a view");
static_assert(can_at<pbt::Value&>::value, "at() on a named Value");
static_assert(!can_at<pbt::Value>::value, "at() on a temporary Value must not compile");
static_assert(can_at<pbt::ValueView>::value, "chained views are fine: the view points into the parent");

// ---- direct sampling is bounded (stacked rare filters) ---------------------------------------------------
void test_direct_sampling_is_bounded() {
    // Four stacked filters, each accepting 1 in 100 of what the one below accepted: ~1e8 draws per value
    // if nothing charged them.  The direct-sample budget ends the draw as property-exhausted instead.
    unsigned long calls = 0;
    pbt::Gen g = pbt::gen::u64().filter(
        [&calls](pbt::ValueView v) {
            ++calls;
            return v.to_u64_wrapping() % 100 == 0;
        },
        PBT_MAX_FILTER_RETRIES);
    for (std::uint64_t level = 1, div = 100; level < 4; ++level, div *= 100)
        g = g.filter([div](pbt::ValueView v) { return (v.to_u64_wrapping() / div) % 100 == 0; }, PBT_MAX_FILTER_RETRIES);
    pbt::Rng rng(1);
    int status = 0;
    try {
        (void)g.sample(rng);
    } catch (const pbt::StatusError& e) {
        status = e.status();
    }
    ISO_CHECK_EQ_INT(status, PBT_E_EXHAUSTED);
    ISO_CHECK(calls <= pbt::detail::kDirectSampleWork);
    ISO_CHECK(calls > 1000);  // it did try
    // An ordinary rare filter still succeeds, and every call gets a FRESH budget.
    pbt::Gen easy = pbt::gen::u8().filter([](pbt::ValueView v) { return v.as_i64() % 10 == 0; });
    for (int i = 0; i < 200; ++i) ISO_CHECK(easy.sample(rng).as_i64() % 10 == 0);
}

// ---- sub_seed ----------------------------------------------------------------------------------------------
void test_sub_seed_rejects_embedded_nul() {
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::sub_seed(1, std::string("a\0b", 3)); }));
    ISO_CHECK(throws<pbt::Error>([] { (void)pbt::sub_seed(1, std::string("\0", 1)); }));
    ISO_CHECK(pbt::sub_seed(1, "ab") != pbt::sub_seed(1, "a"));
    int status = 0;
    try {
        (void)pbt::sub_seed(1, std::string("x\0y", 3));
    } catch (const pbt::Error& e) {
        status = e.status();
    }
    ISO_CHECK_EQ_INT(status, PBT_E_ARG);
}

// ---- a shrinker's reported step count is untrusted --------------------------------------------------------
void test_shrinker_step_counts_are_validated() {
    const int bad_counts[] = {-1, 1001, 2147483647};  // negative, above max_steps (1000), INT_MAX
    for (int n : bad_counts) {
        pbt::Gen g = pbt::Gen::custom([](pbt::Run&) { return pbt::Value::integer(5); },
                                      [n](pbt::Value&, const pbt::Falsifies&, int) { return n; });
        pbt::Value v = pbt::Value::integer(5);
        int status = 0;
        try {
            (void)g.shrink(v, [](pbt::ValueView) { return true; });
        } catch (const pbt::StatusError& e) {
            status = e.status();
        }
        ISO_CHECK_EQ_INT(status, PBT_E_ARG);
        // In a check the lie costs the shrink: the original witness is reported with zero steps.
        pbt::ShrinkFn liar = [n](pbt::Value& value, const pbt::Falsifies&, int) {
            value = pbt::Value::integer(7);
            return n;
        };
        pbt::Verdict r = pbt::check_property(5, pbt::gen::integer(0, 1000), [](pbt::ValueView x) { return x.as_i64() < 0; },
                                             unredacted(), liar);
        ISO_CHECK(r.minimal().has_value() && *r.minimal() == *r.counterexample());
        ISO_CHECK_EQ_INT(r.shrink_steps(), 0);
    }
    // An honest count within the budget is accepted and added.
    pbt::Gen ok = pbt::Gen::custom([](pbt::Run&) { return pbt::Value::integer(5); },
                                   [](pbt::Value&, const pbt::Falsifies&, int) { return 3; });
    pbt::Value v = pbt::Value::integer(5);
    ISO_CHECK_EQ_INT(ok.shrink(v, [](pbt::ValueView) { return true; }), 3);
    // Exactly max_steps is the largest honest count.
    pbt::Gen edge = pbt::Gen::custom([](pbt::Run&) { return pbt::Value::integer(5); },
                                     [](pbt::Value&, const pbt::Falsifies&, int max_steps) { return max_steps; });
    ISO_CHECK_EQ_INT(edge.shrink(v, [](pbt::ValueView) { return true; }, 17), 17);
}

}  // namespace

int main() {
    test_value_scalars();
    test_value_wide_integers();
    test_value_containers();
    test_value_copy_move_equality();
    test_rng_and_helpers();
    test_primitive_generators();
    test_combinators();
    test_anchors_outlive_the_builder();
    test_custom_generator();
    test_gen_sample_and_shrink();
    test_passing_property();
    test_failing_property_unredacted();
    test_redaction_fails_closed();
    test_verdict_honesty();
    test_nested_check_uses_the_enclosing_scope();
    test_custom_shrinker_override();
    test_standalone_shrinkers();
    test_production_has_no_seams();
    test_direct_sampling_is_bounded();
    test_sub_seed_rejects_embedded_nul();
    test_shrinker_step_counts_are_validated();
    return ISO_TEST_RESULT();
}
