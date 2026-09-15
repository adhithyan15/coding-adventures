---
category: Native extensions & FFI
---

# `napi_create_threadsafe_function`

: pass C `NULL` (`ptr::null_mut()`), not `napi_get_undefined()`, for `async_resource` — Node v25 checks `IsObject()` and JS undefined isn't an Object → `napi_invalid_arg`. When using a custom `call_js_cb`, also pass `func = NULL` and carry the JS function via the `context` pointer as an `napi_ref`.
