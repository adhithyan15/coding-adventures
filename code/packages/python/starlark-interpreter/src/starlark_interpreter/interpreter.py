"""Starlark Interpreter — The complete execution pipeline.

==========================================================================
Chapter 1: What Is an Interpreter?
==========================================================================

An interpreter takes source code and executes it. Unlike a compiler that
produces an executable file, an interpreter runs the program directly. Our
Starlark interpreter uses a **multi-stage pipeline** internally:

    source code → tokens → AST → bytecode → execution

Each stage is handled by a separate package:

    1. **Lexer** (starlark-lexer): Breaks source text into tokens.
       ``"x = 1 + 2"`` → ``[NAME("x"), EQUALS, INT("1"), PLUS, INT("2")]``

    2. **Parser** (starlark-parser): Groups tokens into an Abstract Syntax
       Tree (AST). ``[NAME, EQUALS, INT, PLUS, INT]`` → ``AssignStmt(x, Add(1, 2))``

    3. **Compiler** (starlark-ast-to-bytecode-compiler): Translates the AST
       into bytecode instructions. ``AssignStmt(x, Add(1, 2))`` →
       ``[LOAD_CONST 1, LOAD_CONST 2, ADD, STORE_NAME x]``

    4. **VM** (starlark-vm): Executes bytecode on a virtual stack machine.
       Runs the instructions and produces the final result.

This package chains them together and adds the critical ``load()`` function.

==========================================================================
Chapter 2: The load() Function
==========================================================================

``load()`` is what makes BUILD files work. It's how a BUILD file imports
rule definitions from a shared library:

    load("//rules/python.star", "py_library")

    py_library(
        name = "mylib",
        deps = ["//other:lib"],
    )

When the VM encounters a ``load()`` call:

1. **Resolve** the path — ``//rules/python.star`` → actual file contents
2. **Execute** the file through the same interpreter pipeline
3. **Extract** the requested symbols from the result
4. **Inject** them into the current scope

This means ``load()`` is **recursive** — the loaded file is itself a Starlark
program that gets interpreted. Loaded files are cached so each file is
evaluated at most once, matching Bazel's semantics.

==========================================================================
Chapter 3: File Resolvers
==========================================================================

The interpreter doesn't know where files live on disk. Instead, it accepts
a **file resolver** — a callable that maps label paths to file contents:

    def my_resolver(label: str) -> str:
        # //rules/python.star → read from repo root
        path = label.replace("//", "/path/to/repo/")
        with open(path) as f:
            return f.read()

The build tool provides a resolver that knows the repository layout.
For testing, you can provide a dict-based resolver:

    resolver = {"//rules/test.star": "def foo(): return 42"}
    result = interpret(source, file_resolver=resolver)

==========================================================================
Chapter 4: Usage Examples
==========================================================================

**Simple execution:**

    from starlark_interpreter import interpret

    result = interpret("x = 1 + 2\\nprint(x)\\n")
    assert result.variables["x"] == 3
    assert result.output == ["3"]

**With load():**

    files = {
        "//rules/math.star": "def double(n):\\n    return n * 2\\n",
    }
    result = interpret(
        'load("//rules/math.star", "double")\\n'
        'result = double(21)\\n',
        file_resolver=files,
    )
    assert result.variables["result"] == 42

**From a file:**

    result = interpret_file("path/to/program.star")
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable

from virtual_machine import GenericVM

from starlark_ast_to_bytecode_compiler import compile_starlark
from starlark_vm import StarlarkResult, create_starlark_vm


# =========================================================================
# File Resolver Types
# =========================================================================

# A file resolver is either a callable (label → content) or a dict.
FileResolver = Callable[[str], str] | dict[str, str] | None


def _resolve_file(resolver: FileResolver, label: str) -> str:
    """Resolve a label to file contents using the configured resolver.

    Supports two forms:
    - A dict mapping labels to content strings (for testing)
    - A callable that takes a label and returns content (for production)

    Raises FileNotFoundError if the label cannot be resolved.
    """
    if resolver is None:
        raise FileNotFoundError(
            f"load() called but no file_resolver configured. "
            f"Cannot resolve: {label}"
        )
    if isinstance(resolver, dict):
        if label in resolver:
            return resolver[label]
        raise FileNotFoundError(
            f"load(): file not found in resolver: {label}"
        )
    # Callable resolver
    return resolver(label)


# =========================================================================
# The Interpreter
# =========================================================================


@dataclass
class StarlarkInterpreter:
    """A configurable Starlark interpreter.

    Wraps the full lexer → parser → compiler → VM pipeline with:
    - ``load()`` support via a file resolver
    - File caching (each loaded file is evaluated at most once)
    - Configurable recursion limits

    For most use cases, the module-level ``interpret()`` function is
    simpler. Use this class when you need to share a cache across
    multiple interpret calls or configure advanced options.
    """

    file_resolver: FileResolver = None
    """How to resolve ``load()`` paths to file contents."""

    max_recursion_depth: int = 200
    """Maximum call stack depth for function calls."""

    _load_cache: dict[str, dict[str, Any]] = field(
        default_factory=dict, repr=False
    )
    """Cache of already-loaded files: label → exported variables.

    Each file is evaluated at most once. Subsequent ``load()`` calls for
    the same file return cached symbols. This matches Bazel semantics
    where loaded files are frozen after first evaluation.
    """

    def interpret(self, source: str) -> StarlarkResult:
        """Execute Starlark source code and return the result.

        This is the main entry point. It:
        1. Compiles the source to bytecode
        2. Creates a fresh VM with ``load()`` registered as a builtin
        3. Executes the bytecode
        4. Returns the result (variables, output, traces)

        Parameters
        ----------
        source : str
            Starlark source code. Should end with a newline.

        Returns
        -------
        StarlarkResult
            The execution result with variables, output, and traces.
        """
        # Compile source to bytecode
        code = compile_starlark(source)

        # Create a VM with load() support
        vm = create_starlark_vm(max_recursion_depth=self.max_recursion_depth)
        self._register_load_handlers(vm)

        # Execute
        traces = vm.execute(code)

        return StarlarkResult(
            variables=dict(vm.variables),
            output=list(vm.output),
            traces=traces,
        )

    def interpret_file(self, path: str) -> StarlarkResult:
        """Execute a Starlark file by reading it from the filesystem.

        Parameters
        ----------
        path : str
            Path to the Starlark file.

        Returns
        -------
        StarlarkResult
            The execution result.
        """
        with open(path) as f:
            source = f.read()
        # Ensure source ends with newline (parser requirement)
        if not source.endswith("\n"):
            source += "\n"
        return self.interpret(source)

    def _register_load_handlers(self, vm: GenericVM) -> None:
        """Override the VM's LOAD_MODULE handler to actually resolve and execute files.

        The compiler compiles ``load("file.star", "symbol")`` into:
        - ``LOAD_MODULE`` — resolve and execute the file, push a module dict
        - ``IMPORT_FROM`` — extract a symbol from the module dict

        The default VM handlers are stubs. We override ``LOAD_MODULE`` with a
        closure that uses the interpreter's file resolver and cache to actually
        load files.

        We also register a ``load`` builtin as a fallback for any code that
        calls ``load()`` as a function rather than a statement.
        """
        from starlark_ast_to_bytecode_compiler import Op
        from virtual_machine import VMTypeError
        from virtual_machine.vm import CodeObject, Instruction

        interpreter = self  # Capture for closures

        def handle_load_module(
            vm_inst: GenericVM, instr: Instruction, code: CodeObject
        ) -> str | None:
            """LOAD_MODULE — Resolve a file label, execute it, push its variables.

            =================================================================
            How Module Loading Works
            =================================================================

            When the compiler encounters ``load("//rules/python.star", "sym")``,
            it emits:

                LOAD_MODULE 0    # names[0] = "//rules/python.star"
                DUP              # Keep module on stack for multiple imports
                IMPORT_FROM 1    # names[1] = "sym" — extract from module dict
                STORE_NAME 1     # Store as "sym" in current scope

            This handler:
            1. Reads the module label from the names pool
            2. Checks the interpreter's cache (each file evaluated once)
            3. If not cached, resolves the file and executes it
            4. Pushes the module's variables as a dict onto the stack

            IMPORT_FROM then pops symbols from this dict.
            """
            index = instr.operand
            assert isinstance(index, int)
            module_label = code.names[index]

            # Check cache
            if module_label not in interpreter._load_cache:
                # Resolve and execute the file
                contents = _resolve_file(interpreter.file_resolver, module_label)
                if not contents.endswith("\n"):
                    contents += "\n"
                result = interpreter.interpret(contents)
                interpreter._load_cache[module_label] = dict(result.variables)

            # Push the module's exported variables as a dict
            vm_inst.push(dict(interpreter._load_cache[module_label]))
            vm_inst.advance_pc()
            return None

        # Override the LOAD_MODULE handler with our file-resolving version
        vm.register_opcode(Op.LOAD_MODULE, handle_load_module)


# =========================================================================
# Module-level Convenience Functions
# =========================================================================


def interpret(
    source: str,
    file_resolver: FileResolver = None,
    max_recursion_depth: int = 200,
) -> StarlarkResult:
    """Execute Starlark source code and return the result.

    This is the simplest API — one function call does everything.

    Parameters
    ----------
    source : str
        Starlark source code. Should end with a newline.
    file_resolver : FileResolver, optional
        How to resolve ``load()`` paths. Can be a dict mapping labels
        to content strings (for testing) or a callable.
    max_recursion_depth : int
        Maximum call stack depth. Default 200.

    Returns
    -------
    StarlarkResult
        The execution result with variables, output, and traces.

    Examples
    --------
    >>> result = interpret("x = 1 + 2\\nprint(x)\\n")
    >>> result.variables["x"]
    3
    >>> result.output
    ['3']
    """
    interp = StarlarkInterpreter(
        file_resolver=file_resolver,
        max_recursion_depth=max_recursion_depth,
    )
    return interp.interpret(source)


def interpret_file(
    path: str,
    file_resolver: FileResolver = None,
    max_recursion_depth: int = 200,
) -> StarlarkResult:
    """Execute a Starlark file by path.

    Parameters
    ----------
    path : str
        Path to the Starlark file.
    file_resolver : FileResolver, optional
        How to resolve ``load()`` paths.
    max_recursion_depth : int
        Maximum call stack depth. Default 200.

    Returns
    -------
    StarlarkResult
        The execution result.
    """
    interp = StarlarkInterpreter(
        file_resolver=file_resolver,
        max_recursion_depth=max_recursion_depth,
    )
    return interp.interpret_file(path)
