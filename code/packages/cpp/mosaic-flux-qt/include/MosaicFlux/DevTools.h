// DevTools.h — dev-tools middleware factory.
//
// v0.1.0 ships a no-op middleware so the surface is in place; the
// real Qt-side wire-up (local socket / shared memory channel) lands
// in v0.2.0.

#ifndef MOSAIC_FLUX_DEVTOOLS_H
#define MOSAIC_FLUX_DEVTOOLS_H

#include <string>
#include <utility>

#include "MosaicFlux/Middleware.h"

namespace MosaicFlux {

template <typename State>
Middleware<State> devToolsMiddleware(std::string storeName = "default") {
    return [storeName = std::move(storeName)](const MosaicAction<State>&,
                                              const State&,
                                              const State&) {
        // No-op for v0.1.0.  v0.2.0 will dispatch this through a
        // local channel back to the WinUI inspector / Qt debug UI.
        (void)storeName;
    };
}

}  // namespace MosaicFlux

#endif  // MOSAIC_FLUX_DEVTOOLS_H
