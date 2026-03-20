"""AST-based capability detector for Python source code.

This module walks the Python abstract syntax tree to find patterns that
indicate OS-level capability usage. Each detected pattern maps to a
capability in the format `category:action:target`.

## How AST Walking Works

Python's `ast` module parses source code into a tree of typed nodes.
For example, the code:

    import socket
    socket.connect(("evil.com", 443))

Produces a tree like:

    Module
    ├── Import(names=[alias(name='socket')])
    └── Expr(value=Call(
            func=Attribute(value=Name(id='socket'), attr='connect'),
            args=[Tuple(elts=[Constant('evil.com'), Constant(443)])]))

We use `ast.NodeVisitor` to walk this tree. When we visit an `Import`
node with `name='socket'`, we record `net:*:*` (broad network access).
When we visit a `Call` node for `open(...)`, we record `fs:read:*` or
`fs:write:*` depending on the mode argument.

## Detection Rules

Each detection rule maps an AST pattern to a capability:

    AST Pattern                    →  Capability
    ─────────────────────────────     ──────────────────
    import os                      →  fs:*:* (broad)
    open("file.txt")               →  fs:read:file.txt
    open("file.txt", "w")          →  fs:write:file.txt
    pathlib.Path(...).read_text()  →  fs:read:*
    import socket                  →  net:*:*
    import subprocess              →  proc:exec:*
    os.environ["KEY"]              →  env:read:KEY
    import ctypes                  →  ffi:*:*

When the target (e.g., a filename) is a string literal, we record the
exact value. When it's a variable or expression, we record `*` (any
target) because we can't determine the value statically.
"""

import ast
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class DetectedCapability:
    """A single OS capability detected in source code.

    Attributes:
        category: The kind of resource (fs, net, proc, env, ffi, time,
                  stdin, stdout).
        action:   The operation (read, write, connect, exec, etc.).
        target:   The specific resource ("file.txt", "host:443", "*").
        file:     The source file where detection occurred.
        line:     The line number in the source file.
        evidence: The code pattern that triggered detection (for
                  human-readable reporting).
    """

    category: str
    action: str
    target: str
    file: str
    line: int
    evidence: str

    def __str__(self) -> str:
        return f"{self.category}:{self.action}:{self.target}"

    def as_dict(self) -> dict[str, str | int]:
        """Convert to a dictionary for JSON serialization."""
        return {
            "category": self.category,
            "action": self.action,
            "target": self.target,
            "file": self.file,
            "line": self.line,
            "evidence": self.evidence,
        }


# ── Import-to-capability mapping ──────────────────────────────────────
#
# When Python code imports a module, the module name tells us what kind
# of OS capability the code might use. This mapping is conservative:
# importing `os` doesn't mean the code uses the filesystem, but it
# *could*. We flag it and let the manifest comparison decide.
#
# The mapping is organized by category:

_IMPORT_CAPABILITIES: dict[str, tuple[str, str]] = {
    # Filesystem access
    "os": ("fs", "*"),
    "os.path": ("fs", "*"),
    "shutil": ("fs", "*"),
    "pathlib": ("fs", "*"),
    "glob": ("fs", "list"),
    "fnmatch": ("fs", "list"),
    "tempfile": ("fs", "write"),
    "io": ("fs", "*"),
    # Network access
    "socket": ("net", "*"),
    "ssl": ("net", "*"),
    "http": ("net", "*"),
    "http.client": ("net", "connect"),
    "http.server": ("net", "listen"),
    "urllib": ("net", "connect"),
    "urllib.request": ("net", "connect"),
    "xmlrpc": ("net", "*"),
    # Process execution
    "subprocess": ("proc", "exec"),
    "multiprocessing": ("proc", "fork"),
    "signal": ("proc", "signal"),
    # FFI / native code
    "ctypes": ("ffi", "*"),
    "cffi": ("ffi", "*"),
    # Environment (os.environ is handled specially in the visitor)
}

# ── Attribute-call-to-capability mapping ──────────────────────────────
#
# Beyond imports, specific function calls indicate capability usage.
# For example, `os.listdir(".")` indicates filesystem list capability.
# These are detected by looking at Call nodes whose func is an Attribute.

_CALL_CAPABILITIES: dict[tuple[str, str], tuple[str, str]] = {
    # os module filesystem calls
    ("os", "listdir"): ("fs", "list"),
    ("os", "scandir"): ("fs", "list"),
    ("os", "walk"): ("fs", "list"),
    ("os", "makedirs"): ("fs", "create"),
    ("os", "mkdir"): ("fs", "create"),
    ("os", "remove"): ("fs", "delete"),
    ("os", "unlink"): ("fs", "delete"),
    ("os", "rmdir"): ("fs", "delete"),
    ("os", "rename"): ("fs", "write"),
    ("os", "replace"): ("fs", "write"),
    ("os", "chmod"): ("fs", "write"),
    ("os", "chown"): ("fs", "write"),
    ("os", "stat"): ("fs", "read"),
    ("os", "lstat"): ("fs", "read"),
    # os module process calls
    ("os", "system"): ("proc", "exec"),
    ("os", "exec"): ("proc", "exec"),
    ("os", "execl"): ("proc", "exec"),
    ("os", "execle"): ("proc", "exec"),
    ("os", "execlp"): ("proc", "exec"),
    ("os", "execv"): ("proc", "exec"),
    ("os", "execve"): ("proc", "exec"),
    ("os", "execvp"): ("proc", "exec"),
    ("os", "execvpe"): ("proc", "exec"),
    ("os", "popen"): ("proc", "exec"),
    ("os", "fork"): ("proc", "fork"),
    ("os", "kill"): ("proc", "signal"),
    ("os", "killpg"): ("proc", "signal"),
    # os module environment calls
    ("os", "getenv"): ("env", "read"),
    ("os", "putenv"): ("env", "write"),
    ("os", "unsetenv"): ("env", "write"),
    # subprocess calls
    ("subprocess", "run"): ("proc", "exec"),
    ("subprocess", "call"): ("proc", "exec"),
    ("subprocess", "check_call"): ("proc", "exec"),
    ("subprocess", "check_output"): ("proc", "exec"),
    ("subprocess", "Popen"): ("proc", "exec"),
    # shutil calls
    ("shutil", "copy"): ("fs", "write"),
    ("shutil", "copy2"): ("fs", "write"),
    ("shutil", "copytree"): ("fs", "write"),
    ("shutil", "rmtree"): ("fs", "delete"),
    ("shutil", "move"): ("fs", "write"),
}


class CapabilityAnalyzer(ast.NodeVisitor):
    """Walks a Python AST to detect OS capability usage.

    This is the core detection engine. It inherits from `ast.NodeVisitor`,
    which provides a `visit_*` method dispatch mechanism. For each node
    type we care about (Import, ImportFrom, Call, Subscript, Attribute),
    we define a `visit_*` method that checks if the node matches a
    capability pattern.

    Usage:
        tree = ast.parse(source_code)
        analyzer = CapabilityAnalyzer("path/to/file.py")
        analyzer.visit(tree)
        capabilities = analyzer.detected
    """

    def __init__(self, filename: str) -> None:
        self.filename = filename
        self.detected: list[DetectedCapability] = []

        # Track imported module names so we can resolve calls like
        # `os.listdir(".")` where `os` was imported earlier.
        self._imported_modules: set[str] = set()

        # Track `from X import Y` aliases so we can resolve calls like
        # `listdir(".")` where `from os import listdir` was used.
        self._from_imports: dict[str, str] = {}

    def _add(
        self,
        category: str,
        action: str,
        target: str,
        line: int,
        evidence: str,
    ) -> None:
        """Record a detected capability."""
        self.detected.append(
            DetectedCapability(
                category=category,
                action=action,
                target=target,
                file=self.filename,
                line=line,
                evidence=evidence,
            )
        )

    # ── Import visitors ───────────────────────────────────────────────

    def visit_Import(self, node: ast.Import) -> None:
        """Detect capabilities from `import X` statements.

        Example:
            import socket    →  net:*:*
            import os        →  fs:*:*
            import subprocess →  proc:exec:*
        """
        for alias in node.names:
            module_name = alias.name
            local_name = alias.asname if alias.asname else module_name
            self._imported_modules.add(local_name)

            if module_name in _IMPORT_CAPABILITIES:
                category, action = _IMPORT_CAPABILITIES[module_name]
                self._add(
                    category,
                    action,
                    "*",
                    node.lineno,
                    f"import {module_name}",
                )
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        """Detect capabilities from `from X import Y` statements.

        Example:
            from os import listdir     →  fs:*:*
            from socket import socket  →  net:*:*
            from subprocess import run →  proc:exec:*
        """
        module_name = node.module or ""

        if node.names:
            for alias in node.names:
                local_name = alias.asname if alias.asname else alias.name
                self._from_imports[local_name] = module_name

        if module_name in _IMPORT_CAPABILITIES:
            category, action = _IMPORT_CAPABILITIES[module_name]
            names = ", ".join(a.name for a in (node.names or []))
            self._add(
                category,
                action,
                "*",
                node.lineno,
                f"from {module_name} import {names}",
            )
        self.generic_visit(node)

    # ── Call visitors ─────────────────────────────────────────────────

    def visit_Call(self, node: ast.Call) -> None:
        """Detect capabilities from function and method calls.

        Handles several patterns:

        1. Built-in `open()` call:
           open("file.txt")       →  fs:read:file.txt
           open("file.txt", "w")  →  fs:write:file.txt

        2. Module attribute call:
           os.listdir(".")        →  fs:list:*
           subprocess.run(...)    →  proc:exec:*

        3. Direct imported call:
           from os import listdir
           listdir(".")           →  fs:list:*
        """
        self._check_open_call(node)
        self._check_attribute_call(node)
        self._check_direct_imported_call(node)
        self.generic_visit(node)

    def _check_open_call(self, node: ast.Call) -> None:
        """Detect `open(path)` and `open(path, mode)` calls.

        The built-in `open()` is the most common way to access files in
        Python. We determine read vs write from the mode argument:

        - No mode or "r" → read
        - "w", "a", "x" → write
        - "r+" → read and write

        If the path argument is a string literal, we record the exact path.
        If it's a variable, we record "*" (unknown target).
        """
        if not isinstance(node.func, ast.Name):
            return
        if node.func.id != "open":
            return

        # Determine the target path
        target = "*"
        if (
            node.args
            and isinstance(node.args[0], ast.Constant)
            and isinstance(node.args[0].value, str)
        ):
            target = node.args[0].value

        # Determine read vs write from mode argument
        mode = "r"
        if (
            len(node.args) >= 2
            and isinstance(node.args[1], ast.Constant)
            and isinstance(node.args[1].value, str)
        ):
            mode = node.args[1].value
        # Also check keyword argument `mode=`
        for kw in node.keywords:
            if (
                kw.arg == "mode"
                and isinstance(kw.value, ast.Constant)
                and isinstance(kw.value.value, str)
            ):
                mode = kw.value.value

        action = "write" if any(c in mode for c in "wax") else "read"

        self._add(
            "fs",
            action,
            target,
            node.lineno,
            f"open({target!r}, {mode!r})" if target != "*" else f"open(..., {mode!r})",
        )

    def _check_attribute_call(self, node: ast.Call) -> None:
        """Detect `module.function(...)` calls like `os.listdir(...)`.

        We check if the call's function is an attribute access (e.g.,
        `os.listdir`) where the object name matches an imported module
        and the attribute matches a known capability-bearing function.
        """
        if not isinstance(node.func, ast.Attribute):
            return
        if not isinstance(node.func.value, ast.Name):
            return

        obj_name = node.func.value.id
        attr_name = node.func.attr

        # Check if this is a known capability-bearing call
        key = (obj_name, attr_name)
        if key in _CALL_CAPABILITIES:
            category, action = _CALL_CAPABILITIES[key]
            self._add(
                category,
                action,
                "*",
                node.lineno,
                f"{obj_name}.{attr_name}(...)",
            )

    def _check_direct_imported_call(self, node: ast.Call) -> None:
        """Detect calls to directly imported functions.

        Example:
            from os import listdir
            listdir(".")  →  fs:list:*

        We look up the function name in our `_from_imports` dict to find
        the original module, then check the capability mapping.
        """
        if not isinstance(node.func, ast.Name):
            return

        func_name = node.func.id
        if func_name in self._from_imports:
            module_name = self._from_imports[func_name]
            key = (module_name, func_name)
            if key in _CALL_CAPABILITIES:
                category, action = _CALL_CAPABILITIES[key]
                self._add(
                    category,
                    action,
                    "*",
                    node.lineno,
                    f"{func_name}(...) [from {module_name}]",
                )

    # ── Subscript visitors ────────────────────────────────────────────

    def visit_Subscript(self, node: ast.Subscript) -> None:
        """Detect `os.environ["KEY"]` patterns.

        os.environ is a dict-like object. Accessing it via subscript
        indicates environment variable reading.
        """
        if (
            isinstance(node.value, ast.Attribute)
            and isinstance(node.value.value, ast.Name)
            and node.value.value.id == "os"
            and node.value.attr == "environ"
        ):
            target = "*"
            if isinstance(node.slice, ast.Constant) and isinstance(
                node.slice.value, str
            ):
                target = node.slice.value

            self._add(
                "env",
                "read",
                target,
                node.lineno,
                f"os.environ[{target!r}]" if target != "*" else "os.environ[...]",
            )
        self.generic_visit(node)


def analyze_file(filepath: str | Path) -> list[DetectedCapability]:
    """Analyze a single Python file for capability usage.

    Args:
        filepath: Path to the Python source file.

    Returns:
        List of detected capabilities.

    Raises:
        SyntaxError: If the file contains invalid Python syntax.
        FileNotFoundError: If the file does not exist.
    """
    filepath = Path(filepath)
    source = filepath.read_text(encoding="utf-8")
    tree = ast.parse(source, filename=str(filepath))
    analyzer = CapabilityAnalyzer(str(filepath))
    analyzer.visit(tree)
    return analyzer.detected


def analyze_directory(
    directory: str | Path,
    exclude_tests: bool = False,
) -> list[DetectedCapability]:
    """Analyze all Python files in a directory tree.

    Walks the directory recursively, parsing each `.py` file and
    collecting all detected capabilities.

    Args:
        directory: Root directory to analyze.
        exclude_tests: If True, skip files in `tests/` and `test/`
                       directories. Test files often use capabilities
                       (e.g., tempfile) that the package itself doesn't.

    Returns:
        List of all detected capabilities across all files.
    """
    directory = Path(directory)
    all_detected: list[DetectedCapability] = []

    skip_dirs = {
        ".venv",
        "__pycache__",
        ".git",
        "node_modules",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
    }
    if exclude_tests:
        skip_dirs |= {"tests", "test"}

    for py_file in directory.rglob("*.py"):
        # Skip excluded directories
        if any(part in skip_dirs for part in py_file.parts):
            continue

        try:
            detected = analyze_file(py_file)
            all_detected.extend(detected)
        except SyntaxError:
            pass  # Skip files with syntax errors

    return all_detected
