# Changelog

## [0.1.0] - Unreleased

### Added

- `Store<S>` class — Flux-like event-driven state store with dispatch, subscribe,
  and getState. Supports middleware for side effects (logging, persistence).
- `useStore<S>(store)` — React hook using useSyncExternalStore for correct
  concurrent-mode integration. Subscribes on mount, unsubscribes on unmount.
- Action, Reducer, Listener, Middleware types — full TypeScript definitions
  with literate-programming documentation of the Flux pattern.
- Middleware chain — first-added runs outermost. Call next() to continue,
  skip next() to swallow the action.
- 15+ unit tests: store dispatch, subscribe, unsubscribe, middleware order,
  middleware swallowing, re-entrant dispatch, React hook rendering.
