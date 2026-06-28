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

engram-anki-package  APKG/V11 SQLite import bridge for native file-open flows
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
`eg_export_anki_basic_tsv` and `eg_parse_anki_basic_tsv` expose the shared
Anki-compatible Basic front/back text path for native file-open/save flows.
`eg_parse_anki_apkg` previews a legacy/V11 APKG import as Engram state JSON,
and `eg_import_anki_apkg` applies that imported state to the session. These
byte-slice APIs currently support `collection.anki2` and `collection.anki21`
packages and return JSON errors for modern `collection.anki21b` packages until
the modern Anki package reader lands.
`eg_inspect_anki_apkg` returns collection/media manifest JSON, and
`eg_read_anki_apkg_media` reads one archived media payload by archive name for
native import flows that need to copy audio or images alongside imported cards.
`eg_materialized_cards` mirrors the JSON facade's durable generated-card
materialization for native note/template editors.
Native shells can send review-control commands such as `buryCardSiblings`
through `eg_dispatch`, using the same JSON contract as web and Mosaic hosts.

## Build

```bash
cargo test -p engram-capi
```
