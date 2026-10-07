# Python IIR compiler

This crate lowers a bounded Python 3.12 float-expression and `print` subset
directly from `python-parser`'s grammar tree to `interpreter-ir`, then runs it
on the Rust `vm-core` interpreter. Semantic IR is not in this execution path.

The accepted subset and explicit rejections are specified in
`code/specs/LANG79-python-direct-iir-pilot.md`. This is a native interpreter
pilot, not a complete Python implementation.

Run a supported source file with `cargo run -p python-iir-compiler --bin pyvm --
path/to/source.py` from the Rust workspace.
