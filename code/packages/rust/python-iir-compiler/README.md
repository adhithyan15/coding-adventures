# Python IIR compiler

This crate lowers a bounded Python 3.12 float-expression and `print` subset
directly from `python-parser`'s grammar tree to `interpreter-ir`, then runs it
on the Rust `vm-core` interpreter. Semantic IR is not in this execution path.

The accepted subset and explicit rejections are specified in
`code/specs/LANG79-python-direct-iir-pilot.md`. This is a native interpreter
pilot, not a complete Python implementation.

The bounded `print()` form writes one newline through a zero-argument Rust VM
builtin. `print(1.0)` uses the one-float builtin, and
`print(1.0, 2.0)` writes two supported floats with one separating space.
Three or more arguments, keyword arguments, other callees, and unsupported
expressions fail explicitly.

Run a supported source file with `cargo run -p python-iir-compiler --bin pyvm --
path/to/source.py` from the Rust workspace.

`compile_source` limits input to 64 KiB. The public `compile_ast` entry point
also caps tree items, depth, and text bytes before lowering. `run_source`
returns `PythonRunError` on failure; its `output` field contains any successful
earlier `print` calls, and `pyvm` writes those bytes before the error message.
