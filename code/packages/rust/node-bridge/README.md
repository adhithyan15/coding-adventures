# node-bridge

Thin safe Rust wrapper over Node.js N-API — no napi-rs, no macros, no magic.

## What is this?

A ~350-line crate that wraps only the raw N-API functions needed to build Node.js native addons. It replaces napi-rs with explicit, debuggable code.

## What it provides

| Category | Functions |
|----------|-----------|
| Strings | `str_to_js`, `str_from_js` |
| Arrays | `array_new`, `vec_str_to_js`, `vec_str_from_js`, `vec_vec_str_to_js` |
| Booleans | `bool_to_js` |
| Numbers | `usize_to_js` |
| Arguments | `get_cb_info` |
| Data wrapping | `wrap_data`, `unwrap_data`, `unwrap_data_mut` |
| Classes | `define_class`, `method_property`, `set_named_property` |
| Errors | `throw_error` |
| Constants | `undefined`, `null` |

## Dependencies

None. The N-API functions are declared by hand as `extern "C"` in
`src/lib.rs` — no `napi-sys`, no `napi-rs`, no bindgen.

## How the N-API symbols get linked

The `napi_*` functions live inside the Node.js process, not in any library
on disk, so each platform needs a different answer at link time:

| Platform | What makes the link succeed                                      |
|----------|------------------------------------------------------------------|
| Linux    | ELF shared objects may leave symbols undefined; `dlopen` fills them |
| macOS    | `build.rs` passes `-undefined dynamic_lookup`                     |
| Windows  | `#[link(name = "node.exe", kind = "raw-dylib")]` on the extern block |

On Windows a DLL must name the module behind every import. `raw-dylib`
lets rustc write that `node.exe` import table straight from the
declarations, which is what node-gyp's `node.lib` would otherwise supply.
The import is bound to a host named `node.exe`; renamed hosts such as
Electron would need a delay-load hook, which this crate does not provide.
