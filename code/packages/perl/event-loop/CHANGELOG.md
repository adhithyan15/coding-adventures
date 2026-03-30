# Changelog — CodingAdventures::EventLoop (Perl)

## [0.01] — 2026-03-29

### Added
- `new()` constructor with empty handler registry and tick state.
- `on($event, $cb)` — persistent event handler registration.
- `once($event, $cb)` — one-shot handler (auto-removed after first fire).
- `off($event [, $cb])` — remove all or specific handler.
- `emit($event, $data)` — fire handlers with snapshot iteration.
- `on_tick($cb)` — tick handler registration.
- `tick([$dt])` — single time step with delta_time.
- `run([$n, $dt])` — multi-tick runner.
- `step([$dt])` — convenience alias.
- `elapsed_time` and `tick_count` tracking.
- Comprehensive test suite: 25+ subtests covering all methods and edge cases.
