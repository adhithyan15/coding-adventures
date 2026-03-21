# Changelog

All notable changes to the python-bridge crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- PyObj RAII wrapper for automatic reference counting
- String conversion (`str_to_py`, `str_from_py`)
- List conversion (`vec_str_to_py`, `vec_str_from_py`, `vec_vec_str_to_py`, `vec_tuple2_str_to_py`)
- Set conversion (`set_str_to_py`, `set_str_from_py`)
- Boolean and integer conversion
- Argument parsing helpers
- Module object registration
- Exception class creation
- Error handling utilities
