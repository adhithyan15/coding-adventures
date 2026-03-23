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

Only `napi-sys` — raw N-API bindings. Zero abstractions on top.
