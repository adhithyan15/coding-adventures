# Python IIR compiler

This crate lowers a bounded Python 3.12 float-expression, module-level
binding and `print` subset directly from `python-parser`'s grammar tree to
`interpreter-ir`, then runs it on the Rust `vm-core` interpreter. Semantic IR
is not in this execution path.

The accepted subset and explicit rejections are specified in
`code/specs/LANG79-python-direct-iir-pilot.md`. This is a native interpreter
pilot, not a complete Python implementation.

The bounded `print()` form writes one newline through a zero-argument Rust VM
builtin. `print(1.0)` uses the one-float builtin, and
`print(1.0, 2.0)` writes two supported floats with one separating space.
`print(1.0, 2.0, 3.0)` writes three supported floats with one space between
each value. Four or more arguments, keyword arguments, other callees, and
unsupported expressions fail explicitly.

Module-level float bindings are supported. A statement `NAME = <float
expression>` evaluates the value and then binds it; names may be rebound and
read in any float expression, including every `print` argument:

```python
rate = 1.5
total = rate * 2.0
total = total + 1.0
print(rate, total)      # 1.5 4.0
print(missing)          # NameError: name 'missing' is not defined
```

Reading a name before any statement binds it raises `NameError` at run time;
output from earlier statements is kept and later statements do not run. User
names live in the `py:` register namespace (`total` is `py:total`) so they
cannot collide with compiler temporaries (`py0`, `py1`, ...) or builtins.
Binding `print`, chained (`a = b = 1.0`), tuple, annotated, augmented
(`x += 1.0`), attribute and subscript assignment, integer values, and using
`print` or an unbound builtin name such as `float` as a value all fail at
compile time. Direct AST callers must supply canonical NAME and EQUALS tokens.

Run a supported source file with `cargo run -p python-iir-compiler --bin pyvm --
path/to/source.py` from the Rust workspace.

`compile_source` limits input to 64 KiB. The public `compile_ast` entry point
also caps tree items, depth, and text bytes before lowering. `run_source`
returns `PythonRunError` on failure; its `output` field contains any successful
earlier `print` calls, and `pyvm` writes those bytes before the error message.

Direct AST arithmetic also requires canonical unary/binary operator tokens
and grouping delimiters, including token kind, effective type and spelling.
Forged syntax tokens fail before execution; supported source expressions and
the existing resource bounds retain their behavior.

Direct AST float atoms also require the canonical named `FLOAT` token kind and
the complete ASCII spelling declared by the Python 3.12 lexer, checked before
underscore removal or f64 conversion. Forged integer spellings, malformed
underscores, nonnumeric names and conflicting enum kinds fail explicitly.
Valid exponent overflow/underflow and raw-versus-cooked metadata behavior,
source syntax and all existing runtime/resource bounds remain unchanged.
