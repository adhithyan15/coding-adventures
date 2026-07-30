#!/usr/bin/env python3
"""Load an approved Linux OCI backend without ambient Python authority.

The parent process receives already-authorized exact bytes.  It seals those
bytes in anonymous Linux files and starts the sealed copy of this loader in a
fresh isolated Python worker.  The worker verifies the closed stdlib import
manifest and required interfaces, but never invokes preflight or Podman.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import selectors
import signal
import subprocess  # nosec B404
import sys
import time
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    import fcntl
except ImportError:  # pragma: no cover - exercised by the platform guard
    fcntl = None  # type: ignore[assignment]

MAX_COMPONENT_BYTES = 16_777_216
MAX_WORKER_OUTPUT_BYTES = 65_536
WORKER_TIMEOUT_SECONDS = 10.0
MODULE_NAME = "build_tool_conformance_linux_oci"
REQUIRED_EXPORTS = (
    "CommandResult",
    "LinuxOciUnavailable",
    "preflight_brokered",
    "preflight_prevalidated",
)
FORBIDDEN_DYNAMIC_IMPORTS = {"__import__", "import_module"}
RESERVED_SAFE_BINDINGS = {"Path", "dataclass", "frozenset", "__name__"}


class LoaderUnavailable(RuntimeError):
    """A stable fail-closed exact-loader error."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


@dataclass(frozen=True)
class ImportManifest:
    """Closed import and interface contract for the approved backend."""

    module: str
    imports: tuple[str, ...]
    required_exports: tuple[str, ...]


def _strict_json(raw: bytes, *, code: str) -> dict[str, Any]:
    if not raw or len(raw) > MAX_COMPONENT_BYTES:
        raise LoaderUnavailable(code, "loader document exceeds its byte ceiling")
    try:
        text = raw.decode("utf-8", errors="strict")

        def reject_duplicates(
            pairs: list[tuple[str, object]],
        ) -> dict[str, object]:
            value: dict[str, object] = {}
            for key, item in pairs:
                if key in value:
                    raise ValueError("duplicate key")
                value[key] = item
            return value

        value = json.loads(text, object_pairs_hook=reject_duplicates)
    except (UnicodeDecodeError, ValueError) as error:
        raise LoaderUnavailable(code, "loader document is not strict JSON") from error
    if not isinstance(value, dict):
        raise LoaderUnavailable(code, "loader document must be an object")
    return value


def parse_import_manifest(raw: bytes) -> ImportManifest:
    """Parse the exact closed import manifest."""

    value = _strict_json(raw, code="LOADER_IMPORT_MANIFEST_INVALID")
    if set(value) != {
        "schema_version",
        "module",
        "imports",
        "required_exports",
    }:
        raise LoaderUnavailable(
            "LOADER_IMPORT_MANIFEST_INVALID",
            "import manifest fields are not the closed v1 profile",
        )
    imports = value.get("imports")
    exports = value.get("required_exports")
    valid_imports = (
        isinstance(imports, list)
        and all(
            isinstance(item, str)
            and item
            and item == item.strip()
            and all(part.isidentifier() for part in item.split("."))
            for item in imports
        )
        and len(imports) == len(set(imports))
        and imports == sorted(imports)
        and all(
            item.split(".", 1)[0] in sys.stdlib_module_names or item == "__future__"
            for item in imports
        )
    )
    if (
        value.get("schema_version") != 1
        or value.get("module") != MODULE_NAME
        or not valid_imports
        or exports != list(REQUIRED_EXPORTS)
    ):
        raise LoaderUnavailable(
            "LOADER_IMPORT_MANIFEST_INVALID",
            "import manifest does not match the closed backend profile",
        )
    return ImportManifest(
        module=MODULE_NAME,
        imports=tuple(item for item in imports if isinstance(item, str)),
        required_exports=REQUIRED_EXPORTS,
    )


def _parsed_source(raw: bytes) -> ast.Module:
    if not raw or len(raw) > MAX_COMPONENT_BYTES:
        raise LoaderUnavailable(
            "LOADER_BACKEND_SOURCE_INVALID",
            "backend source exceeds its byte ceiling",
        )
    try:
        text = raw.decode("utf-8", errors="strict")
        return ast.parse(text, filename="<approved-linux-oci-backend>")
    except (SyntaxError, UnicodeDecodeError) as error:
        raise LoaderUnavailable(
            "LOADER_BACKEND_SOURCE_INVALID",
            "backend source is not valid strict UTF-8 Python",
        ) from error


def source_imports(raw: bytes) -> frozenset[str]:
    """Return every statically declared import in exact backend source."""

    imports: set[str] = set()
    for node in ast.walk(_parsed_source(raw)):
        if isinstance(node, ast.Import):
            imports.update(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom):
            if node.level != 0 or node.module is None:
                raise LoaderUnavailable(
                    "LOADER_RELATIVE_IMPORT_FORBIDDEN",
                    "relative backend imports are forbidden",
                )
            if any(alias.name == "*" for alias in node.names):
                raise LoaderUnavailable(
                    "LOADER_WILDCARD_IMPORT_FORBIDDEN",
                    "wildcard backend imports are forbidden",
                )
            imports.add(node.module)
    return frozenset(imports)


def _call_is_safe_assignment(node: ast.Call) -> bool:
    if not isinstance(node.func, ast.Name) or node.keywords:
        return False
    if node.func.id == "Path":
        return (
            len(node.args) == 1
            and isinstance(node.args[0], ast.Constant)
            and isinstance(node.args[0].value, str)
        )
    if node.func.id == "frozenset" and len(node.args) == 1:
        value = node.args[0]
        return isinstance(value, (ast.Set, ast.Tuple, ast.List)) and all(
            isinstance(item, ast.Constant) for item in value.elts
        )
    return False


def _validate_static_expression(node: ast.expr) -> None:
    forbidden = (
        ast.Await,
        ast.DictComp,
        ast.GeneratorExp,
        ast.Lambda,
        ast.ListComp,
        ast.NamedExpr,
        ast.SetComp,
        ast.Yield,
        ast.YieldFrom,
    )
    for child in ast.walk(node):
        if isinstance(child, forbidden) or (
            isinstance(child, ast.Call) and not _call_is_safe_assignment(child)
        ):
            raise LoaderUnavailable(
                "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
                "backend import-time executable code is forbidden",
            )


def _validate_function_header(node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
    if node.decorator_list:
        raise LoaderUnavailable(
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
            "backend function decorators are forbidden",
        )
    defaults = [*node.args.defaults, *node.args.kw_defaults]
    for default in defaults:
        if default is not None:
            _validate_static_expression(default)


def _is_frozen_dataclass_decorator(node: ast.expr) -> bool:
    return (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Name)
        and node.func.id == "dataclass"
        and not node.args
        and len(node.keywords) == 1
        and node.keywords[0].arg == "frozen"
        and isinstance(node.keywords[0].value, ast.Constant)
        and node.keywords[0].value.value is True
    )


def _validate_class_shape(node: ast.ClassDef) -> None:
    if node.keywords or any(
        not isinstance(base, ast.Name) or base.id != "RuntimeError"
        for base in node.bases
    ):
        raise LoaderUnavailable(
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
            "backend class construction is outside the closed profile",
        )
    if node.decorator_list:
        valid_dataclass = len(
            node.decorator_list
        ) == 1 and _is_frozen_dataclass_decorator(node.decorator_list[0])
        if not valid_dataclass:
            raise LoaderUnavailable(
                "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
                "backend class decorators are outside the closed profile",
            )
    for statement in node.body:
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)):
            _validate_function_header(statement)
        elif isinstance(statement, ast.Expr) and isinstance(
            statement.value,
            ast.Constant,
        ):
            continue
        elif isinstance(statement, ast.AnnAssign):
            if statement.value is not None:
                _validate_static_expression(statement.value)
        elif isinstance(statement, ast.Pass):
            continue
        else:
            raise LoaderUnavailable(
                "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
                "backend class body contains executable import-time code",
            )


def _is_main_guard(node: ast.If) -> bool:
    return (
        isinstance(node.test, ast.Compare)
        and isinstance(node.test.left, ast.Name)
        and node.test.left.id == "__name__"
        and len(node.test.ops) == 1
        and isinstance(node.test.ops[0], ast.Eq)
        and len(node.test.comparators) == 1
        and isinstance(node.test.comparators[0], ast.Constant)
        and node.test.comparators[0].value == "__main__"
        and not node.orelse
    )


def _validate_import_time_shape(tree: ast.Module) -> None:
    imported_bindings: dict[str, tuple[str, str | None]] = {}
    for statement in tree.body:
        if isinstance(statement, ast.Import):
            for alias in statement.names:
                imported_bindings[alias.asname or alias.name.split(".")[0]] = (
                    alias.name,
                    None,
                )
        elif isinstance(statement, ast.ImportFrom) and statement.module is not None:
            for alias in statement.names:
                imported_bindings[alias.asname or alias.name] = (
                    statement.module,
                    alias.name,
                )
    calls = [
        node.func.id
        for node in ast.walk(tree)
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name)
    ]
    if ("Path" in calls and imported_bindings.get("Path") != ("pathlib", "Path")) or (
        "dataclass" in calls
        and imported_bindings.get("dataclass") != ("dataclasses", "dataclass")
    ):
        raise LoaderUnavailable(
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
            "backend safe constructors do not have their fixed bindings",
        )
    if "frozenset" in imported_bindings:
        raise LoaderUnavailable(
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
            "backend may not shadow its safe builtin constructor",
        )

    for statement in tree.body:
        if isinstance(statement, (ast.Import, ast.ImportFrom)):
            continue
        if isinstance(statement, ast.Expr) and isinstance(
            statement.value,
            ast.Constant,
        ):
            continue
        if isinstance(statement, ast.Assign):
            if not all(
                isinstance(target, ast.Name) and target.id not in RESERVED_SAFE_BINDINGS
                for target in statement.targets
            ):
                raise LoaderUnavailable(
                    "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
                    "backend assignment target is outside the closed profile",
                )
            _validate_static_expression(statement.value)
            continue
        if isinstance(statement, ast.AnnAssign):
            if (
                not isinstance(statement.target, ast.Name)
                or statement.target.id in RESERVED_SAFE_BINDINGS
            ):
                raise LoaderUnavailable(
                    "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
                    "backend assignment target is outside the closed profile",
                )
            if statement.value is not None:
                _validate_static_expression(statement.value)
            continue
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)):
            _validate_function_header(statement)
            continue
        if isinstance(statement, ast.ClassDef):
            _validate_class_shape(statement)
            continue
        if isinstance(statement, ast.If) and _is_main_guard(statement):
            continue
        raise LoaderUnavailable(
            "LOADER_IMPORT_TIME_CODE_FORBIDDEN",
            "backend contains executable import-time statements",
        )


def validate_source_closure(raw: bytes, manifest: ImportManifest) -> None:
    """Require exact static imports and reject dynamic import entry points."""

    tree = _parsed_source(raw)
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            function = node.func
            name = function.id if isinstance(function, ast.Name) else None
            attribute = function.attr if isinstance(function, ast.Attribute) else None
            if name in FORBIDDEN_DYNAMIC_IMPORTS or attribute in (
                FORBIDDEN_DYNAMIC_IMPORTS
            ):
                raise LoaderUnavailable(
                    "LOADER_DYNAMIC_IMPORT_FORBIDDEN",
                    "dynamic backend imports are forbidden",
                )
    _validate_import_time_shape(tree)
    if source_imports(raw) != frozenset(manifest.imports):
        raise LoaderUnavailable(
            "LOADER_IMPORT_CLOSURE_MISMATCH",
            "backend imports do not match the exact approved manifest",
        )


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _sealed_memfd(name: str, raw: bytes) -> int:
    """Copy exact bytes into one write-sealed anonymous Linux file."""

    if (
        not sys.platform.startswith("linux")
        or not hasattr(os, "memfd_create")
        or fcntl is None
    ):
        raise LoaderUnavailable(
            "LOADER_PLATFORM_UNSUPPORTED",
            "sealed exact loading requires Linux memfd support",
        )
    if not raw or len(raw) > MAX_COMPONENT_BYTES:
        raise LoaderUnavailable(
            "LOADER_COMPONENT_SIZE_INVALID",
            "loader component exceeds its byte ceiling",
        )
    descriptor = os.memfd_create(
        name,
        flags=os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING,
    )
    try:
        offset = 0
        while offset < len(raw):
            offset += os.write(descriptor, raw[offset:])
        os.fsync(descriptor)
        seals = (
            fcntl.F_SEAL_SEAL
            | fcntl.F_SEAL_SHRINK
            | fcntl.F_SEAL_GROW
            | fcntl.F_SEAL_WRITE
        )
        fcntl.fcntl(descriptor, fcntl.F_ADD_SEALS, seals)
        actual = fcntl.fcntl(descriptor, fcntl.F_GET_SEALS)
        if actual & seals != seals:
            raise LoaderUnavailable(
                "LOADER_SEAL_FAILED",
                "loader component could not be fully sealed",
            )
        os.lseek(descriptor, 0, os.SEEK_SET)
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


def _read_fd(descriptor: int) -> bytes:
    status = os.fstat(descriptor)
    if status.st_size <= 0 or status.st_size > MAX_COMPONENT_BYTES:
        raise LoaderUnavailable(
            "LOADER_COMPONENT_SIZE_INVALID",
            "sealed loader component has an invalid size",
        )
    raw = os.pread(descriptor, status.st_size + 1, 0)
    if len(raw) != status.st_size:
        raise LoaderUnavailable(
            "LOADER_COMPONENT_CHANGED",
            "sealed loader component did not retain its exact size",
        )
    return raw


def _validate_backend_structure(
    source: bytes,
    manifest: ImportManifest,
) -> None:
    """Compile and inspect exact source without executing backend code."""

    validate_source_closure(source, manifest)
    tree = _parsed_source(source)
    try:
        compile(
            source.decode("utf-8"),
            "<sealed-linux-oci-backend>",
            "exec",
            dont_inherit=True,
        )
    except (SyntaxError, UnicodeDecodeError) as error:
        raise LoaderUnavailable(
            "LOADER_BACKEND_COMPILE_FAILED",
            "approved backend failed isolated compilation",
        ) from error

    class_nodes = [node for node in tree.body if isinstance(node, ast.ClassDef)]
    function_nodes = [node for node in tree.body if isinstance(node, ast.FunctionDef)]
    classes = {node.name: node for node in class_nodes}
    functions = {node.name: node for node in function_nodes}
    def valid_preflight_signature(preflight: ast.FunctionDef | None) -> bool:
        return bool(
            preflight is not None
            and [argument.arg for argument in preflight.args.posonlyargs] == []
            and [argument.arg for argument in preflight.args.args] == ["identity"]
            and preflight.args.vararg is None
            and [argument.arg for argument in preflight.args.kwonlyargs]
            == [
                "runtime_info",
                "image_inspect",
                "platform_name",
                "effective_uid",
            ]
            and preflight.args.kw_defaults[0] is None
            and preflight.args.kw_defaults[1] is None
            and isinstance(preflight.args.kw_defaults[2], ast.Constant)
            and preflight.args.kw_defaults[2].value is None
            and isinstance(preflight.args.kw_defaults[3], ast.Constant)
            and preflight.args.kw_defaults[3].value is None
            and preflight.args.kwarg is None
        )

    valid_brokered = valid_preflight_signature(functions.get("preflight_brokered"))
    valid_preflight = valid_preflight_signature(
        functions.get("preflight_prevalidated")
    )
    unavailable = classes.get("LinuxOciUnavailable")
    unavailable_init = (
        next(
            (
                statement
                for statement in unavailable.body
                if isinstance(statement, ast.FunctionDef)
                and statement.name == "__init__"
            ),
            None,
        )
        if unavailable is not None
        else None
    )
    assigned_error_fields = (
        {
            target.attr
            for statement in unavailable_init.body
            if isinstance(statement, ast.Assign)
            for target in statement.targets
            if isinstance(target, ast.Attribute)
            and isinstance(target.value, ast.Name)
            and target.value.id == "self"
        }
        if unavailable_init is not None
        else set()
    )
    valid_unavailable = (
        unavailable is not None
        and len(unavailable.bases) == 1
        and isinstance(unavailable.bases[0], ast.Name)
        and unavailable.bases[0].id == "RuntimeError"
        and unavailable_init is not None
        and [argument.arg for argument in unavailable_init.args.args]
        == ["self", "code", "message"]
        and not unavailable_init.args.posonlyargs
        and not unavailable_init.args.defaults
        and unavailable_init.args.vararg is None
        and not unavailable_init.args.kwonlyargs
        and unavailable_init.args.kwarg is None
        and assigned_error_fields == {"code", "message"}
    )
    command_result = classes.get("CommandResult")
    command_field_statements = (
        [
            statement
            for statement in command_result.body
            if isinstance(statement, ast.AnnAssign)
        ]
        if command_result is not None
        else []
    )
    command_fields = {
        statement.target.id: statement.annotation.id
        for statement in command_field_statements
        if isinstance(statement.target, ast.Name)
        and isinstance(statement.annotation, ast.Name)
    }
    valid_command_result = (
        command_result is not None
        and len(command_result.decorator_list) == 1
        and _is_frozen_dataclass_decorator(command_result.decorator_list[0])
        and len(command_field_statements) == 3
        and command_fields
        == {"returncode": "int", "stdout": "bytes", "stderr": "bytes"}
    )
    required_names = {
        "CommandResult",
        "LinuxOciUnavailable",
        "preflight_brokered",
        "preflight_prevalidated",
    }
    duplicate_required = any(
        sum(node.name == name for node in [*class_nodes, *function_nodes]) != 1
        for name in required_names
    )
    if (
        not valid_brokered
        or not valid_preflight
        or not valid_unavailable
        or not valid_command_result
        or duplicate_required
    ):
        raise LoaderUnavailable(
            "LOADER_BACKEND_INTERFACE_INVALID",
            "approved backend is missing a required structural interface",
        )


def _worker_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--isolated-worker", action="store_true", required=True)
    for name in ("loader", "backend", "manifest", "identity"):
        parser.add_argument(f"--{name}-fd", type=int, required=True)
        parser.add_argument(f"--{name}-sha256", required=True)
    return parser


def _isolated_worker(argv: Sequence[str]) -> int:
    arguments = _worker_parser().parse_args(argv)
    values: dict[str, bytes] = {}
    for name in ("loader", "backend", "manifest", "identity"):
        raw = _read_fd(getattr(arguments, f"{name}_fd"))
        if _sha256(raw) != getattr(arguments, f"{name}_sha256"):
            raise LoaderUnavailable(
                "LOADER_COMPONENT_DIGEST_MISMATCH",
                "sealed loader component has the wrong digest",
            )
        values[name] = raw
    manifest = parse_import_manifest(values["manifest"])
    identity = _strict_json(
        values["identity"],
        code="LOADER_IDENTITY_INVALID",
    )
    if (
        identity.get("backend_kind") != "linux_oci"
        or identity.get("platform") != "linux"
        or identity.get("architecture") != "amd64"
    ):
        raise LoaderUnavailable(
            "LOADER_IDENTITY_INVALID",
            "backend identity is outside the closed Linux amd64 profile",
        )
    _validate_backend_structure(values["backend"], manifest)
    print(
        json.dumps(
            {
                "schema_version": 1,
                "authorization_scope": "loadability-only",
                "backend_sha256": _sha256(values["backend"]),
                "identity_sha256": _sha256(values["identity"]),
                "loader_sha256": _sha256(values["loader"]),
                "manifest_sha256": _sha256(values["manifest"]),
                "status": "loadable",
                "conformance_status": "not-run",
            },
            sort_keys=True,
            separators=(",", ":"),
        )
    )
    return 0


def _terminate_worker(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def _read_worker_output(
    process: subprocess.Popen[bytes],
) -> tuple[bytes, bytes]:
    """Stream both worker pipes with one aggregate hard byte ceiling."""

    if process.stdout is None or process.stderr is None:
        raise LoaderUnavailable(
            "LOADER_WORKER_PROTOCOL_INVALID",
            "isolated loader worker pipes are unavailable",
        )
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ, "stdout")
    selector.register(process.stderr, selectors.EVENT_READ, "stderr")
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    total = 0
    deadline = time.monotonic() + WORKER_TIMEOUT_SECONDS
    try:
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                _terminate_worker(process)
                raise LoaderUnavailable(
                    "LOADER_WORKER_TIMEOUT",
                    "isolated loader worker exceeded its time ceiling",
                )
            events = selector.select(min(remaining, 0.1))
            for key, _mask in events:
                chunk = os.read(
                    key.fileobj.fileno(),
                    min(16_384, MAX_WORKER_OUTPUT_BYTES + 1 - total),
                )
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                buffers[key.data].extend(chunk)
                total += len(chunk)
                if total > MAX_WORKER_OUTPUT_BYTES:
                    _terminate_worker(process)
                    raise LoaderUnavailable(
                        "LOADER_WORKER_OUTPUT_LIMIT",
                        "isolated loader worker exceeded its output ceiling",
                    )
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            _terminate_worker(process)
            raise LoaderUnavailable(
                "LOADER_WORKER_TIMEOUT",
                "isolated loader worker exceeded its time ceiling",
            )
        process.wait(timeout=remaining)
        return bytes(buffers["stdout"]), bytes(buffers["stderr"])
    except subprocess.TimeoutExpired as error:
        _terminate_worker(process)
        raise LoaderUnavailable(
            "LOADER_WORKER_TIMEOUT",
            "isolated loader worker exceeded its time ceiling",
        ) from error
    finally:
        selector.close()


def validate_exact_backend(
    *,
    loader_source: bytes,
    backend_source: bytes,
    import_manifest: bytes,
    identity: bytes,
) -> dict[str, Any]:
    """Validate exact retained bytes in a fresh isolated one-shot worker."""

    if not sys.platform.startswith("linux"):
        raise LoaderUnavailable(
            "LOADER_PLATFORM_UNSUPPORTED",
            "exact backend loadability validation requires Linux",
        )
    manifest = parse_import_manifest(import_manifest)
    validate_source_closure(backend_source, manifest)
    components = {
        "loader": loader_source,
        "backend": backend_source,
        "manifest": import_manifest,
        "identity": identity,
    }
    descriptors: dict[str, int] = {}
    try:
        for name, raw in components.items():
            descriptors[name] = _sealed_memfd(f"build-tool-{name}", raw)
        command = [
            sys.executable,
            "-I",
            "-S",
            "-B",
            f"/proc/self/fd/{descriptors['loader']}",
            "--isolated-worker",
        ]
        for name, raw in components.items():
            command.extend(
                [
                    f"--{name}-fd",
                    str(descriptors[name]),
                    f"--{name}-sha256",
                    _sha256(raw),
                ]
            )
        environment = {
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "PATH": "/usr/bin:/bin",
            "PYTHONHASHSEED": "0",
            "TZ": "UTC",
        }
        process = subprocess.Popen(  # nosec B603
            command,
            cwd="/",
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            close_fds=True,
            pass_fds=tuple(descriptors.values()),
            start_new_session=True,
        )
        stdout, stderr = _read_worker_output(process)
        if process.returncode != 0:
            code = "LOADER_BACKEND_IMPORT_FAILED"
            try:
                error_value = json.loads(stderr.decode("utf-8"))
                if isinstance(error_value, dict) and isinstance(
                    error_value.get("code"),
                    str,
                ):
                    code = error_value["code"]
            except (UnicodeDecodeError, ValueError):
                pass
            raise LoaderUnavailable(
                code,
                "isolated loader worker rejected the approved backend",
            )
        receipt = _strict_json(stdout, code="LOADER_WORKER_RESPONSE_INVALID")
        if (
            receipt.get("status") != "loadable"
            or receipt.get("conformance_status") != "not-run"
            or receipt.get("authorization_scope") != "loadability-only"
        ):
            raise LoaderUnavailable(
                "LOADER_WORKER_RESPONSE_INVALID",
                "isolated loader worker returned an invalid receipt",
            )
        return receipt
    finally:
        for descriptor in descriptors.values():
            os.close(descriptor)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate one externally approved exact Linux backend loader."
    )
    parser.add_argument("--authority-bundle", type=Path, required=True)
    parser.add_argument("--approved-authority-sha256", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-tree", required=True)
    parser.add_argument("--repository-root", type=Path, required=True)
    return parser


def _parent_main(argv: Sequence[str]) -> int:
    arguments = _build_parser().parse_args(argv)
    import build_tool_conformance_authority as authority

    approved = authority.authorize_backend_loader(
        arguments.authority_bundle,
        approved_digest=arguments.approved_authority_sha256,
        expected_commit_oid=arguments.source_commit,
        expected_tree_oid=arguments.source_tree,
        repository_root=arguments.repository_root,
    )
    receipt = validate_exact_backend(
        loader_source=approved.components["preflight_loader"],
        backend_source=approved.components["linux_preflight_backend"],
        import_manifest=approved.components["preflight_import_manifest"],
        identity=approved.components["linux_backend_identity"],
    )
    receipt["authorization_scope"] = approved.bundle["authorization_scope"]
    receipt["authority_sha256"] = approved.bundle_digest
    receipt["source_commit"] = arguments.source_commit
    receipt["source_tree"] = arguments.source_tree
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":")))
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    arguments = list(sys.argv[1:] if argv is None else argv)
    try:
        if "--isolated-worker" in arguments:
            return _isolated_worker(arguments)
        return _parent_main(arguments)
    except LoaderUnavailable as error:
        print(
            json.dumps(
                {"code": error.code, "message": error.message, "status": "error"},
                sort_keys=True,
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1
    except Exception as error:
        code = getattr(error, "code", None)
        message = getattr(error, "message", None)
        if not isinstance(code, str) or not isinstance(message, str):
            raise
        print(
            json.dumps(
                {"code": code, "message": message, "status": "error"},
                sort_keys=True,
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
