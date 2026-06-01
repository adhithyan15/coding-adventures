// middleware.dart — cross-cutting concern hook.
//
// Middleware sees every dispatched (action, prevState, nextState)
// triple AFTER apply() produces the next state. Use for loggers,
// analytics, persistence, effect schedulers.
//
// Errors thrown by middleware are caught and printed; subsequent
// middleware still run (matches the TS / Swift / Kotlin runtimes —
// one bad middleware can't take down the others).

import 'action.dart';

typedef Middleware<S> = void Function(
  MosaicAction<S> action,
  S prevState,
  S nextState,
);

/// Combine middleware in registration order. Errors thrown by one
/// are caught and printed; subsequent middleware still run. Returns
/// a no-op when the list is empty.
Middleware<S> composeMiddleware<S>(List<Middleware<S>> middleware) {
  if (middleware.isEmpty) {
    return (_, __, ___) {};
  }
  if (middleware.length == 1) {
    return middleware[0];
  }
  return (action, prev, next) {
    for (final m in middleware) {
      try {
        m(action, prev, next);
      } catch (e, st) {
        // Match the TS / Swift / Kotlin runtimes' behaviour:
        // log via print + stderr and continue.
        // ignore: avoid_print
        print('[mosaic-flux] middleware threw: $e\n$st');
      }
    }
  };
}

/// Dev logger middleware — prints action runtime type on each
/// dispatch. Production hosts typically compose their own logger.
Middleware<S> loggerMiddleware<S>() {
  return (action, _, __) {
    // ignore: avoid_print
    print('[mosaic-flux] ${action.runtimeType}');
  };
}
