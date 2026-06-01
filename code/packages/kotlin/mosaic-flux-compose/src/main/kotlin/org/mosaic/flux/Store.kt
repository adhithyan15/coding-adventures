// Store.kt — MosaicStore: state container + dispatcher.
//
// The MosaicStore is the runtime's center of gravity.  It holds the
// current state, accepts action dispatches, runs middleware, and
// exposes the state as a kotlinx.coroutines.flow.StateFlow<S> for
// fine-grained Compose `collectAsState` integration.
//
// Design choices (per UI33-rewrite §6):
//
//   1. No central reducer.  dispatch() calls action.apply(state)
//      directly — Command Pattern.
//
//   2. Fine-grained subscription via StateFlow.  Compose hosts use
//      `store.stateFlow.collectAsState()` to re-compose only when
//      the observed slice changes; non-Compose hosts use the
//      imperative subscribe(selector, equality, callback) variant.
//
//   3. Synchronous dispatch.  apply() runs immediately on the
//      caller's thread; subscribers fire; middleware runs — all
//      synchronous.  Async work belongs in middleware that
//      schedules subsequent dispatches.

package org.mosaic.flux

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.concurrent.atomic.AtomicInteger

/**
 * Equality function for fine-grained subscription decisions.
 */
typealias Equality<T> = (T, T) -> Boolean

private val defaultEquality: Equality<Any?> = { a, b -> a === b }

/**
 * The Mosaic state container and dispatcher.
 *
 * @param initialState The state at construction time.
 * @param middleware Optional middleware that runs after each dispatch.
 */
class MosaicStore<S>(
    initialState: S,
    middleware: List<Middleware<S>> = emptyList(),
) {
    private val _state = MutableStateFlow(initialState)
    private val middlewareFn: Middleware<S> = composeMiddleware(middleware)
    private val subscriptions = mutableMapOf<Int, InternalSubscription<S, *>>()
    private val nextSubscriptionId = AtomicInteger(0)

    /**
     * StateFlow surface for Compose-aware consumers.  Use with
     * `collectAsState()` for automatic recomposition on state change.
     */
    val stateFlow: StateFlow<S> = _state.asStateFlow()

    /**
     * Current state, read synchronously.
     */
    val state: S get() = _state.value

    /**
     * Dispatch an action.  Runs action.apply(state), swaps state,
     * notifies subscribers whose projected slice changed, runs
     * middleware.
     */
    fun <A : MosaicAction<S>> dispatch(action: A) {
        val prev = _state.value
        val next = action.apply(prev)
        if (prev === next) {
            // No-op transform; middleware still runs so loggers see
            // every dispatch.
            middlewareFn(action, prev, next)
            return
        }
        _state.value = next
        // Snapshot subscriptions so a callback that unsubscribes
        // doesn't perturb iteration.
        val snapshot = subscriptions.values.toList()
        for (sub in snapshot) {
            sub.notifyIfChanged(next)
        }
        middlewareFn(action, prev, next)
    }

    /**
     * Subscribe to a slice of state via a selector.  Callback fires
     * when the projected slice changes by the supplied equality
     * function (default: reference identity).
     *
     * Returns an unsubscribe function — invoke it to stop receiving
     * callbacks.
     */
    fun <T> subscribe(
        selector: (S) -> T,
        @Suppress("UNCHECKED_CAST")
        equality: Equality<T> = defaultEquality as Equality<T>,
        callback: (T) -> Unit,
    ): () -> Unit {
        val id = nextSubscriptionId.getAndIncrement()
        val sub = InternalSubscription(selector, callback, equality, selector(_state.value))
        subscriptions[id] = sub
        return { subscriptions.remove(id) }
    }

    /**
     * One-shot read of a slice without subscription.
     */
    fun <T> select(selector: (S) -> T): T = selector(_state.value)
}

private class InternalSubscription<S, T>(
    private val selector: (S) -> T,
    private val callback: (T) -> Unit,
    private val equality: Equality<T>,
    initial: T,
) {
    private var lastValue: T = initial

    fun notifyIfChanged(nextState: S) {
        val nextValue = selector(nextState)
        if (!equality(lastValue, nextValue)) {
            lastValue = nextValue
            callback(nextValue)
        }
    }
}
