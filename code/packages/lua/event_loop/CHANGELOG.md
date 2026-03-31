# Changelog — coding-adventures-event-loop (Lua)

## [0.1.0] — 2026-03-29

### Added
- `EventLoop.new()` — create event loop with handler registry and tick state.
- `loop:on(event, cb)` — register persistent event handler.
- `loop:once(event, cb)` — register one-shot handler (auto-removed after first fire).
- `loop:off(event [, cb])` — remove all handlers or a specific one.
- `loop:emit(event, data)` — fire all handlers for an event.
- `loop:on_tick(cb)` — register tick handler receiving delta_time.
- `loop:tick([dt])` — advance time step and fire tick handlers.
- `loop:run([n, dt])` — run n ticks.
- `loop:step([dt])` — run one tick.
- `elapsed_time` and `tick_count` tracking.
- Comprehensive test suite: 30+ tests covering all methods and interactions.
