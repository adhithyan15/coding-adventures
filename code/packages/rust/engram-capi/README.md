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
`eg_engram_app_props` mirrors `EngramSession::engram_app_props()` and returns
flat, Mosaic-slot-shaped JSON for the shared `EngramApp` surface.
`eg_handle_engram_app_event` lets native Mosaic shells forward generated
`EngramApp` events such as `onReveal`, `reveal`, `onGood`, `onUndo`,
`onBuryCard`, `onSuspendCard`, or `onToggleMark` into the shared Rust review
flow; it also accepts browser row events such as
`onBrowserToggleMarkSelected|card-id`. Host-owned actions such as
`onImportAnki`, `onExportAnki`, and `onBrowserOpenSelected` return a
`hostIntent` object so SwiftUI, XAML, Qt, and Electron shells can perform
native file dialogs or viewer navigation without forking Engram's business
logic. `onBrowserEditSelected` hydrates the shared Mosaic note editor through
props instead, and `onAddNote` starts the same shared note editor draft rather
than asking each native shell to own a custom add-note modal. Event handling
returns updated state and refreshed Mosaic props.
`eg_review_history` mirrors `EngramSession::review_history()` for native stats
views that need deck-scoped review-log summaries over a timestamp range.
`eg_daily_limit_usage` and `eg_build_queue_with_daily_limits` expose the shared
daily limit accounting and queue builder for native review screens.
`eg_export_anki_basic_tsv` and `eg_parse_anki_basic_tsv` expose the shared
Anki-compatible Basic front/back text path for native file-open/save flows.
`eg_parse_anki_apkg` previews a legacy/V11 APKG import as Engram state JSON.
`eg_import_anki_apkg` replaces the session with that imported state, while
`eg_merge_anki_apkg` upserts imported decks, note models, notes, cards,
progress, review history, sources, and media into the current session. These
byte-slice APIs support legacy `collection.anki2` / `collection.anki21`
packages and modern `collection.anki21b` envelopes by decoding Anki's `meta`
protobuf plus zstd-compressed collection/media payloads before the shared
SQLite import path runs.
Native shells can also call the package-neutral `eg_parse_anki_package`,
`eg_import_anki_package`, `eg_merge_anki_package`,
`eg_inspect_anki_package`, and `eg_read_anki_package_media` aliases when they
accept both `.apkg` and `.colpkg` file extensions.
When an imported media payload conflicts with an existing Engram media ID or
archive name, merge keeps both payloads by assigning deterministic `-merge-N`
names; ID remaps also retarget the imported media provenance record to the new
ID.
`eg_export_anki_apkg` writes the current session as a deterministic legacy/V11
APKG. `eg_export_anki_apkg_modern` writes the same state in a modern
`collection.anki21b` envelope with zstd-compressed collection/media payloads.
Both return package bytes as a JSON byte array under `apkg`, keeping the native
ABI string-shaped while target-specific shells decide how to save or share the
bytes.
`eg_export_anki_package` and `eg_export_anki_package_modern` are neutral aliases
for those export paths.
`eg_inspect_anki_apkg` returns collection/media manifest JSON, and
`eg_read_anki_apkg_media` reads one archived media payload by archive name for
native import flows that need to copy audio or images alongside imported cards.
`eg_analyze_media_references` scans the current Engram state for Anki
`[sound:...]`, local HTML `src`/`poster`/`data`/`srcset` references, and CSS
`url(...)` references, returning referenced filenames, matched media asset IDs,
missing filenames, and unreferenced asset IDs.
`eg_materialized_cards` mirrors the JSON facade's durable generated-card
materialization for native note/template editors.
Native shells can send review-control commands such as `buryCardSiblings`
through `eg_dispatch`, using the same JSON contract as web and Mosaic hosts.
The same dispatch path accepts shared media commands such as
`upsertMediaAsset`, `deleteMediaAsset`, and `deleteMediaAssets`, so native
file-open/editor flows can copy, replace, or prune media assets without
duplicating Engram state mutation logic.
Native settings screens can also dispatch `setDeckOptions` with a camelCase
`DeckOptions` object to update the durable scheduler options used by queues and
reviews.

## Build

```bash
cargo test -p engram-capi
```
