# mosaic-app-wasm

Package-independent WebAssembly transport for `MosaicApp`. The final application
invokes `mosaic_app_wasm::export_mosaic_wasm!(App, App::default())` alongside its
native C ABI export. Build with `--target wasm32-unknown-unknown`; no binding
generator or app-specific JavaScript reducer is required.

`js/mosaic-host.mjs` accepts module bytes through `loadMosaicModule(bytes)`.
Its `create(context)` returns a host with `update`, `dispatch(name, payload)`,
`snapshot()`, `restore(snapshot)` and idempotent `dispose()`. Updates preserve
the standard props, effects and announcements. Rendering, effect execution and
durable storage belong to the consuming host. Restore does not consume an event
sequence; rejected dispatches can retry the same sequence. Each create owns an
independent Rust runtime. A WASM trap invalidates the entire module; reload it
instead of attempting to reuse potentially interrupted application state.

The wire protocol uses scalar wasm32 pointers, never C structure lowering:

1. `mosaic_wasm_alloc(length)` returns an owned zeroed input buffer, or zero for
   an invalid length (maximum 64 MiB per request).
2. Write UTF-8 JSON into that allocation. `mosaic_wasm_call(pointer)` consumes
   it and returns a response allocation: a four-byte little-endian length followed
   by UTF-8 JSON `{ok:true,value:...}` or `{ok:false,error:...}`.
3. `mosaic_wasm_free(pointer)` releases response or unused input allocations.
   Unknown pointers are ignored; call rejects unknown pointers with zero.

Requests are tagged with `op`: `create` takes `context`; `dispatch` takes `handle`
and standard `event`; `snapshot` and `destroy` take `handle`; `restore` takes
`handle` and standard `snapshot`. Creation returns `{handle,update}`. Dispatch
and restore return an update; snapshot returns the opaque snapshot or null.
Handles are never reused. JavaScript refreshes memory views after calls because
Rust allocation may grow linear memory. Host code must not retain buffer views.
As with any WASM module, code with direct memory access is a trusted caller.

Validation (from the Rust workspace):

```sh
cargo test -p mosaic-app-wasm
cargo build -p mosaic-app-conformance -p visicalc-mosaic-app --target wasm32-unknown-unknown
node --test mosaic-app-wasm/js/conformance.test.mjs
```

The Node tests load compiled artifacts, exercise independent runtime instances,
failed-event retries, opaque snapshots and teardown, and replay VisiCalc's
shared presentation contract. Browser rendering acceptance belongs to the root
application integration; these tests do not establish visual or native parity.

## Effect completion

`module.create({protocolVersion: 2})` enables UI47's dedicated channel. Call
`host.completeEffect(id, {ok: payload})`, `{cancelled: {}}`, or
`{failed: {message: "..."}}`. The returned update can contain further effects.
Completions advance revision but do not consume dispatch sequence. Failed calls
retain the last successful `host.update`. Disposed hosts reject callbacks locally;
wire requests also reject destroyed handles, which are never reused.

The new wire request is `{op: "completeEffect", handle, id, result}`. Errors
preserve the existing `error` string and add `code` and `pendingEffects` fields.
JavaScript throws `MosaicHostError`; pending snapshot/restore errors have code
`"pendingEffects"` and the outstanding IDs. Defer autosave until completion;
neither snapshot nor restore silently abandons or replays a pending operation.
Actual file/clipboard/dialog execution belongs to capability handlers. Existing
create calls default to protocol 1.

## Browser file capabilities

`js/mosaic-file-effects.mjs` exports `createBrowserFileEffects(host)`. Create one
executor per protocol-2 host and call `run(effect)` directly from the originating
click/key handler, before yielding to React effects or timers. Await the returned
update and render it; recursively inspect any new effects. Requests that outlive
user activation fail explicitly and need a fresh gesture. The browser enforces
permission through its native file picker; no path or handle is accepted from app
payloads. See the [File System Access API guide](https://developer.chrome.com/docs/capabilities/web-apis/file-system-access)
for its secure-context and user-activation requirements.

| Await kind | Payload | Successful result payload |
|---|---|---|
| `file.open` | `{}` | `{name, bytes}` |
| `file.save` | `{suggestedName, bytes}` | `{name}` |

`bytes` is base64, with a 16 MiB decoded limit (below the WASM JSON request limit).
Both payloads may include `mimeType` and `extension` together, for example
`"application/json"` and `".json"`. File content remains opaque; Rust validates
and applies its own versioned format. The selected save destination is written
and closed before success is reported. Picker dismissal returns cancellation;
denial, unsupported capabilities, concurrent dialogs and I/O errors return failure.
Browsers without these picker APIs explicitly degrade; no download fallback can
claim that a file was durably saved.

Duplicate `run` calls return no update and do not repeat I/O or replay old renders.
If `completeEffect` rejects, `run` rejects and the executor retains the outcome;
`retry(id)` resubmits it without repeating I/O. Applications should accept malformed
file content as a completed operation with an error view, preserving the workbook,
rather than leave an uncorrectable payload pending forever. Notify requests are
rejected by this Await-only executor.

Call executor `dispose()` before disposing its MosaicHost. Outstanding picker/read
callbacks are ignored, and interrupted writable streams are aborted before close
when possible. Disposal cannot dismiss an OS dialog or undo a write whose close
has already started. The executor never retains a user file handle for later use.

Run `node --test mosaic-app-wasm/js/file-effects.test.mjs` after building the
conformance WASM artifact. Tests cover the browser boundary and real Rust completion
rejection/cancellation. Actual browser file-dialog acceptance and generated VisiCalc
Open/Save controls remain #14548; native migration and release acceptance remain
separate requirements.
