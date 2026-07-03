// Store.swift — MosaicStore: state container + dispatcher.
//
// The MosaicStore is the runtime's center of gravity.  It holds the
// current state, accepts action dispatches, runs middleware, and
// notifies subscribers when state changes.
//
// Design choices (per UI33-rewrite §6):
//
//   1. No central reducer.  `dispatch(_:)` calls `action.apply(to:)`
//      directly — the action is its own reducer fragment (Command
//      Pattern).
//
//   2. Fine-grained subscription.  `subscribe(selector:callback:)`
//      only invokes the callback when the projected slice changes.
//
//   3. Synchronous dispatch.  Action's `apply` runs immediately;
//      state swaps; subscribers fire — all on the calling thread.
//      Async work belongs in middleware.
//
//   4. Per-instance store.  No singletons.  Multi-store apps
//      instantiate multiple stores.
//
// SwiftUI integration is intentionally NOT in this v0.1.0 release.
// The store works fine with @Bindable / @Observable wrappers that
// adapters can add in v0.2.0; for now, hosts use `subscribe(...)`
// imperatively and update @State / @Published bindings themselves.

import Foundation

/// A callback fired when a subscribed slice of state changes.
public typealias MosaicSubscriber<T> = (T) -> Void

/// Default equality: reference identity for class types, `false`
/// for value types (value-typed callers must pass an explicit
/// equality function — typically `==` from `Equatable`, or a deep-
/// compare for collections).
///
/// We can't constrain T to `Equatable` in the public surface
/// because some selectors project non-Equatable values (e.g.,
/// closure references).  Reference identity is the only sensible
/// default that works for any T.
///
/// `@usableFromInline` lets the default argument reference this
/// helper from outside this file's compilation scope (Swift
/// requires the default value's body to be visible at call sites).
@usableFromInline
internal func mosaicDefaultEquality<T>(_ a: T, _ b: T) -> Bool {
    if let aObj = a as AnyObject?, let bObj = b as AnyObject? {
        return aObj === bObj
    }
    return false
}

/// The Mosaic state container and dispatcher.
///
/// Generic over the state type.  Action types are determined per
/// dispatch call (any type conforming to `MosaicAction` whose
/// associated `State` matches this store's State).
public final class MosaicStore<State> {
    private var _state: State
    private let middlewareFn: Middleware<State>
    private var subscriptions: [SubscriptionToken: AnySubscription<State>] = [:]
    private var nextSubscriptionId: Int = 0

    /// Initialize a store with an initial state and optional
    /// middleware list.
    public init(initialState: State, middleware: [Middleware<State>] = []) {
        self._state = initialState
        self.middlewareFn = composeMiddleware(middleware)
    }

    /// The current state.  Read-only from the consumer's perspective;
    /// mutations only happen via `dispatch(_:)`.
    public var state: State { _state }

    /// Dispatch an action.  Runs `action.apply(to: state)`, swaps
    /// state, notifies subscribers whose projected slice changed,
    /// then runs middleware.
    ///
    /// Middleware sees the dispatch even when the apply is a no-op
    /// (i.e., produces a state that equals the previous one by class
    /// identity).  This is so loggers see every dispatch even
    /// "useless" ones.
    public func dispatch<A: MosaicAction>(_ action: A) where A.State == State {
        let prev = _state
        let next = action.apply(to: prev)
        let isNoOp = MosaicStore.isReferenceIdentical(prev, next)
        if !isNoOp {
            _state = next
            // Snapshot subscriptions so a callback that unsubscribes
            // (itself or a peer) doesn't perturb iteration.
            let snapshot = Array(subscriptions.values)
            for sub in snapshot {
                sub.notifyIfChanged(nextState: next)
            }
        }
        middlewareFn(action, prev, next)
    }

    /// Subscribe to a slice of state via a selector.  The callback
    /// fires when the slice changes by the supplied equality
    /// function (default: reference identity).
    ///
    /// Returns an `Unsubscribe` closure — invoke it to stop
    /// receiving callbacks.
    @discardableResult
    public func subscribe<T>(
        selector: @escaping (State) -> T,
        equality: @escaping (T, T) -> Bool = mosaicDefaultEquality,
        callback: @escaping MosaicSubscriber<T>
    ) -> () -> Void {
        let id = SubscriptionToken(id: nextSubscriptionId)
        nextSubscriptionId += 1
        let sub = TypedSubscription<State, T>(
            selector: selector,
            callback: callback,
            equality: equality,
            initial: selector(_state)
        )
        subscriptions[id] = AnySubscription(sub)
        return { [weak self] in
            self?.subscriptions.removeValue(forKey: id)
        }
    }

    /// One-shot read of a slice without subscribing.
    public func select<T>(_ selector: (State) -> T) -> T {
        selector(_state)
    }

    /// Reference-identity check that works for class types AND value
    /// types (value types are always considered "different" so the
    /// store treats every value-state dispatch as a change).  This
    /// matches the TS runtime's `Object.is` default semantics.
    private static func isReferenceIdentical(_ a: State, _ b: State) -> Bool {
        if let aObj = a as AnyObject?, let bObj = b as AnyObject? {
            return aObj === bObj
        }
        return false
    }
}

// MARK: - Subscription internals

/// Opaque key for the subscription dictionary.
private struct SubscriptionToken: Hashable {
    let id: Int
}

/// Erased subscription holder so the store can keep
/// subscriptions with different T parameters in one collection.
private struct AnySubscription<State> {
    private let _notifyIfChanged: (State) -> Void

    init<T>(_ typed: TypedSubscription<State, T>) {
        var typed = typed  // capture as mutable so notifyIfChanged
                            // can update its last-seen value
        self._notifyIfChanged = { nextState in
            typed.notifyIfChanged(nextState: nextState)
        }
    }

    func notifyIfChanged(nextState: State) {
        _notifyIfChanged(nextState)
    }
}

/// Per-subscription bookkeeping (current selector value, equality
/// function, callback).
private struct TypedSubscription<State, T> {
    let selector: (State) -> T
    let callback: MosaicSubscriber<T>
    let equality: (T, T) -> Bool
    var lastValue: T

    init(
        selector: @escaping (State) -> T,
        callback: @escaping MosaicSubscriber<T>,
        equality: @escaping (T, T) -> Bool,
        initial: T
    ) {
        self.selector = selector
        self.callback = callback
        self.equality = equality
        self.lastValue = initial
    }

    mutating func notifyIfChanged(nextState: State) {
        let nextValue = selector(nextState)
        if !equality(lastValue, nextValue) {
            lastValue = nextValue
            callback(nextValue)
        }
    }
}
