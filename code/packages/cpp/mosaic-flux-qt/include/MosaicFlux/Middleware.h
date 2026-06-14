// Middleware.h — dispatch hook for logging / dev-tools / async glue.
//
// Middleware in Mosaic is intentionally simpler than Redux's
// thunk-style middleware: it does NOT wrap dispatch.  Instead it's
// invoked AFTER state has been swapped, with (action, prevState,
// nextState).  This is enough for logging, dev-tools, persistence,
// and post-hoc analytics — the things users actually want — without
// the cognitive load of curried `next` chains.

#ifndef MOSAIC_FLUX_MIDDLEWARE_H
#define MOSAIC_FLUX_MIDDLEWARE_H

#include <functional>
#include <iostream>
#include <string>
#include <vector>

#include "MosaicFlux/Action.h"

namespace MosaicFlux {

// A middleware is a callable invoked once per dispatch with the
// action plus the (prev, next) state pair.
template <typename State>
using Middleware =
    std::function<void(const MosaicAction<State>&, const State&, const State&)>;

// Compose a list of middlewares into one.  Each one runs in order;
// exceptions in one don't prevent later ones from running, so a
// busted dev-tools middleware can't break logging.
template <typename State>
Middleware<State> composeMiddleware(std::vector<Middleware<State>> mws) {
    if (mws.empty()) {
        return [](const MosaicAction<State>&, const State&, const State&) {};
    }
    if (mws.size() == 1) {
        return mws.front();
    }
    return [mws = std::move(mws)](const MosaicAction<State>& a,
                                  const State& prev,
                                  const State& next) {
        for (const auto& mw : mws) {
            try {
                mw(a, prev, next);
            } catch (...) {
                // Intentional: one bad middleware must not break
                // the rest of the chain.  Errors get swallowed here;
                // dev-tools middleware can surface them elsewhere.
            }
        }
    };
}

// A trivial logger: writes the action's typeid name to stderr.  Not
// fancy, but enough to verify the wiring works.
template <typename State>
Middleware<State> loggerMiddleware() {
    return [](const MosaicAction<State>& a, const State&, const State&) {
        std::cerr << "[MosaicFlux] dispatched: " << typeid(a).name() << '\n';
    };
}

}  // namespace MosaicFlux

#endif  // MOSAIC_FLUX_MIDDLEWARE_H
