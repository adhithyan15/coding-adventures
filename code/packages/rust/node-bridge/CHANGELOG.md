# Changelog

All notable changes to the node-bridge crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-05-18

### Added — Buffer helpers

Phase 2b of [MX07](../../../specs/MX07-matrix-rust-napi.md) needs to
exchange tensor bytes with JavaScript via `Buffer` objects, and so does
every future binding that ever has to move raw bytes through the napi
boundary (image addons, crypto addons, compressed-stream addons, etc).
This release adds the helper set:

* `napi_create_buffer` / `napi_create_buffer_copy` /
  `napi_get_buffer_info` / `napi_is_buffer` — raw `extern "C"`
  declarations.
* `buffer_to_js(env, &[u8]) -> napi_value` — allocate a fresh Buffer
  containing a copy of the bytes.
* `buffer_from_js(env, napi_value) -> Option<Vec<u8>>` — extract a
  Buffer's bytes into an **owned** `Vec<u8>`, copying immediately to
  avoid use-after-detach if the underlying ArrayBuffer is transferred
  later.  Returns `None` for non-Buffer values (no panic; caller
  throws a precise JS error).
* `is_buffer(env, napi_value) -> bool` — convenience wrapper around
  the raw `napi_is_buffer`.
* `vec_buf_to_js(env, &[Vec<u8>]) -> napi_value` —
  `Array<Buffer>` builder for output tensor lists.
* `vec_buf_from_js(env, napi_value) -> Option<Vec<Vec<u8>>>` —
  `Array<Buffer>` reader for input tensor lists.

All helpers follow the **copy-in / copy-out** discipline.  Reasons:

* **Detachment safety.**  Per the N-API contract, a JS Buffer's
  underlying ArrayBuffer can be detached or transferred to a Worker
  at any later point.  Holding a raw pointer past that moment is
  undefined behaviour.  Copying immediately into a Rust `Vec<u8>`
  eliminates the entire class of use-after-detach bugs.
* **Lifetime independence.**  Outputs are fresh Buffers owned by the
  napi env, decoupled from any Rust storage that produced them.
* **Simplicity.**  No `napi_wrap`, no finalizer plumbing, no lifetime
  parameters in the helper signatures — just bytes in, bytes out.

For the rare performance-critical case where copies matter, callers
can drop down to the raw `napi_create_buffer` / `napi_get_buffer_info`
externs and manage the lifetimes by hand.  But the safe wrappers are
the default — and the default is "safe and obvious".

No new dependencies.  No change to existing exports.  font-parser-node
continues to compile unchanged (it already re-declares
`napi_get_buffer_info` locally, which the linker resolves harmlessly
to the same dynamic symbol).

## [0.1.0] - Unreleased

### Added
- String conversion (`str_to_js`, `str_from_js`)
- Array conversion (`vec_str_to_js`, `vec_str_from_js`, `vec_vec_str_to_js`, `vec_tuple2_str_to_js`)
- Boolean and number conversion
- Argument parsing (`get_cb_info`)
- Data wrapping for Rust structs (`wrap_data`, `unwrap_data`)
- Class definition (`define_class`, `method_property`)
- Standalone function creation (`create_function`) — wraps `napi_create_function` (N-API v1) to create JS function values not attached to any class; needed for module-level exports
- Error handling (`throw_error`)
- Constants (`undefined`, `null`)
