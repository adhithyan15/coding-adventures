// Middleware.kt — cross-cutting concern hook.
//
// Middleware sees every dispatched (action, prevState, nextState)
// triple AFTER apply() has produced the next state.  Use for
// loggers, analytics, persistence, and effect schedulers.
//
// Errors thrown by middleware are caught and printed; subsequent
// middleware still run (matches the TS / Swift runtimes — one bad
// middleware can't take down the others).

package org.mosaic.flux

typealias Middleware<S> = (MosaicAction<S>, S, S) -> Unit

/**
 * Combine an array of middleware into a single middleware.  Each
 * runs in registration order.  Errors thrown by one are caught and
 * printed; subsequent middleware still run.  Returns a no-op when
 * the list is empty.
 */
fun <S> composeMiddleware(middleware: List<Middleware<S>>): Middleware<S> {
    if (middleware.isEmpty()) return { _, _, _ -> /* no-op */ }
    if (middleware.size == 1) return middleware[0]
    return { action, prev, next ->
        for (m in middleware) {
            try {
                m(action, prev, next)
            } catch (t: Throwable) {
                // Match the TS / Swift runtimes' behaviour: log and
                // continue, so one bad middleware can't break peers.
                System.err.println("[mosaic-flux] middleware threw: ${t.message}")
            }
        }
    }
}

/**
 * Dev logger middleware — prints action class name on each dispatch.
 * Production hosts typically compose their own logger that ships to
 * telemetry rather than println.
 */
fun <S> loggerMiddleware(): Middleware<S> = { action, _, _ ->
    println("[mosaic-flux] ${action::class.simpleName ?: "Action"}")
}
