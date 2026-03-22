# Changelog

All notable changes to the node-bridge crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- String conversion (`str_to_js`, `str_from_js`)
- Array conversion (`vec_str_to_js`, `vec_str_from_js`, `vec_vec_str_to_js`, `vec_tuple2_str_to_js`)
- Boolean and number conversion
- Argument parsing (`get_cb_info`)
- Data wrapping for Rust structs (`wrap_data`, `unwrap_data`)
- Class definition (`define_class`, `method_property`)
- Error handling (`throw_error`)
- Constants (`undefined`, `null`)
