#include <string>
#include <vector>

#include "MosaicFlux/Store.h"
#include "test_harness.h"

struct StoreState {
    int count = 0;
    std::string label = "";
    bool operator==(const StoreState& o) const {
        return count == o.count && label == o.label;
    }
};

struct StIncrement final : MosaicFlux::MosaicAction<StoreState> {
    StoreState apply(const StoreState& s) const override {
        return StoreState{s.count + 1, s.label};
    }
};

struct StSetLabel final : MosaicFlux::MosaicAction<StoreState> {
    std::string label;
    explicit StSetLabel(std::string l) : label(std::move(l)) {}
    StoreState apply(const StoreState& s) const override {
        return StoreState{s.count, label};
    }
};

struct StNoOp final : MosaicFlux::MosaicAction<StoreState> {
    StoreState apply(const StoreState& s) const override { return s; }
};

MOSAIC_TEST(store_starts_at_initial_state) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    MOSAIC_ASSERT_EQ(store.state().count, 0);
    MOSAIC_ASSERT_EQ(store.state().label, std::string(""));
}

MOSAIC_TEST(store_dispatch_applies_action) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    store.dispatch(StIncrement{});
    MOSAIC_ASSERT_EQ(store.state().count, 1);
}

MOSAIC_TEST(store_payloaded_action_works) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    store.dispatch(StSetLabel("hi"));
    MOSAIC_ASSERT_EQ(store.state().label, std::string("hi"));
}

MOSAIC_TEST(store_select_returns_projection) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{5, ""});
    MOSAIC_ASSERT_EQ(store.select([](const StoreState& s) { return s.count; }), 5);
}

MOSAIC_TEST(store_subscribe_fires_on_changed_slice) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    std::vector<int> received;
    auto sub = store.subscribe<int>(
        [](const StoreState& s) { return s.count; },
        [&](const int& c) { received.push_back(c); });
    store.dispatch(StIncrement{});
    MOSAIC_ASSERT_EQ(received.size(), static_cast<size_t>(1));
    MOSAIC_ASSERT_EQ(received[0], 1);
}

MOSAIC_TEST(store_subscribe_silent_on_unrelated_change) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    std::vector<int> received;
    auto sub = store.subscribe<int>(
        [](const StoreState& s) { return s.count; },
        [&](const int& c) { received.push_back(c); });
    store.dispatch(StSetLabel("x"));
    MOSAIC_ASSERT_EQ(received.size(), static_cast<size_t>(0));
}

MOSAIC_TEST(store_unsubscribe_stops_notifications) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    std::vector<int> received;
    {
        auto sub = store.subscribe<int>(
            [](const StoreState& s) { return s.count; },
            [&](const int& c) { received.push_back(c); });
        store.dispatch(StIncrement{});
    }  // sub destructor unsubscribes
    store.dispatch(StIncrement{});
    MOSAIC_ASSERT_EQ(received.size(), static_cast<size_t>(1));
}

MOSAIC_TEST(store_explicit_unsubscribe_is_idempotent) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    auto sub = store.subscribe<int>(
        [](const StoreState& s) { return s.count; },
        [](const int&) {});
    sub.unsubscribe();
    sub.unsubscribe();  // must not crash
}

MOSAIC_TEST(store_noop_skips_subscribers_but_runs_middleware) {
    int middlewareCalls = 0;
    std::vector<MosaicFlux::Middleware<StoreState>> mws;
    mws.push_back([&](const MosaicFlux::MosaicAction<StoreState>&,
                      const StoreState&,
                      const StoreState&) { middlewareCalls++; });
    MosaicFlux::MosaicStore<StoreState> store(StoreState{}, std::move(mws));
    int subscriberCalls = 0;
    auto sub = store.subscribe<int>(
        [](const StoreState& s) { return s.count; },
        [&](const int&) { subscriberCalls++; });
    store.dispatch(StNoOp{});
    MOSAIC_ASSERT_EQ(subscriberCalls, 0);
    MOSAIC_ASSERT_EQ(middlewareCalls, 1);
}

MOSAIC_TEST(store_middleware_sees_triple) {
    int prevSeen = -1;
    int nextSeen = -1;
    std::vector<MosaicFlux::Middleware<StoreState>> mws;
    mws.push_back([&](const MosaicFlux::MosaicAction<StoreState>&,
                      const StoreState& prev,
                      const StoreState& next) {
        prevSeen = prev.count;
        nextSeen = next.count;
    });
    MosaicFlux::MosaicStore<StoreState> store(StoreState{}, std::move(mws));
    store.dispatch(StIncrement{});
    MOSAIC_ASSERT_EQ(prevSeen, 0);
    MOSAIC_ASSERT_EQ(nextSeen, 1);
}

MOSAIC_TEST(store_nested_dispatch_throws) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    auto sub = store.subscribe<int>(
        [](const StoreState& s) { return s.count; },
        [&](const int&) {
            // Reentrant dispatch from a subscriber must throw, not
            // deadlock.
            store.dispatch(StIncrement{});
        });
    bool threw = false;
    try {
        store.dispatch(StIncrement{});
    } catch (const std::logic_error&) {
        threw = true;
    }
    MOSAIC_ASSERT(threw);
}

MOSAIC_TEST(store_custom_equality_respected) {
    MosaicFlux::MosaicStore<StoreState> store(StoreState{});
    std::vector<int> received;
    auto sub = store.subscribeWithEquality<int>(
        [](const StoreState& s) { return s.count; },
        [&](const int& c) { received.push_back(c); },
        [](const int&, const int&) { return true; });  // "always equal"
    store.dispatch(StIncrement{});
    MOSAIC_ASSERT_EQ(received.size(), static_cast<size_t>(0));
}

MOSAIC_MAIN()
