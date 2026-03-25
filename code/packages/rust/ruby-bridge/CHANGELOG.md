# Changelog

All notable changes to the ruby-bridge crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
