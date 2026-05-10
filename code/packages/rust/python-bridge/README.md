# python-bridge

Thin safe Rust wrapper over Python's C API — no PyO3, no macros, no magic.

## What is this?

A ~400-line crate that wraps only the raw Python C API functions needed to build native extensions. It replaces PyO3 (~50,000 lines) with explicit, debuggable code.

## What it provides

| Category | Functions |
|----------|-----------|
| Strings | `str_to_py`, `str_from_py` |
| Bytes | `bytes_to_py`, `bytes_from_py` |
| Lists | `vec_str_to_py`, `vec_str_from_py`, `vec_vec_str_to_py`, `vec_tuple2_str_to_py` |
| Sets | `set_str_to_py`, `set_str_from_py` |
| Booleans | `bool_to_py` |
| Integers | `usize_to_py` |
| Args | `parse_arg_str`, `parse_args_2str` |
| Module | `module_add_object` |
| Exceptions | `new_exception` |
| Ref counting | `PyObj` RAII wrapper (auto Py_DECREF on drop) |
| None | `none()` |

## Dependencies

Only `pyo3-ffi` — raw Python C API bindings. Zero abstractions on top.

## Building

Requires Python development headers:
```bash
# Ubuntu/Debian
sudo apt install python3-dev

# macOS (Homebrew)
brew install python

# The headers are found automatically by pyo3-ffi
```
