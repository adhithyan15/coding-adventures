"""Starlark Interpreter — Full pipeline from source to execution.

This is the top-level package that chains all four stages of Starlark
execution into a single, easy-to-use API:

    source code → lexer → parser → compiler → VM → result

It also adds ``load()`` support, enabling BUILD files to import rule
definitions from other Starlark files — the key mechanism that makes
Bazel-style build systems work.

Key exports:
    - interpret: Execute Starlark source code and return the result.
    - interpret_file: Execute a Starlark file by path.
    - StarlarkInterpreter: The interpreter class with full configuration.
"""

from starlark_interpreter.interpreter import (
    StarlarkInterpreter,
    interpret,
    interpret_file,
)

__all__ = [
    "StarlarkInterpreter",
    "interpret",
    "interpret_file",
]
