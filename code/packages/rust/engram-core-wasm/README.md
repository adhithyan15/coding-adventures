# engram-core-wasm

`engram-core-wasm` is the string-in / JSON-out facade over
[`engram-core`](../engram-core). It is intentionally a normal Rust library so
it can be tested without a WASM toolchain.

This crate is the shared host contract for every Engram shell:

```text
HTML / React / Electron / XAML / SwiftUI macOS+iOS / Qt
        |
        v
engram-wasm / engram-capi / platform loader
        |
        v
engram-core-wasm        string-in / JSON-out facade
        |
        v
engram-core             notes, cards, scheduling, queues, sessions
```

Target shells may own packaging and platform APIs, but they should not
reimplement scheduling, card generation, queueing, stats, or state transitions.

## API

`EngramSession` owns an in-memory `AppState` and exposes:

- `snapshot()`
- `load_snapshot(json)`
- `dispatch(command_json)`
- `build_queue(deck_id, now)`
- `deck_stats(deck_id, now)`
- `generated_cards(note_type_id, note_id)`

All JSON uses camelCase field names to match the existing TypeScript Engram app
and keep generated bindings idiomatic.
