"""AST capability scanner and static CWE detector (VER08/VER15).

Scans Python Abstract Syntax Trees for declared/undeclared OS capabilities
and flags security weaknesses (CWE-1333, CWE-674, CWE-78, CWE-94, CWE-22).
Pure Python stdlib only. Zero third-party runtime dependencies.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass
from typing import Any

from coding_adventures_ca_capability_analyzer.types import (
    CAP_DYNAMIC_EVAL,
    CAP_FFI_UNSAFE,
    CAP_FS_READ,
    CAP_FS_WRITE,
    CAP_NET_CLIENT,
    CAP_NET_SERVER,
    CAP_PROCESS_EXEC,
    CAP_REGEX_RE,
    MAX_AST_NODES,
    MAX_FILE_BYTES,
    Finding,
)


@dataclass(frozen=True)
class FileScanResult:
    """Scan results for an individual Python file."""

    filename: str
    capabilities: frozenset[str]
    findings: tuple[Finding, ...]
    nodes_visited: int


class _ASTScanner(ast.NodeVisitor):
    def __init__(self, filename: str) -> None:
        self.filename = filename
        self.capabilities: set[str] = set()
        self.findings: list[Finding] = []
        self.nodes_visited = 0
        self.imports: dict[str, str] = {}
        self.enclosing_functions: list[str] = []

    def generic_visit(self, node: ast.AST) -> Any:
        self.nodes_visited += 1
        if self.nodes_visited > MAX_AST_NODES:
            raise ValueError(
                f"File '{self.filename}' exceeded MAX_AST_NODES ({MAX_AST_NODES})"
            )
        return super().generic_visit(node)

    def _add_finding(
        self,
        node: ast.AST,
        cwe: str,
        message: str,
        capability: str | None = None,
        severity: str = "error",
    ) -> None:
        line = getattr(node, "lineno", 1)
        col = getattr(node, "col_offset", 0)
        self.findings.append(
            Finding(
                file_path=self.filename,
                line=line,
                col=col,
                cwe=cwe,
                message=message,
                capability=capability,
                severity=severity,
            )
        )

    def visit_Import(self, node: ast.Import) -> Any:
        for alias in node.names:
            name = alias.name
            asname = alias.asname or name
            self.imports[asname] = name

            # Check imports implying capabilities
            if name == "re" or name.startswith("re."):
                self.capabilities.add(CAP_REGEX_RE)
                self._add_finding(
                    node,
                    "CWE-1333",
                    "Flagged `re` module usage / regex compilation",
                    capability=CAP_REGEX_RE,
                    severity="warning",
                )
            elif name == "ctypes" or name.startswith("ctypes."):
                self.capabilities.add(CAP_FFI_UNSAFE)
            elif name in (
                "socket",
                "socketserver",
                "http.client",
                "urllib",
                "urllib.request",
            ):
                self.capabilities.add(CAP_NET_CLIENT)
            elif name in ("subprocess", "posix_spawn"):
                self.capabilities.add(CAP_PROCESS_EXEC)
            elif name in ("pickle", "_pickle", "shelve"):
                self.capabilities.add(CAP_DYNAMIC_EVAL)
        return self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> Any:
        module = node.module or ""
        for alias in node.names:
            full_name = f"{module}.{alias.name}" if module else alias.name
            asname = alias.asname or alias.name
            self.imports[asname] = full_name

            if module == "re" or module.startswith("re.") or full_name == "re":
                self.capabilities.add(CAP_REGEX_RE)
                self._add_finding(
                    node,
                    "CWE-1333",
                    "Flagged `re` module import / regex compilation",
                    capability=CAP_REGEX_RE,
                    severity="warning",
                )
            elif module == "ctypes" or module.startswith("ctypes."):
                self.capabilities.add(CAP_FFI_UNSAFE)
            elif module.startswith("subprocess"):
                self.capabilities.add(CAP_PROCESS_EXEC)
            elif module in ("socket", "socketserver", "http.client", "urllib.request"):
                self.capabilities.add(CAP_NET_CLIENT)
            elif module in ("pickle", "_pickle", "shelve"):
                self.capabilities.add(CAP_DYNAMIC_EVAL)

        return self.generic_visit(node)

    def visit_FunctionDef(self, node: ast.FunctionDef) -> Any:
        self.enclosing_functions.append(node.name)
        self.generic_visit(node)
        self.enclosing_functions.pop()

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> Any:
        self.enclosing_functions.append(node.name)
        self.generic_visit(node)
        self.enclosing_functions.pop()

    def _resolve_callee(self, func_node: ast.AST) -> str | None:
        """Resolve called function to its canonical name iteratively."""
        parts: list[str] = []
        curr = func_node
        while isinstance(curr, ast.Attribute):
            parts.append(curr.attr)
            curr = curr.value

        if isinstance(curr, ast.Name):
            base = self.imports.get(curr.id, curr.id)
            parts.append(base)
            parts.reverse()
            return ".".join(parts)

        return None

    def visit_Call(self, node: ast.Call) -> Any:
        # CWE-674: Precise direct recursion detection in AST call graph
        if self.enclosing_functions:
            current_fn = self.enclosing_functions[-1]
            is_direct_recursion = False

            if (
                isinstance(node.func, ast.Name)
                and node.func.id == current_fn
                or (
                    isinstance(node.func, ast.Attribute)
                    and isinstance(node.func.value, ast.Name)
                    and node.func.value.id in ("self", "cls")
                    and node.func.attr == current_fn
                )
            ):
                is_direct_recursion = True

            if is_direct_recursion:
                self._add_finding(
                    node,
                    "CWE-674",
                    f"Direct recursion detected in function '{current_fn}'",
                    severity="warning",
                )

        callee = self._resolve_callee(node.func)

        # Attribute-based checks for methods on instances (e.g. s.bind, s.connect)
        if isinstance(node.func, ast.Attribute):
            attr = node.func.attr
            if attr in ("bind", "listen"):
                self.capabilities.add(CAP_NET_SERVER)
            elif attr in ("connect", "create_connection"):
                self.capabilities.add(CAP_NET_CLIENT)
            elif attr in ("read_bytes", "read_text"):
                self.capabilities.add(CAP_FS_READ)
            elif attr in ("write_bytes", "write_text"):
                self.capabilities.add(CAP_FS_WRITE)

        if callee:
            # Process execution / CWE-78
            if (
                callee.startswith("subprocess.")
                or callee in ("os.system", "os.popen", "os.posix_spawn", "os.fork")
                or callee.startswith("os.spawn")
                or callee.startswith("os.exec")
            ):
                self.capabilities.add(CAP_PROCESS_EXEC)
                self._add_finding(
                    node,
                    "CWE-78",
                    f"Dangerous OS process execution sink: {callee}",
                    capability=CAP_PROCESS_EXEC,
                    severity="error",
                )

            # Network client / server
            elif callee in (
                "socket.connect",
                "socket.create_connection",
                "socket.socket",
            ):
                self.capabilities.add(CAP_NET_CLIENT)
            elif callee in ("socket.bind", "socket.listen"):
                self.capabilities.add(CAP_NET_SERVER)
            elif callee.startswith("urllib.request.") or callee.startswith(
                "http.client."
            ):
                self.capabilities.add(CAP_NET_CLIENT)
            elif callee.startswith("socketserver.") or callee.startswith(
                "http.server."
            ):
                self.capabilities.add(CAP_NET_SERVER)

            # FFI unsafe
            elif callee.startswith("ctypes."):
                self.capabilities.add(CAP_FFI_UNSAFE)

            # Dynamic code evaluation / CWE-94
            elif callee in ("eval", "exec", "compile"):
                self.capabilities.add(CAP_DYNAMIC_EVAL)
                self._add_finding(
                    node,
                    "CWE-94",
                    f"Dangerous dynamic code evaluation sink: {callee}",
                    capability=CAP_DYNAMIC_EVAL,
                    severity="error",
                )
            elif (
                callee.startswith("pickle.")
                or callee.startswith("_pickle.")
                or callee.startswith("shelve.")
                or callee == "yaml.load"
            ):
                self.capabilities.add(CAP_DYNAMIC_EVAL)
                self._add_finding(
                    node,
                    "CWE-94",
                    f"Dangerous object deserialization sink: {callee}",
                    capability=CAP_DYNAMIC_EVAL,
                    severity="error",
                )

            # Regex / CWE-1333
            elif callee.startswith("re."):
                self.capabilities.add(CAP_REGEX_RE)
                self._add_finding(
                    node,
                    "CWE-1333",
                    f"Flagged regex execution / compilation: {callee}",
                    capability=CAP_REGEX_RE,
                    severity="warning",
                )

            # File read / write and CWE-22 path traversal
            elif callee in ("open", "io.open") or callee.endswith(".open"):
                self._check_file_open(node, callee)

        return self.generic_visit(node)

    def _check_file_open(self, node: ast.Call, callee: str) -> None:
        """Analyze open() mode and path for capabilities and CWE-22."""
        mode_val = "r"
        if len(node.args) >= 2:
            mode_arg = node.args[1]
            if isinstance(mode_arg, ast.Constant) and isinstance(mode_arg.value, str):
                mode_val = mode_arg.value
            else:
                mode_val = "rw"
        for kw in node.keywords:
            if kw.arg == "mode":
                if isinstance(kw.value, ast.Constant) and isinstance(
                    kw.value.value, str
                ):
                    mode_val = kw.value.value
                else:
                    mode_val = "rw"

        is_write = any(c in mode_val for c in ("w", "a", "x", "+"))
        is_read = "r" in mode_val or "+" in mode_val

        if is_write:
            self.capabilities.add(CAP_FS_WRITE)
        if is_read or not is_write:
            self.capabilities.add(CAP_FS_READ)

        # Path traversal check (CWE-22)
        if node.args:
            path_arg = node.args[0]
            if isinstance(path_arg, ast.Constant) and isinstance(path_arg.value, str):
                if ".." in path_arg.value:
                    msg = f"Path traversal ('..') detected: '{path_arg.value}'"
                    self._add_finding(
                        node,
                        "CWE-22",
                        msg,
                        capability=CAP_FS_READ if is_read else CAP_FS_WRITE,
                        severity="error",
                    )
            elif isinstance(path_arg, (ast.BinOp, ast.JoinedStr)):
                self._add_finding(
                    node,
                    "CWE-22",
                    "Unvalidated dynamic file path in file open",
                    capability=CAP_FS_READ if is_read else CAP_FS_WRITE,
                    severity="warning",
                )


def scan_source(source_text: str, filename: str = "<stdin>") -> FileScanResult:
    """Parse and scan Python source code string for capabilities and CWEs."""
    encoded = source_text.encode("utf-8")
    if len(encoded) > MAX_FILE_BYTES:
        msg = (
            f"Source size ({len(encoded)} bytes) exceeds "
            f"MAX_FILE_BYTES ({MAX_FILE_BYTES})"
        )
        finding = Finding(
            file_path=filename,
            line=1,
            col=0,
            cwe="CWE-400",
            message=msg,
            severity="error",
        )
        return FileScanResult(
            filename=filename,
            capabilities=frozenset(),
            findings=(finding,),
            nodes_visited=0,
        )

    try:
        tree = ast.parse(source_text, filename=filename)
        scanner = _ASTScanner(filename)
        scanner.visit(tree)
        return FileScanResult(
            filename=filename,
            capabilities=frozenset(scanner.capabilities),
            findings=tuple(scanner.findings),
            nodes_visited=scanner.nodes_visited,
        )
    except (RecursionError, MemoryError) as err:
        msg = f"Resource exhaustion during AST analysis: {type(err).__name__}: {err}"
        finding = Finding(
            file_path=filename,
            line=1,
            col=0,
            cwe="CWE-400",
            message=msg,
            severity="error",
        )
        return FileScanResult(
            filename=filename,
            capabilities=frozenset(),
            findings=(finding,),
            nodes_visited=0,
        )
