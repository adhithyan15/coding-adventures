// store.dart — MosaicStore: state container + dispatcher.
//
// The MosaicStore is the runtime's center of gravity. It holds the
// current state, accepts action dispatches, runs middleware, and
// notifies subscribers when state changes.
//
// Design choices (per UI33-rewrite §6):
//
//   1. No central reducer. dispatch() calls action.apply(state)
//      directly — Command Pattern.
//
//   2. Fine-grained subscription. subscribe(selector, equality,
//      callback) fires only when the projected slice changes.
//
//   3. Synchronous dispatch. apply() runs immediately on the
//      caller's thread; subscribers fire; middleware runs.
//
//   4. ChangeNotifier integration is NOT a hard dependency here.
//      We expose a lightweight onChange() Stream-equivalent that
//      Flutter's MosaicBuilder widget (in builder.dart) wraps.
//      Non-Flutter consumers can use subscribe() directly without
//      pulling in Flutter.

import 'action.dart';
import 'middleware.dart';

/// Equality function for fine-grained subscriptions.
typedef Equality<T> = bool Function(T a, T b);

bool _defaultEquality<T>(T a, T b) => identical(a, b);

/// The Mosaic state container and dispatcher.
class MosaicStore<S> {
  S _state;
  final Middleware<S> _middleware;
  final Map<int, _Subscription<S, dynamic>> _subscriptions = {};
  int _nextSubscriptionId = 0;
  final List<void Function()> _onChangeListeners = [];

  MosaicStore({
    required S initialState,
    List<Middleware<S>> middleware = const [],
  })  : _state = initialState,
        _middleware = composeMiddleware(middleware);

  /// The current state. Read-only from the consumer's perspective.
  S get state => _state;

  /// Dispatch an action.
  ///
  /// Runs action.apply(state), swaps state, notifies subscribers
  /// whose projected slice changed, then runs middleware.
  void dispatch(MosaicAction<S> action) {
    final prev = _state;
    final next = action.apply(prev);
    if (identical(prev, next)) {
      // No-op transform; middleware still runs.
      _middleware(action, prev, next);
      return;
    }
    _state = next;
    // Snapshot subscriptions so a callback that unsubscribes can't
    // perturb iteration.
    final snapshot = _subscriptions.values.toList(growable: false);
    for (final sub in snapshot) {
      sub.notifyIfChanged(next);
    }
    // Bulk onChange notification for Flutter ChangeNotifier-style
    // consumers (e.g., MosaicBuilder).
    for (final listener in List<void Function()>.from(_onChangeListeners)) {
      try {
        listener();
      } catch (_) {
        // Don't let a bad listener break peers.
      }
    }
    _middleware(action, prev, next);
  }

  /// Subscribe to a slice of state via a selector.
  ///
  /// Returns an unsubscribe function.
  void Function() subscribe<T>(
    T Function(S) selector,
    void Function(T) callback, {
    Equality<T>? equality,
  }) {
    final id = _nextSubscriptionId++;
    final eq = equality ?? _defaultEquality<T>;
    _subscriptions[id] = _Subscription<S, T>(
      selector: selector,
      callback: callback,
      equality: eq,
      initial: selector(_state),
    );
    return () => _subscriptions.remove(id);
  }

  /// One-shot read without subscription.
  T select<T>(T Function(S) selector) => selector(_state);

  /// Bulk onChange listener for Flutter ChangeNotifier-style
  /// integration. Returns an unsubscribe function.
  ///
  /// Used by MosaicBuilder to trigger setState on every state
  /// change. Most direct consumers should prefer subscribe() with a
  /// selector for fine-grained control.
  void Function() addListener(void Function() listener) {
    _onChangeListeners.add(listener);
    return () => _onChangeListeners.remove(listener);
  }
}

class _Subscription<S, T> {
  final T Function(S) selector;
  final void Function(T) callback;
  final Equality<T> equality;
  T _lastValue;

  _Subscription({
    required this.selector,
    required this.callback,
    required this.equality,
    required T initial,
  }) : _lastValue = initial;

  void notifyIfChanged(S nextState) {
    final nextValue = selector(nextState);
    if (!equality(_lastValue, nextValue)) {
      _lastValue = nextValue;
      callback(nextValue);
    }
  }
}
