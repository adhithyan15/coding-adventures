// Action.h — strict-Flux action contract for Mosaic / Qt.
//
// Mosaic's strict-Flux contract requires that every action be
// addressable as a *class*: callers can switch on it, debuggers can
// print it, and the framework can persist it in time-travel
// histories.  In C++ that means a virtual base with an `apply`
// method returning the next state.
//
// We follow the Command Pattern literally:
//
//   struct Increment final : MosaicAction<AppState> {
//       AppState apply(const AppState& s) const override {
//           return AppState{ s.count + 1 };
//       }
//   };
//
// Actions are typically tiny, copyable values; users can stack-
// allocate them and pass by reference into `dispatch`.

#ifndef MOSAIC_FLUX_ACTION_H
#define MOSAIC_FLUX_ACTION_H

namespace MosaicFlux {

// Base class for all actions over a state type `State`.  Pure
// virtual `apply` makes the contract explicit and forces every
// action to be its own type so the framework can identify it.
template <typename State>
class MosaicAction {
public:
    virtual ~MosaicAction() = default;

    // Pure function: read prev, return next.  No side effects on
    // `state`; no I/O.  The store enforces that this is the *only*
    // way state is allowed to change.
    virtual State apply(const State& state) const = 0;
};

}  // namespace MosaicFlux

#endif  // MOSAIC_FLUX_ACTION_H
