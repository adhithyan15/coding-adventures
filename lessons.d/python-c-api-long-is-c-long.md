---
category: Native extensions & FFI
---

# Python C API `long` is `c_long`

— on Windows x64, `c_long == i32`, not `i64`. Always use `std::ffi::c_long` for `PyLong_AsLong`/`PyLong_FromLong`/`PyModule_AddIntConstant`. Hardcoding `i64` fails Windows compile.
