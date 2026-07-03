#include <string>

#include "MosaicFlux/Selector.h"
#include "test_harness.h"

struct SelState {
    int a = 0;
    int b = 0;
    std::string label = "";
};

MOSAIC_TEST(selector_single_input_recomputes_on_change) {
    int calls = 0;
    auto doubled = MosaicFlux::createSelector1<SelState, int, int>(
        [](const SelState& s) { return s.a; },
        [&](const int& a) {
            calls++;
            return a * 2;
        });
    MOSAIC_ASSERT_EQ(doubled(SelState{5, 0, ""}), 10);
    MOSAIC_ASSERT_EQ(doubled(SelState{7, 0, ""}), 14);
    MOSAIC_ASSERT_EQ(calls, 2);
}

MOSAIC_TEST(selector_single_input_caches_on_stable) {
    int calls = 0;
    auto doubled = MosaicFlux::createSelector1<SelState, int, int>(
        [](const SelState& s) { return s.a; },
        [&](const int& a) {
            calls++;
            return a * 2;
        });
    SelState s{5, 0, ""};
    doubled(s);
    doubled(s);
    doubled(s);
    MOSAIC_ASSERT_EQ(calls, 1);
}

MOSAIC_TEST(selector_single_input_caches_across_state_refs) {
    int calls = 0;
    auto doubled = MosaicFlux::createSelector1<SelState, int, int>(
        [](const SelState& s) { return s.a; },
        [&](const int& a) {
            calls++;
            return a * 2;
        });
    doubled(SelState{5, 0, ""});
    doubled(SelState{5, 999, "different"});
    MOSAIC_ASSERT_EQ(calls, 1);
}

MOSAIC_TEST(selector_two_input_recomputes_when_either_changes) {
    int calls = 0;
    auto sum = MosaicFlux::createSelector2<SelState, int, int, int>(
        [](const SelState& s) { return s.a; },
        [](const SelState& s) { return s.b; },
        [&](const int& a, const int& b) {
            calls++;
            return a + b;
        });
    MOSAIC_ASSERT_EQ(sum(SelState{1, 2, ""}), 3);
    MOSAIC_ASSERT_EQ(sum(SelState{1, 5, ""}), 6);
    MOSAIC_ASSERT_EQ(sum(SelState{4, 5, ""}), 9);
    MOSAIC_ASSERT_EQ(calls, 3);
}

MOSAIC_TEST(selector_two_input_caches_on_stable_inputs) {
    int calls = 0;
    auto sum = MosaicFlux::createSelector2<SelState, int, int, int>(
        [](const SelState& s) { return s.a; },
        [](const SelState& s) { return s.b; },
        [&](const int& a, const int& b) {
            calls++;
            return a + b;
        });
    SelState s{1, 2, ""};
    sum(s);
    sum(s);
    MOSAIC_ASSERT_EQ(calls, 1);
}

MOSAIC_TEST(selector_three_input_recomputes_when_any_changes) {
    int calls = 0;
    auto fmt =
        MosaicFlux::createSelector3<SelState, int, int, std::string, std::string>(
            [](const SelState& s) { return s.a; },
            [](const SelState& s) { return s.b; },
            [](const SelState& s) { return s.label; },
            [&](const int& a, const int& b, const std::string& lbl) {
                calls++;
                return lbl + ":" + std::to_string(a + b);
            });
    MOSAIC_ASSERT_EQ(fmt(SelState{1, 2, "x"}), std::string("x:3"));
    MOSAIC_ASSERT_EQ(fmt(SelState{1, 2, "x"}), std::string("x:3"));
    MOSAIC_ASSERT_EQ(fmt(SelState{1, 2, "y"}), std::string("y:3"));
    MOSAIC_ASSERT_EQ(calls, 2);
}

MOSAIC_MAIN()
