# engram-wasm

`engram-wasm` is the zero-dependency `extern "C"` and linear-memory ABI over
`engram-core-wasm`. It follows the same repo convention as
`spreadsheet-wasm`: no `wasm-bindgen`, no wasm-pack, and no browser APIs in
Rust.

The crate owns only the host boundary:

```text
Mosaic React/Electron host
        |
        v
engram-wasm                 extern "C" + linear-memory ABI
        |
        v
engram-core-wasm            JSON facade
        |
        v
engram-core                 scheduling, cards, queues, snapshots
```

## Memory Protocol

Strings cross the WASM boundary as `(ptr, len)` UTF-8 inputs and packed output
buffers:

- JS calls `alloc(len)`, writes UTF-8 bytes, passes `(ptr, len)`, then frees the
  input buffer with `dealloc(ptr, len)`.
- Rust string outputs are returned as `[len: u32 little-endian][utf8 bytes]`;
  JS reads the payload and frees the whole buffer with `dealloc(ptr, 4 + len)`.

## Exports

Core session exports include `reset`, `snapshot`, `get_state`, `load_snapshot`,
`dispatch`, `export_anki_apkg`, `merge_anki_apkg`, `build_queue`,
`get_deck_stats`, `session_progress`, `review_history`, `search_cards`,
`engram_app_props`, `engram_browser_props`, and `handle_engram_app_event`.
In browser WASM builds, APKG import/export returns an explicit native-host
delegation error because real package I/O is handled by platform bridges such
as Electron's `engram-host-cli` sidecar.

The JS loader in `js/engram-mosaic-host-wasm.mjs` adapts those JSON responses to
the generated Mosaic React/Electron host contract by converting Mosaic
kebab-case slot names such as `app-title` to generated prop names such as
`appTitle`. Generated app events that require platform work, such as browser
open/edit, Anki package import/export, or note dialogs, also return a
`hostIntent` object and may be observed through
`createMosaicHost({ onHostIntent })`. `installEngramMosaicHost(window, ...)`
dispatches `mosaic-host-ready` after installing `window.mosaicHost`, allowing
generated React/Electron renderers to refresh after asynchronous host setup.

## Building

PowerShell:

```powershell
cd code/packages/rust/engram-wasm
.\build-wasm.ps1
node js/smoke.mjs
```

Bash:

```bash
cd code/packages/rust/engram-wasm
bash build-wasm.sh
node js/smoke.mjs
```

The Rust tests exercise the ABI on the host target, so CI can validate the
marshalling protocol without requiring a WASM toolchain:

```bash
cargo test -p engram-wasm
```
