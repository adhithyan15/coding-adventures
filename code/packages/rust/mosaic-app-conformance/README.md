# mosaic-app-conformance

Reusable Rust application library for executing Mosaic host bindings against
the real `mosaic-app-capi` ABI. The fixture starts with `count = 0`, accepts an
`increment` semantic event, returns revisioned props, and supports snapshots.

Native backend acceptance harnesses load the built dynamic library through the
same generated binding shipped to applications. This keeps conformance focused
on lifecycle, event sequencing, JSON projection, buffer ownership, and teardown
without introducing an app-specific platform reducer.

The adjacent `package/` directory is a minimal MIL/MLL/MSL UI whose required
`platform` and `status` props come from this Rust engine. Packaging acceptance
uses the pair to prove a strict generated native application can carry and load
its engine without application-owned platform glue or a global library install.

Under protocol 2, `requestEffect` emits an awaited `conformance.counter` effect
(or Notify with `{notify: true}`). Complete it with `{ok: {amount: 7}}` to add
to the counter; optional `chain: true` requests another Await. Cancellation and
failure retain the count. Invalid amounts reject completion without changing state.
IDs survive same-instance restore and are not persisted into a fresh instance.
Rust tests call the real seventh C ABI symbol and free every returned buffer;
the adjacent WASM package tests the same fixture as compiled WebAssembly.
