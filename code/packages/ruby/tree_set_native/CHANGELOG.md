# Changelog

## Unreleased

### Fixed

- Windows link: the `build.rs` import-library symbol list now includes every `rb_*` function `ruby-bridge` declares (it was missing `rb_eval_string`, `rb_str_new`, `rb_string_value_ptr`, `rb_define_module_function`, `rb_hash_new`, `rb_hash_aset`), so the cdylib no longer fails to link with undefined `rb_*` symbols on the Windows runner.

- Its own `rb_int2inum` declaration takes an `isize` (`intptr_t`) instead of `c_long`, which is 32 bits on 64-bit Windows and returned negative Integers as their unsigned 32-bit value.

## 0.1.0

- Added the native Ruby tree set implementation.
