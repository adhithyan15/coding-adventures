# Changelog

All notable changes to the ruby-bridge crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `rb_int2inum` is declared with an `isize` (`intptr_t`) parameter instead of `c_long`. Ruby's `rb_int2inum` takes an `intptr_t`; on 64-bit Windows (LLP64) `long` is 32 bits, so the upper half of the argument register was undefined and a negative value arrived as its unsigned 32-bit reading (`-494` became `4294966802`). `usize_to_rb` casts through `isize` accordingly. Found by the first Windows run of `font-parser-ruby`'s smoke test; every extension that returns a negative Integer through this binding was affected on Windows.

### Changed — `BUILD`/`BUILD_windows` build and lint instead of echoing

Both recipes were an `echo` ("compile-only crate").  They now run
`cargo build --release` and `cargo clippy --all-targets -D warnings` on every OS.
The crate has no tests of its own; its `rb_*` imports are exercised end to end by
font-parser-ruby's smoke test, which requires a real cdylib into Ruby.

## [0.1.0] - Unreleased

### Added
- Module and class definition (`define_module`, `define_class_under`)
- Module function definition (`define_module_function_raw`) — defines a function as both a module singleton method and a private instance method when the module is mixed in; the Ruby idiom for stateless utilities
- Class path lookup (`path2class`) — looks up a Ruby class or module by fully-qualified constant path (e.g. `"CodingAdventures::Foo"`); avoids the Windows MSVC linking issues that affect the `rb_eArgError` extern static symbols
- Method binding (`define_method`, `define_singleton_method`)
- String conversion (`str_to_rb`, `str_from_rb`)
- Array conversion (`vec_str_to_rb`, `vec_str_from_rb`, `vec_vec_str_to_rb`, `vec_tuple2_str_to_rb`)
- Boolean and integer conversion
- Data wrapping for Rust structs (`wrap_data`, `unwrap_data`)
- Exception handling (`raise`, `raise_runtime_error`, `raise_arg_error`)
- Well-known class constants (`object_class`, `standard_error_class`, etc.)
