# Changelog

All notable changes to `Mosaic.Flux` are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-06-01

### Added

- `IMosaicAction<TState>` interface: command-pattern action with an
  `Apply(state) → state` method.
- `MosaicStore<TState>` dispatcher that holds state, runs reducers via
  `action.Apply`, fires `INotifyPropertyChanged` for the XAML binding
  pipeline, and supports custom middleware.
- `MosaicStore.Subscribe<TSlice>` with selector + equality comparator
  (default: `Object.ReferenceEquals` falling through to `Equals`),
  returning an `IDisposable` unsubscribe handle.
- `MosaicStore.Select<TSlice>` one-shot projection.
- `Middleware<TState>` delegate plus `MiddlewareHelpers.Compose` and
  `MiddlewareHelpers.Logger<TState>()`.
- `Selector.Create` memoised helpers for 1-, 2-, and 3-input slice
  projections, mirroring the shape used by Reselect and the other
  Mosaic runtimes.
- `DevTools.Create<TState>(string storeName = "default")` middleware
  factory — no-op locally, ready to be wired to a WinUI debug channel.
- 29 xUnit tests covering action `Apply` semantics, dispatch +
  subscribe + unsubscribe, no-op short-circuit, `PropertyChanged`
  emission, middleware composition + error isolation, selector
  memoisation, and dev-tools integration.

### Notes

- Targets `net9.0` to match the rest of the repo's CI toolchain.
- `TreatWarningsAsErrors=true` and `Nullable=enable` are on for the
  library project so any reference-nullability or analyzer warning
  fails the build.
