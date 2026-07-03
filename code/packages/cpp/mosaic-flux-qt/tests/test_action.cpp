#include "MosaicFlux/Action.h"
#include "test_harness.h"

struct ActionState {
    int count;
    bool operator==(const ActionState& o) const { return count == o.count; }
};

struct Increment final : MosaicFlux::MosaicAction<ActionState> {
    ActionState apply(const ActionState& s) const override {
        return ActionState{s.count + 1};
    }
};

struct Add final : MosaicFlux::MosaicAction<ActionState> {
    int amount;
    explicit Add(int a) : amount(a) {}
    ActionState apply(const ActionState& s) const override {
        return ActionState{s.count + amount};
    }
};

MOSAIC_TEST(action_apply_returns_next_without_mutating_input) {
    ActionState initial{5};
    Increment inc;
    auto next = inc.apply(initial);
    MOSAIC_ASSERT_EQ(next.count, 6);
    MOSAIC_ASSERT_EQ(initial.count, 5);
}

MOSAIC_TEST(action_payload_accessible) {
    Add add(7);
    MOSAIC_ASSERT_EQ(add.amount, 7);
    auto next = add.apply(ActionState{3});
    MOSAIC_ASSERT_EQ(next.count, 10);
}

MOSAIC_TEST(action_deterministic) {
    ActionState state{0};
    Add add(5);
    MOSAIC_ASSERT(add.apply(state) == add.apply(state));
}

MOSAIC_MAIN()
