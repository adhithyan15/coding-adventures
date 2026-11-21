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
    CAP_ENV_READ,
    CAP_ENV_WRITE,
    CAP_FFI_UNSAFE,
    CAP_FS_READ,
    CAP_FS_WRITE,
    CAP_NET_CLIENT,
    CAP_NET_SERVER,
    CAP_PROCESS_EXEC,
    CAP_RAND_READ,
    CAP_REGEX_RE,
    CAP_THREAD_SPAWN,
    CAP_TIME_READ,
    CAP_TIME_SLEEP,
    MAX_AST_NODES,
    MAX_FILE_BYTES,
    Finding,
)

_OS_FS_READ_FUNCS = frozenset(
    {
        "os.listdir",
        "os.scandir",
        "os.walk",
        "os.stat",
        "os.lstat",
        "os.readlink",
        "os.path.exists",
        "os.path.isfile",
        "os.path.isdir",
        "os.path.islink",
        "os.path.realpath",
    }
)

_OS_FS_WRITE_FUNCS = frozenset(
    {
        "os.mkdir",
        "os.makedirs",
        "os.remove",
        "os.unlink",
        "os.rmdir",
        "os.rename",
        "os.replace",
        "os.chmod",
        "os.chown",
        "os.symlink",
        "os.link",
        "os.truncate",
    }
)

_PATH_READ_METHODS = frozenset(
    {
        "read_bytes",
        "read_text",
        "iterdir",
        "rglob",
        "is_file",
        "is_dir",
        "is_symlink",
        "lstat",
    }
)

_PATH_WRITE_METHODS = frozenset(
    {
        "write_bytes",
        "write_text",
        "mkdir",
        "rmdir",
        "unlink",
        "touch",
    }
)

_AMBIGUOUS_PATH_READ_METHODS = frozenset({"exists", "stat", "resolve", "glob"})
_AMBIGUOUS_PATH_WRITE_METHODS = frozenset({"replace", "rename", "chmod"})

_DEPTH_GUARD_TOKENS = ("depth", "budget", "limit", "quota", "step")


def _has_parent_traversal(raw_path: str) -> bool:
    """Return True if raw_path contains a '..' directory traversal segment."""
    return ".." in raw_path.replace("\\", "/").split("/")


def _has_depth_guard(args: ast.arguments) -> bool:
    """Return True if any parameter name implies an explicit recursion bound."""
    all_args = [
        *getattr(args, "posonlyargs", ()),
        *args.args,
        *args.kwonlyargs,
    ]
    return any(
        any(tok in arg.arg.lower() for tok in _DEPTH_GUARD_TOKENS)
        for arg in all_args
    )


def _os_open_flags_modes(
    flags_node: ast.AST, imports: dict[str, str]
) -> tuple[bool, bool]:
    """Infer (is_read, is_write) from os.open flags AST; defaults to (True, True)."""
    write_flags = {
        "os.O_WRONLY",
        "os.O_RDWR",
        "os.O_CREAT",
        "os.O_TRUNC",
        "os.O_APPEND",
    }
    read_flags = {"os.O_RDONLY", "os.O_RDWR"}
    names: set[str] = set()
    stack = [flags_node]
    while stack:
        cur = stack.pop()
        if isinstance(cur, ast.BinOp) and isinstance(cur.op, ast.BitOr):
            stack.append(cur.left)
            stack.append(cur.right)
        elif isinstance(cur, ast.Attribute) and isinstance(cur.value, ast.Name):
            base = imports.get(cur.value.id, cur.value.id)
            names.add(f"{base}.{cur.attr}")
        elif isinstance(cur, ast.Name):
            names.add(imports.get(cur.id, cur.id))
        elif (
            isinstance(cur, ast.Call)
            and isinstance(cur.func, ast.Name)
            and cur.func.id == "getattr"
            and len(cur.args) >= 2
            and isinstance(cur.args[0], ast.Name)
            and imports.get(cur.args[0].id, cur.args[0].id) == "os"
            and isinstance(cur.args[1], ast.Constant)
            and isinstance(cur.args[1].value, str)
        ):
            names.add(f"os.{cur.args[1].value}")
        else:
            return (True, True)
    if not names:
        return (True, True)
    has_write = any(n in write_flags for n in names)
    has_read = any(n in read_flags for n in names) or not has_write
    return (has_read, has_write)


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
        self.path_vars: set[str] = set()
        self.enclosing_functions: list[tuple[str, bool]] = []

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

    def _classify_module_import(self, mod: str, node: ast.AST) -> None:
        """Record capabilities implied by importing module `mod`."""
        if mod == "re" or mod.startswith("re."):
            self.capabilities.add(CAP_REGEX_RE)
            self._add_finding(
                node,
                "CWE-1333",
                "Flagged `re` module usage / regex compilation",
                capability=CAP_REGEX_RE,
                severity="warning",
            )
        elif mod in ("ctypes", "cffi") or mod.startswith(("ctypes.", "cffi.")):
            self.capabilities.add(CAP_FFI_UNSAFE)
        elif mod in (
            "socket",
            "socketserver",
            "http.client",
            "urllib",
            "urllib.request",
        ):
            self.capabilities.add(CAP_NET_CLIENT)
        elif mod in ("subprocess", "posix_spawn") or mod.startswith("subprocess."):
            self.capabilities.add(CAP_PROCESS_EXEC)
        elif mod in ("pickle", "_pickle", "shelve", "marshal"):
            self.capabilities.add(CAP_DYNAMIC_EVAL)
        elif mod in ("random", "secrets") or mod.startswith(("random.", "secrets.")):
            self.capabilities.add(CAP_RAND_READ)
        elif mod in (
            "threading",
            "multiprocessing",
            "concurrent.futures",
        ) or mod.startswith(("threading.", "multiprocessing.", "concurrent.futures.")):
            self.capabilities.add(CAP_THREAD_SPAWN)

    def visit_Import(self, node: ast.Import) -> Any:
        for alias in node.names:
            name = alias.name
            asname = alias.asname or name
            self.imports[asname] = name
            self._classify_module_import(name, node)
        return self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> Any:
        module = node.module or ""
        for alias in node.names:
            full_name = f"{module}.{alias.name}" if module else alias.name
            asname = alias.asname or alias.name
            self.imports[asname] = full_name
            self._classify_module_import(module or full_name, node)
        return self.generic_visit(node)

    def _is_path_receiver(self, node: ast.AST) -> bool:
        stack = [node]
        steps = 0
        while stack and steps < 64:
            cur = stack.pop()
            steps += 1
            if isinstance(cur, ast.Name) and cur.id in self.path_vars:
                return True
            if isinstance(cur, ast.Call) and self._resolve_callee(cur.func) in (
                "pathlib.Path",
                "Path",
            ):
                return True
            if isinstance(cur, ast.BinOp) and isinstance(cur.op, ast.Div):
                stack.append(cur.left)
                stack.append(cur.right)
        return False

    @staticmethod
    def _annotation_has_path(ann: ast.AST | None) -> bool:
        if ann is None:
            return False
        for node in ast.walk(ann):
            if (isinstance(node, ast.Name) and node.id == "Path") or (
                isinstance(node, ast.Attribute) and node.attr == "Path"
            ):
                return True
        return False

    def _record_path_annotations(self, args: ast.arguments) -> None:
        all_args = [
            *getattr(args, "posonlyargs", ()),
            *args.args,
            *args.kwonlyargs,
        ]
        for arg in all_args:
            if self._annotation_has_path(arg.annotation):
                self.path_vars.add(arg.arg)

    def _bind_target_value(self, target: ast.AST, value: ast.AST | None) -> None:
        if not isinstance(target, ast.Name) or value is None:
            return
        if self._is_path_receiver(value):
            self.path_vars.add(target.id)
        if (
            isinstance(value, ast.Call)
            and self._resolve_callee(value.func) == "importlib.import_module"
            and value.args
            and isinstance(value.args[0], ast.Constant)
            and isinstance(value.args[0].value, str)
        ):
            self.imports[target.id] = value.args[0].value

    def visit_Assign(self, node: ast.Assign) -> Any:
        for target in node.targets:
            self._bind_target_value(target, node.value)
        return self.generic_visit(node)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> Any:
        if isinstance(node.target, ast.Name) and self._annotation_has_path(
            node.annotation
        ):
            self.path_vars.add(node.target.id)
        self._bind_target_value(node.target, node.value)
        return self.generic_visit(node)

    def visit_FunctionDef(self, node: ast.FunctionDef) -> Any:
        self._record_path_annotations(node.args)
        self.enclosing_functions.append((node.name, _has_depth_guard(node.args)))
        self.generic_visit(node)
        self.enclosing_functions.pop()

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> Any:
        self._record_path_annotations(node.args)
        self.enclosing_functions.append((node.name, _has_depth_guard(node.args)))
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

    def visit_Attribute(self, node: ast.Attribute) -> Any:
        resolved = self._resolve_callee(node)
        if resolved == "os.environ":
            if isinstance(node.ctx, ast.Store):
                self.capabilities.add(CAP_ENV_WRITE)
            else:
                self.capabilities.add(CAP_ENV_READ)
        return self.generic_visit(node)

    def visit_Subscript(self, node: ast.Subscript) -> Any:
        if (
            isinstance(node.ctx, ast.Store)
            and self._resolve_callee(node.value) == "os.environ"
        ):
            self.capabilities.add(CAP_ENV_WRITE)
        return self.generic_visit(node)

    def visit_Call(self, node: ast.Call) -> Any:
        # CWE-674: Precise direct recursion detection in AST call graph
        if self.enclosing_functions:
            current_fn, has_guard = self.enclosing_functions[-1]
            is_direct_recursion = (
                isinstance(node.func, ast.Name) and node.func.id == current_fn
            ) or (
                isinstance(node.func, ast.Attribute)
                and isinstance(node.func.value, ast.Name)
                and node.func.value.id in ("self", "cls")
                and node.func.attr == current_fn
            )

            if is_direct_recursion and not has_guard:
                self._add_finding(
                    node,
                    "CWE-674",
                    f"Direct recursion detected in function '{current_fn}'",
                    severity="warning",
                )

        callee = self._resolve_callee(node.func)

        # Attribute-based checks for methods on instances (e.g. s.bind, Path.read_text)
        if isinstance(node.func, ast.Attribute):
            attr = node.func.attr
            if attr in ("bind", "listen"):
                self.capabilities.add(CAP_NET_SERVER)
            elif attr in ("connect", "create_connection"):
                self.capabilities.add(CAP_NET_CLIENT)
            elif attr in _PATH_READ_METHODS or (
                attr in _AMBIGUOUS_PATH_READ_METHODS
                and self._is_path_receiver(node.func.value)
            ):
                self.capabilities.add(CAP_FS_READ)
            elif attr in _PATH_WRITE_METHODS or (
                attr in _AMBIGUOUS_PATH_WRITE_METHODS
                and self._is_path_receiver(node.func.value)
            ):
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
            elif callee.startswith(("ctypes.", "cffi.")):
                self.capabilities.add(CAP_FFI_UNSAFE)

            # Environment read/write
            elif callee == "os.getenv":
                self.capabilities.add(CAP_ENV_READ)
            elif callee in ("os.putenv", "os.unsetenv"):
                self.capabilities.add(CAP_ENV_WRITE)

            # Clock / sleep
            elif callee in (
                "time.time",
                "time.monotonic",
                "time.perf_counter",
                "time.process_time",
                "datetime.now",
                "datetime.utcnow",
                "datetime.today",
                "datetime.datetime.now",
                "datetime.datetime.utcnow",
                "datetime.datetime.today",
            ):
                self.capabilities.add(CAP_TIME_READ)
            elif callee == "time.sleep":
                self.capabilities.add(CAP_TIME_SLEEP)

            # Random / entropy
            elif callee in (
                "os.urandom",
                "uuid.uuid4",
                "uuid.uuid1",
            ) or callee.startswith(("random.", "secrets.")):
                self.capabilities.add(CAP_RAND_READ)

            # Dynamic code evaluation / CWE-94
            elif callee in ("eval", "exec", "compile", "__import__"):
                self.capabilities.add(CAP_DYNAMIC_EVAL)
                self._add_finding(
                    node,
                    "CWE-94",
                    f"Dangerous dynamic code evaluation sink: {callee}",
                    capability=CAP_DYNAMIC_EVAL,
                    severity="error",
                )
            elif callee == "importlib.import_module":
                first_arg = node.args[0] if node.args else None
                if isinstance(first_arg, ast.Constant) and isinstance(
                    first_arg.value, str
                ):
                    self._classify_module_import(first_arg.value, node)
                else:
                    self.capabilities.add(CAP_DYNAMIC_EVAL)
                    self._add_finding(
                        node,
                        "CWE-94",
                        f"Dynamic module import sink: {callee}",
                        capability=CAP_DYNAMIC_EVAL,
                        severity="error",
                    )
            elif (
                callee.startswith("pickle.")
                or callee.startswith("_pickle.")
                or callee.startswith("shelve.")
                or callee.startswith("marshal.")
                or callee in ("yaml.load", "yaml.unsafe_load", "yaml.full_load")
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
            elif callee == "os.open":
                self._check_os_open(node)
            elif callee in ("open", "io.open") or callee.endswith(".open"):
                self._check_file_open(node, callee)
            elif callee in _OS_FS_READ_FUNCS:
                self.capabilities.add(CAP_FS_READ)
                self._check_path_arg(node, CAP_FS_READ)
            elif callee in _OS_FS_WRITE_FUNCS or callee.startswith("shutil."):
                self.capabilities.add(CAP_FS_WRITE)
                self._check_path_arg(node, CAP_FS_WRITE)
            elif callee in ("pathlib.Path", "Path"):
                self._check_path_arg(node, CAP_FS_READ)

        return self.generic_visit(node)

    def _check_path_arg(self, node: ast.Call, cap: str) -> None:
        """Check first argument of filesystem call for CWE-22 traversal patterns."""
        if not node.args:
            return
        path_arg = node.args[0]
        if isinstance(path_arg, ast.Constant) and isinstance(path_arg.value, str):
            if _has_parent_traversal(path_arg.value):
                self._add_finding(
                    node,
                    "CWE-22",
                    f"Path traversal ('..') detected: '{path_arg.value}'",
                    capability=cap,
                    severity="error",
                )
        elif isinstance(path_arg, (ast.BinOp, ast.JoinedStr)):
            self._add_finding(
                node,
                "CWE-22",
                "Unvalidated dynamic file path in file operation",
                capability=cap,
                severity="warning",
            )

    def _check_os_open(self, node: ast.Call) -> None:
        """Infer read/write capabilities and CWE-22 for os.open(path, flags)."""
        if len(node.args) >= 2:
            is_read, is_write = _os_open_flags_modes(node.args[1], self.imports)
        else:
            is_read, is_write = (True, True)
        if is_write:
            self.capabilities.add(CAP_FS_WRITE)
        if is_read or not is_write:
            self.capabilities.add(CAP_FS_READ)
        self._check_path_arg(node, CAP_FS_READ if is_read else CAP_FS_WRITE)

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

        self._check_path_arg(node, CAP_FS_READ if is_read else CAP_FS_WRITE)


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
