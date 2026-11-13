---
category: Native extensions & FFI
---

# A compile-only extension crate has never been loaded, so its ABI bugs are all still there

`font-parser-node`, `font-parser-python` and `font-parser-ruby` had `BUILD` files
that were an `echo` ("compile-only crate, requires X headers to link"), so for
months nothing ever loaded their cdylib into the interpreter it was written for.
The first real load found, per crate:

- **node**: five `napi_*` functions declared in the crate's own `extern "C"`
  block lacked the `node.exe` raw-dylib marker that node-bridge's block has, so
  the Windows link failed with `undefined reference to napi_get_value_int64`.
  Every `extern` block that names host symbols needs the marker, not just the
  bridge's.
- **python**: no `build.rs`, so nothing told the Windows linker about
  `pythonXY.lib`.
- **ruby** (three bugs, and `require` failed before any call ran):
  `rb_data_object_get` is a `static inline` function in Ruby's headers, so
  libruby does not export it (`undefined symbol` at `require`);
  `rb_define_module_function` calls the C function as `(self, arg1, ...)` but the
  functions were written `(arg1, ..., self)`, so every call raised
  `TypeError: no implicit conversion of Module into String`; and `Qnil` was
  hard-coded as `0x08`, which is `0x04` since Ruby 3.2.

A compile check proves none of this: a wrong symbol, a wrong argument order and a
wrong constant all compile. The fix that keeps it fixed is a smoke test that
loads the built library into the real host (`node`, `python`, `ruby`) and calls
every export, including an error path, run by `BUILD` on every OS. When several
cdylibs share a `[lib] name` (here all three are `font_parser_native`), give each
recipe its own `--target-dir`, or parallel builds overwrite each other's file.
