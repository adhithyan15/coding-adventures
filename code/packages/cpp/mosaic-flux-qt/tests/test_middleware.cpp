#include <string>
#include <vector>

#include "MosaicFlux/Middleware.h"
#include "test_harness.h"

struct MwState {
    int v = 0;
    bool operator==(const MwState& o) const { return v == o.v; }
};

struct MwBump final : MosaicFlux::MosaicAction<MwState> {
    MwState apply(const MwState& s) const override { return MwState{s.v + 1}; }
};

MOSAIC_TEST(middleware_empty_compose_is_noop) {
    auto m = MosaicFlux::composeMiddleware<MwState>({});
    m(MwBump{}, MwState{0}, MwState{1});  // must not throw
}

MOSAIC_TEST(middleware_single_returned_verbatim_callable) {
    bool ran = false;
    MosaicFlux::Middleware<MwState> single =
        [&](const MosaicFlux::MosaicAction<MwState>&,
            const MwState&,
            const MwState&) { ran = true; };
    auto composed = MosaicFlux::composeMiddleware<MwState>({single});
    composed(MwBump{}, MwState{0}, MwState{1});
    MOSAIC_ASSERT(ran);
}

MOSAIC_TEST(middleware_runs_in_order) {
    std::vector<std::string> calls;
    auto composed = MosaicFlux::composeMiddleware<MwState>(
        {[&](const MosaicFlux::MosaicAction<MwState>&,
             const MwState&,
             const MwState&) { calls.emplace_back("a"); },
         [&](const MosaicFlux::MosaicAction<MwState>&,
             const MwState&,
             const MwState&) { calls.emplace_back("b"); },
         [&](const MosaicFlux::MosaicAction<MwState>&,
             const MwState&,
             const MwState&) { calls.emplace_back("c"); }});
    composed(MwBump{}, MwState{0}, MwState{1});
    MOSAIC_ASSERT_EQ(calls.size(), static_cast<size_t>(3));
    MOSAIC_ASSERT_EQ(calls[0], std::string("a"));
    MOSAIC_ASSERT_EQ(calls[1], std::string("b"));
    MOSAIC_ASSERT_EQ(calls[2], std::string("c"));
}

MOSAIC_TEST(middleware_isolates_throws) {
    std::vector<std::string> calls;
    auto composed = MosaicFlux::composeMiddleware<MwState>(
        {[&](const MosaicFlux::MosaicAction<MwState>&,
             const MwState&,
             const MwState&) { calls.emplace_back("a"); },
         [](const MosaicFlux::MosaicAction<MwState>&,
            const MwState&,
            const MwState&) { throw std::runtime_error("boom"); },
         [&](const MosaicFlux::MosaicAction<MwState>&,
             const MwState&,
             const MwState&) { calls.emplace_back("c"); }});
    composed(MwBump{}, MwState{0}, MwState{1});
    MOSAIC_ASSERT_EQ(calls.size(), static_cast<size_t>(2));
    MOSAIC_ASSERT_EQ(calls[0], std::string("a"));
    MOSAIC_ASSERT_EQ(calls[1], std::string("c"));
}

MOSAIC_TEST(middleware_logger_does_not_throw) {
    auto m = MosaicFlux::loggerMiddleware<MwState>();
    m(MwBump{}, MwState{0}, MwState{1});
}

MOSAIC_MAIN()
