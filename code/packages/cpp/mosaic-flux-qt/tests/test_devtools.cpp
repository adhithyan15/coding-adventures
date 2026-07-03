#include <vector>

#include "MosaicFlux/DevTools.h"
#include "MosaicFlux/Store.h"
#include "test_harness.h"

struct DtState {
    int v = 0;
    bool operator==(const DtState& o) const { return v == o.v; }
};

struct DtBump final : MosaicFlux::MosaicAction<DtState> {
    DtState apply(const DtState& s) const override { return DtState{s.v + 1}; }
};

MOSAIC_TEST(devtools_callable) {
    auto m = MosaicFlux::devToolsMiddleware<DtState>();
    m(DtBump{}, DtState{0}, DtState{1});
}

MOSAIC_TEST(devtools_custom_store_name) {
    auto m = MosaicFlux::devToolsMiddleware<DtState>("my-grid");
    m(DtBump{}, DtState{0}, DtState{1});
}

MOSAIC_TEST(devtools_integrates_with_store) {
    int probeRuns = 0;
    std::vector<MosaicFlux::Middleware<DtState>> mws;
    mws.push_back(MosaicFlux::devToolsMiddleware<DtState>());
    mws.push_back([&](const MosaicFlux::MosaicAction<DtState>&,
                      const DtState&,
                      const DtState&) { probeRuns++; });
    MosaicFlux::MosaicStore<DtState> store(DtState{}, std::move(mws));
    store.dispatch(DtBump{});
    store.dispatch(DtBump{});
    MOSAIC_ASSERT_EQ(probeRuns, 2);
    MOSAIC_ASSERT_EQ(store.state().v, 2);
}

MOSAIC_MAIN()
