# Changelog

All notable changes to `mosaic-flux-qt` are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-06-02

### Added

- `MosaicAction<State>` — command-pattern action base with virtual
  `apply(state) → state`.
- `MosaicStore<State>` template — thread-safe dispatcher with
  `std::mutex`-guarded subscription registry. Iteration is done
  over a snapshot so a callback that unsubscribes during dispatch
  doesn't perturb the loop.
- `MosaicStore::Subscription` — RAII handle whose destructor
  unsubscribes; explicit `.unsubscribe()` is idempotent and safe
  under concurrent double-call via `std::atomic<bool>` exchange.
- `MosaicStore::subscribe<T>` (default operator== equality) and
  `subscribeWithEquality<T>` (custom predicate).
- `MosaicStore::select` — one-shot projection helper.
- `Middleware<State>` callable type plus `composeMiddleware` (each
  middleware runs in order; exceptions in one don't stop later
  ones) and `loggerMiddleware` (typeid-based).
- `createSelector1` / `createSelector2` / `createSelector3` —
  memoised projection helpers mirroring Reselect and the other
  Mosaic runtimes.
- `devToolsMiddleware<State>` — no-op factory; real Qt-side wire
  lands in v0.2.0.
- Thread-local reentry guard: nested dispatch from a subscriber
  callback or middleware throws `std::logic_error` instead of
  deadlocking on the dispatch mutex (matches Redux's reducer-purity
  contract; queue follow-up actions for the next tick instead).
- 29 zero-dependency unit tests covering action `apply` semantics,
  dispatch + subscribe + unsubscribe + RAII cleanup, no-op
  short-circuit, custom equality, middleware composition + error
  isolation, selector memoisation, DevTools integration, and the
  nested-dispatch guard.

### Notes

- Header-only. INTERFACE CMake target `MosaicFlux::MosaicFlux`.
- C++17 minimum. `-Wall -Wextra -Wpedantic -Werror` on the
  interface (or `/W4 /WX` on MSVC).
- No third-party deps. Tests use a tiny in-repo harness
  (`tests/test_harness.h`).
- Q_OBJECT shim for QML deferred to v0.2.0 because templates and
  `moc` don't mix.
