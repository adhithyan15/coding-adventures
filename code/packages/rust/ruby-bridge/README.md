# ruby-bridge

Thin safe Rust wrapper over Ruby's C API — no Magnus, no macros, no magic.

## What is this?

A ~350-line crate that wraps only the raw Ruby C API functions needed to build native extensions. It replaces Magnus (~15,000 lines) with explicit, debuggable code.

## What it provides

| Category | Functions |
|----------|-----------|
| Modules | `define_module`, `define_module_under` |
| Classes | `define_class_under` |
| Methods | `define_method`, `define_singleton_method` |
| Strings | `str_to_rb`, `str_from_rb` |
| Binary strings | `bytes_to_rb`, `bytes_from_rb` |
| Arrays | `array_new`, `array_push`, `vec_str_to_rb`, `vec_str_from_rb`, `vec_vec_str_to_rb` |
| Booleans | `bool_to_rb`, `qtrue`, `qfalse`, `qnil` |
| Integers | `usize_to_rb` |
| Data wrapping | `wrap_data`, `unwrap_data`, `unwrap_data_mut` |
| Exceptions | `raise`, `raise_runtime_error`, `raise_arg_error` |
| Constants | `object_class`, `standard_error_class`, `arg_error_class`, `runtime_error_class` |

## Usage

```rust
use ruby_bridge::*;

#[no_mangle]
pub extern "C" fn Init_my_extension() {
    let module = define_module("MyModule");
    let klass = define_class_under(module, "MyClass", object_class());
    define_method(klass, "hello", my_hello as _, 0);
}

extern "C" fn my_hello(_self: VALUE) -> VALUE {
    str_to_rb("Hello from Rust!")
}
```

## Dependencies

Only `rb-sys` — raw Ruby C API bindings generated via bindgen. Zero abstractions on top.
