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
