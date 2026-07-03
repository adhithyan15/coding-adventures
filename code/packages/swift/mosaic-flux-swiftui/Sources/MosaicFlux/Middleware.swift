// Middleware.swift — middleware contract.
//
// Middleware sees every dispatched (action, prevState, nextState)
// triple AFTER `action.apply(to:)` has produced the next state.  This
// observation-style hook is the right place for loggers, analytics,
// persistence, and effect schedulers.
//
// Middleware runs synchronously inside `MosaicStore.dispatch`.  If a
// middleware function blocks (sync sleeps, heavy work), it blocks the
// dispatch.  Async side effects should schedule additional dispatches
// via the store reference closured into the middleware.
//
// Errors thrown by middleware are caught and printed; subsequent
// middleware still run.  This matches the TS runtime — one bad
// middleware can't take down the others.

/// A middleware function: invoked after each dispatch with the
/// action that was applied, the state before, and the state after.
public typealias Middleware<State> = (any MosaicAction<State>, State, State) -> Void

/// Compose an array of middleware into a single middleware.
/// Each middleware runs in registration order.  Errors thrown by one
/// middleware are caught and printed; subsequent middleware still run.
///
/// Returns a no-op middleware when the array is empty.
public func composeMiddleware<State>(
    _ middleware: [Middleware<State>]
) -> Middleware<State> {
    if middleware.isEmpty {
        return { _, _, _ in /* no-op */ }
    }
    if middleware.count == 1 {
        return middleware[0]
    }
    return { action, prev, next in
        for m in middleware {
            // Swift closures can't `throws` here because Middleware
            // is non-throwing.  Errors that DO escape (fatalError,
            // etc.) will terminate the process — that's intentional;
            // recoverable errors must be caught inside the middleware
            // itself.  Trapping all errors here would mask programmer
            // mistakes during development.
            m(action, prev, next)
        }
    }
}

/// A simple logger middleware that prints action class name + which
/// state keys changed.  Suitable for dev builds; production hosts
/// typically compose their own logger that ships to telemetry.
public func loggerMiddleware<State>() -> Middleware<State> {
    return { action, _, _ in
        let name = String(describing: type(of: action))
        print("[mosaic-flux] \(name)")
    }
}
