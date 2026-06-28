# engram-capi

`engram-capi` is a stable C ABI over [`engram-core-wasm`](../engram-core-wasm)
for native Engram shells.

```text
Qt / C++ / SwiftUI / Flutter / XAML / Compose
        |
        v
engram-capi          opaque handle + UTF-8 C strings
        |
        v
engram-core-wasm     JSON facade
        |
        v
engram-core          study model, scheduling, search, import/export
```

## API

The public C declarations live in [`include/engram.h`](include/engram.h).

Every function that returns `char *` returns a heap-allocated, NUL-terminated
UTF-8 string. Callers must release it with `eg_string_free`, not `free`.
A null return signals a null session handle or an interior-NUL allocation error.

The JSON strings are the same values returned by `engram-core-wasm`, so native
and web shells share one command and result contract.
`eg_session_progress` mirrors `EngramSession::session_progress()` for native
review screens that need total/current/remaining/correct counters.
`eg_review_history` mirrors `EngramSession::review_history()` for native stats
views that need deck-scoped review-log summaries over a timestamp range.
`eg_daily_limit_usage` and `eg_build_queue_with_daily_limits` expose the shared
daily limit accounting and queue builder for native review screens.
Native shells can send review-control commands such as `buryCardSiblings`
through `eg_dispatch`, using the same JSON contract as web and Mosaic hosts.

## Build

```bash
cargo test -p engram-capi
```
