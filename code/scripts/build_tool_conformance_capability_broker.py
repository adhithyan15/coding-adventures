#!/usr/bin/env python3
"""Run the two exact Linux OCI capability commands under separate authority.

The protected parent validates a thirteen-role external authority bundle,
loads the exact process-free backend itself, and passes only sealed broker,
identity, and behavior bytes to a fresh isolated worker. The worker retains
verified Podman, crun, Conmon, state-root, and delegated-cgroup descriptors.
It cannot decode a fixture, load caller Python, create a container, invoke an
adapter, or report conformance or trusted-execution readiness.
"""

from __future__ import annotations

import argparse
import base64
import ctypes
import errno
import hashlib
import json
import os
import selectors
import signal
import stat
import struct
import subprocess  # nosec B404
import sys
import time
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType, ModuleType
from typing import Any

try:
    import fcntl
except ImportError:  # pragma: no cover - exercised by the platform guard
    fcntl = None  # type: ignore[assignment]
try:
    import resource
except ImportError:  # pragma: no cover - exercised by the platform guard
    resource = None  # type: ignore[assignment]

MAX_COMPONENT_BYTES = 16_777_216
MAX_RUNTIME_BINARY_BYTES = 536_870_912
MAX_WORKER_OUTPUT_BYTES = 1_048_576
WORKER_TIMEOUT_SECONDS = 45.0
READ_CHUNK_BYTES = 65_536
PROC_FD_PREFIX = "/proc/self/fd"
STATE_CHILDREN = ("config", "home", "runtime", "runroot", "storage")
OPERATION_IDS = ("runtime_version", "image_inspect")
REQUIRED_CGROUP_CONTROLLERS = frozenset({"cpu", "memory", "pids"})
PR_SET_PDEATHSIG = 1
PR_SET_CHILD_SUBREAPER = 36
PR_SET_NO_NEW_PRIVS = 38
SECCOMP_SET_MODE_FILTER = 1
SECCOMP_RET_KILL_PROCESS = 0x80000000
SECCOMP_RET_ERRNO = 0x00050000
SECCOMP_RET_ALLOW = 0x7FFF0000
SECCOMP_DATA_NR_OFFSET = 0
SECCOMP_DATA_ARCH_OFFSET = 4
SECCOMP_DATA_ARGS_OFFSET = 16
AUDIT_ARCH_X86_64 = 0xC000003E
X32_SYSCALL_BIT = 0x40000000
BPF_LD_W_ABS = 0x20
BPF_JMP_JEQ_K = 0x15
BPF_JMP_JGE_K = 0x35
BPF_JMP_JSET_K = 0x45
BPF_RET_K = 0x06
PROT_EXEC = 0x4
SHM_EXEC = 0o100000
SYS_MMAP = 9
SYS_MPROTECT = 10
SYS_SHMAT = 30
SYS_RECVMSG = 47
SYS_USELIB = 134
SYS_RECVMMSG = 299
SYS_OPEN_BY_HANDLE_AT = 304
SYS_SECCOMP = 317
SYS_MEMFD_CREATE = 319
SYS_EXECVEAT = 322
SYS_PKEY_MPROTECT = 329
SYS_IO_URING_SETUP = 425
SYS_IO_URING_ENTER = 426
SYS_IO_URING_REGISTER = 427
SYS_PIDFD_GETFD = 438
SYS_MEMFD_SECRET = 447
CGROUP2_SUPER_MAGIC = 0x63677270
LANDLOCK_ACCESS_FS_EXECUTE = 1 << 0
LANDLOCK_CREATE_RULESET_VERSION = 1 << 0
LANDLOCK_RULE_PATH_BENEATH = 1
LANDLOCK_MINIMUM_ABI = 1
SYS_LANDLOCK_CREATE_RULESET = 444
SYS_LANDLOCK_ADD_RULE = 445
SYS_LANDLOCK_RESTRICT_SELF = 446
ELF64_HEADER_BYTES = 64
ELF64_PROGRAM_HEADER_BYTES = 56
ELF_MACHINE_X86_64 = 62
ELF_PROGRAM_INTERPRETER = 3
MAX_ELF_PROGRAM_HEADERS = 128
WORKER_COMPONENTS = (
    "broker",
    "identity",
    "behavior_schema",
    "behavior",
)
WORKER_PROTOCOL = "linux_capability_preflight_broker_worker_v1"


class BrokerUnavailable(RuntimeError):
    """A stable fail-closed capability-broker failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


@dataclass(frozen=True)
class RenderedCommand:
    """One closed retained-descriptor runtime request."""

    operation_id: str
    executable_fd: int
    argv: list[str]
    environment: dict[str, str]
    cwd_fd: int
    inherited_fds: tuple[int, ...]


@dataclass(frozen=True)
class CommandResult:
    """Bounded result returned by a retained runtime command."""

    returncode: int
    stdout: bytes
    stderr: bytes


@dataclass(frozen=True)
class CgroupHandle:
    """One fresh delegated cgroup child retained by descriptor."""

    root_fd: int
    child_fd: int
    name: str


class _LinuxStatFs(ctypes.Structure):
    _fields_ = [
        ("f_type", ctypes.c_long),
        ("f_bsize", ctypes.c_long),
        ("f_blocks", ctypes.c_ulong),
        ("f_bfree", ctypes.c_ulong),
        ("f_bavail", ctypes.c_ulong),
        ("f_files", ctypes.c_ulong),
        ("f_ffree", ctypes.c_ulong),
        ("f_fsid", ctypes.c_int * 2),
        ("f_namelen", ctypes.c_long),
        ("f_frsize", ctypes.c_long),
        ("f_flags", ctypes.c_long),
        ("f_spare", ctypes.c_long * 4),
    ]


class _LandlockRulesetAttr(ctypes.Structure):
    _fields_ = [("handled_access_fs", ctypes.c_uint64)]


class _LandlockPathBeneathAttr(ctypes.Structure):
    _pack_ = 1
    _fields_ = [
        ("allowed_access", ctypes.c_uint64),
        ("parent_fd", ctypes.c_int32),
    ]


class _SockFilter(ctypes.Structure):
    _fields_ = [
        ("code", ctypes.c_uint16),
        ("jt", ctypes.c_uint8),
        ("jf", ctypes.c_uint8),
        ("k", ctypes.c_uint32),
    ]


class _SockFprog(ctypes.Structure):
    _fields_ = [
        ("length", ctypes.c_uint16),
        ("filter", ctypes.POINTER(_SockFilter)),
    ]


class OutputAccumulator:
    """Account stdout and stderr under one aggregate streaming ceiling."""

    def __init__(self, *, limit: int) -> None:
        if type(limit) is not int or limit <= 0:
            raise BrokerUnavailable(
                "BROKER_RUNTIME_OUTPUT_LIMIT_INVALID",
                "combined runtime output ceiling must be positive",
            )
        self.limit = limit
        self._stdout = bytearray()
        self._stderr = bytearray()

    @property
    def stdout(self) -> bytes:
        return bytes(self._stdout)

    @property
    def stderr(self) -> bytes:
        return bytes(self._stderr)

    def _add(self, target: bytearray, chunk: bytes) -> None:
        if not isinstance(chunk, bytes):
            raise BrokerUnavailable(
                "BROKER_RUNTIME_OUTPUT_INVALID",
                "runtime output must be bytes",
            )
        if len(self._stdout) + len(self._stderr) + len(chunk) > self.limit:
            self._stdout.clear()
            self._stderr.clear()
            raise BrokerUnavailable(
                "BROKER_RUNTIME_OUTPUT_LIMIT",
                "runtime exceeded the combined streaming output ceiling",
            )
        target.extend(chunk)

    def add_stdout(self, chunk: bytes) -> None:
        self._add(self._stdout, chunk)

    def add_stderr(self, chunk: bytes) -> None:
        self._add(self._stderr, chunk)


def _strict_document(raw: bytes, *, code: str) -> dict[str, Any]:
    if not raw or len(raw) > MAX_COMPONENT_BYTES:
        raise BrokerUnavailable(code, "broker document exceeds its byte ceiling")
    try:
        text = raw.decode("utf-8", errors="strict")
        if text.startswith("\ufeff") or "\x00" in text:
            raise ValueError("invalid broker document prefix")

        def reject_duplicates(
            pairs: list[tuple[str, object]],
        ) -> dict[str, object]:
            value: dict[str, object] = {}
            for key, item in pairs:
                if key in value:
                    raise ValueError("duplicate broker document key")
                value[key] = item
            return value

        document = json.loads(
            text,
            object_pairs_hook=reject_duplicates,
            parse_constant=lambda _value: (_ for _ in ()).throw(
                ValueError("non-finite number")
            ),
        )
    except (RecursionError, UnicodeDecodeError, ValueError) as error:
        raise BrokerUnavailable(code, "broker document is not strict JSON") from error
    if not isinstance(document, dict):
        raise BrokerUnavailable(code, "broker document must be an object")
    return document


def _validate_behavior_shape(value: Mapping[str, Any]) -> None:
    operations = value.get("operations")
    state_root = value.get("state_root")
    execution = value.get("execution")
    cleanup = execution.get("cleanup") if isinstance(execution, Mapping) else None
    valid = (
        value.get("schema_version") == 1
        and value.get("profile") == "linux_capability_preflight_broker_v1"
        and value.get("platform") == "linux"
        and value.get("architecture") == "amd64"
        and value.get("runtime_identity_roles")
        == {
            "runtime": "runtime",
            "oci_runtime": "oci_runtime",
            "conmon": "conmon",
        }
        and isinstance(state_root, Mapping)
        and state_root.get("kind") == "retained_private_directory"
        and state_root.get("mode") == 0o700
        and state_root.get("children") == list(STATE_CHILDREN)
        and state_root.get("prepopulated_children") == ["storage"]
        and state_root.get("locator") == "proc_self_fd"
        and isinstance(operations, list)
        and [item.get("id") for item in operations if isinstance(item, Mapping)]
        == list(OPERATION_IDS)
        and isinstance(execution, Mapping)
        and execution.get("stdin") == "null"
        and execution.get("timeout_ms") == 15_000
        and execution.get("combined_output_bytes") == 262_144
        and execution.get("output_accounting") == "combined_streaming"
        and execution.get("partial_output") == "reject"
        and execution.get("ambient_exec")
        == {
            "kind": "landlock_path_beneath",
            "minimum_abi": LANDLOCK_MINIMUM_ABI,
            "handled_access": "execute",
            "coverage": "pathname_backed_executables",
            "allowed_role": "runtime",
            "default": "deny",
        }
        and execution.get("in_memory_exec")
        == {
            "kind": "seccomp_bpf",
            "architecture": "x86_64",
            "arch_mismatch": "kill_process",
            "x32_syscalls": "kill_process",
            "default": "allow",
            "deny_errno": "EPERM",
            "close_unlisted_descriptors_before_sandbox": True,
            "deny_syscalls": [
                "execveat",
                "io_uring_enter",
                "io_uring_register",
                "io_uring_setup",
                "memfd_create",
                "memfd_secret",
                "open_by_handle_at",
                "pidfd_getfd",
                "recvmmsg",
                "recvmsg",
                "uselib",
            ],
            "deny_flagged_syscalls": [
                {"syscall": "mmap", "argument": 2, "mask": "PROT_EXEC"},
                {"syscall": "mprotect", "argument": 2, "mask": "PROT_EXEC"},
                {
                    "syscall": "pkey_mprotect",
                    "argument": 2,
                    "mask": "PROT_EXEC",
                },
                {"syscall": "shmat", "argument": 2, "mask": "SHM_EXEC"},
            ],
        }
        and isinstance(cleanup, Mapping)
        and cleanup.get("kind") == "delegated_cgroup_v2"
        and cleanup.get("membership") == "cgroup.procs"
        and cleanup.get("kill") == "cgroup.kill"
        and cleanup.get("empty_proof") == "cgroup.events:populated 0"
        and cleanup.get("subreaper") is True
        and cleanup.get("process_group_kill") == "supplemental"
        and cleanup.get("cleanup_timeout_ms") == 5_000
    )
    if not valid:
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_INVALID",
            "capability broker behavior is outside the closed v1 profile",
        )


def parse_behavior_manifest(raw: bytes, schema_raw: bytes) -> dict[str, Any]:
    """Validate the exact language-neutral behavior against its const schema."""

    value = _strict_document(raw, code="BROKER_BEHAVIOR_INVALID")
    schema = _strict_document(schema_raw, code="BROKER_BEHAVIOR_SCHEMA_INVALID")
    if (
        schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema"
        or schema.get("$id")
        != (
            "https://coding-adventures.dev/schemas/"
            "build-tool-linux-capability-preflight-broker-v1.json"
        )
        or schema.get("const") != value
    ):
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_SCHEMA_INVALID",
            "capability broker behavior does not match its exact schema",
        )
    _validate_behavior_shape(value)
    return value


def _proc_fd(descriptor: int) -> str:
    if type(descriptor) is not int or descriptor < 0:
        raise BrokerUnavailable(
            "BROKER_DESCRIPTOR_INVALID",
            "retained descriptor must be a non-negative integer",
        )
    return f"{PROC_FD_PREFIX}/{descriptor}"


def _identity_mapping(
    identity: Mapping[str, Any],
    key: str,
) -> Mapping[str, Any]:
    value = identity.get(key)
    if not isinstance(value, Mapping):
        raise BrokerUnavailable(
            "BROKER_IDENTITY_INVALID",
            "Linux OCI identity is outside the closed broker profile",
        )
    return value


def _render_value(
    token: Mapping[str, Any],
    *,
    identity: Mapping[str, Any],
    oci_runtime_fd: int,
    state_fds: Mapping[str, int],
) -> str:
    kind = token.get("kind")
    if kind == "literal" and set(token) == {"kind", "value"}:
        value = token.get("value")
        if isinstance(value, str):
            return value
    if kind == "state_path" and set(token) == {"kind", "name"}:
        name = token.get("name")
        if isinstance(name, str) and name in STATE_CHILDREN and name in state_fds:
            return _proc_fd(state_fds[name])
    if (
        kind == "opened_executable"
        and token == {"kind": "opened_executable", "role": "oci_runtime"}
    ):
        return _proc_fd(oci_runtime_fd)
    if kind == "identity_ref":
        path = token.get("path")
        prefix = token.get("prefix", "")
        allowed_keys = {"kind", "path"} | ({"prefix"} if "prefix" in token else set())
        if set(token) != allowed_keys or not isinstance(prefix, str):
            raise BrokerUnavailable(
                "BROKER_BEHAVIOR_INVALID",
                "identity reference is outside the closed command grammar",
            )
        if path == "runtime.path":
            selected = _identity_mapping(identity, "runtime").get("path")
        elif path == "oci_runtime.path":
            selected = _identity_mapping(identity, "oci_runtime").get("path")
        elif path == "conmon.path":
            selected = _identity_mapping(identity, "conmon").get("path")
        elif path == "image.config_sha256":
            selected = _identity_mapping(identity, "image").get("config_sha256")
        else:
            selected = None
        if isinstance(selected, str):
            return prefix + selected
    raise BrokerUnavailable(
        "BROKER_BEHAVIOR_INVALID",
        "command token is outside the closed capability grammar",
    )


def render_operations(
    behavior: Mapping[str, Any],
    identity: Mapping[str, Any],
    *,
    runtime_fd: int,
    oci_runtime_fd: int,
    state_fds: Mapping[str, int],
) -> tuple[RenderedCommand, RenderedCommand]:
    """Render exactly the two approved operations from retained descriptors."""

    _validate_behavior_shape(behavior)
    if set(state_fds) != set(STATE_CHILDREN):
        raise BrokerUnavailable(
            "BROKER_STATE_DESCRIPTOR_INVALID",
            "exact retained private state descriptors are required",
        )
    runtime = _identity_mapping(identity, "runtime")
    oci_runtime = _identity_mapping(identity, "oci_runtime")
    conmon = _identity_mapping(identity, "conmon")
    image = _identity_mapping(identity, "image")
    if (
        runtime.get("implementation") != "podman"
        or runtime.get("path") != "/usr/bin/podman"
        or runtime.get("linkage") != "static"
        or oci_runtime.get("implementation") != "crun"
        or oci_runtime.get("path") != "/usr/bin/crun"
        or conmon.get("implementation") != "conmon"
        or conmon.get("path") != "/usr/bin/conmon"
        or not isinstance(image.get("config_sha256"), str)
    ):
        raise BrokerUnavailable(
            "BROKER_IDENTITY_INVALID",
            "Linux OCI identity is outside the closed broker profile",
        )
    environment_value = behavior.get("environment")
    operations_value = behavior.get("operations")
    execution = behavior.get("execution")
    if (
        not isinstance(environment_value, Mapping)
        or not isinstance(operations_value, list)
        or not isinstance(execution, Mapping)
    ):
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_INVALID",
            "capability broker behavior is incomplete",
        )
    environment = {
        key: _render_value(
            token,
            identity=identity,
            oci_runtime_fd=oci_runtime_fd,
            state_fds=state_fds,
        )
        for key, token in environment_value.items()
        if isinstance(key, str) and isinstance(token, Mapping)
    }
    expected_environment = {
        "HOME": _proc_fd(state_fds["home"]),
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "PATH": "/nonexistent",
        "TZ": "UTC",
        "XDG_CONFIG_HOME": _proc_fd(state_fds["config"]),
        "XDG_RUNTIME_DIR": _proc_fd(state_fds["runtime"]),
    }
    if environment != expected_environment:
        raise BrokerUnavailable(
            "BROKER_ENVIRONMENT_INVALID",
            "capability broker environment is not the closed v1 set",
        )
    cwd = execution.get("cwd")
    if not isinstance(cwd, Mapping):
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_INVALID",
            "capability broker cwd is invalid",
        )
    cwd_value = _render_value(
        cwd,
        identity=identity,
        oci_runtime_fd=oci_runtime_fd,
        state_fds=state_fds,
    )
    cwd_fd = next(
        (descriptor for descriptor in state_fds.values() if _proc_fd(descriptor) == cwd_value),
        -1,
    )
    if cwd_fd != state_fds["home"]:
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_INVALID",
            "capability broker cwd is not the retained home directory",
        )
    expected_descriptors = [
        "stdin",
        "stdout",
        "stderr",
        "runtime",
        "state.config",
        "state.home",
        "state.runtime",
        "state.runroot",
        "state.storage",
    ]
    if execution.get("descriptor_allowlist") != expected_descriptors:
        raise BrokerUnavailable(
            "BROKER_DESCRIPTOR_ALLOWLIST_INVALID",
            "capability broker descriptor set is not the closed v1 set",
        )

    rendered: list[RenderedCommand] = []
    for operation in operations_value:
        if not isinstance(operation, Mapping):
            raise BrokerUnavailable(
                "BROKER_BEHAVIOR_INVALID",
                "capability broker operation is invalid",
            )
        executable = operation.get("executable")
        argv_tokens = operation.get("argv")
        if (
            executable != {"kind": "opened_executable", "role": "runtime"}
            or not isinstance(argv_tokens, list)
        ):
            raise BrokerUnavailable(
                "BROKER_BEHAVIOR_INVALID",
                "capability broker executable grammar is invalid",
            )
        argv = [
            _render_value(
                token,
                identity=identity,
                oci_runtime_fd=oci_runtime_fd,
                state_fds=state_fds,
            )
            for token in argv_tokens
            if isinstance(token, Mapping)
        ]
        if len(argv) != len(argv_tokens):
            raise BrokerUnavailable(
                "BROKER_BEHAVIOR_INVALID",
                "capability broker argv token is invalid",
            )
        rendered.append(
            RenderedCommand(
                operation_id=str(operation.get("id")),
                executable_fd=runtime_fd,
                argv=argv,
                environment=dict(environment),
                cwd_fd=cwd_fd,
                inherited_fds=(
                    *(state_fds[name] for name in STATE_CHILDREN),
                ),
            )
        )
    if [item.operation_id for item in rendered] != list(OPERATION_IDS):
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_INVALID",
            "capability broker operations are not the closed ordered pair",
        )
    storage_common = [
        "/usr/bin/podman",
        "--remote=false",
        "--root",
        _proc_fd(state_fds["storage"]),
        "--runroot",
        _proc_fd(state_fds["runroot"]),
        "--runtime",
        "/usr/bin/crun",
        "--conmon",
        "/usr/bin/conmon",
        "--storage-driver",
        "vfs",
    ]
    exact_argv = (
        [
            "/usr/bin/podman",
            "--remote=false",
            "version",
            "--format",
            "json",
        ],
        [
            *storage_common,
            "image",
            "inspect",
            "--format",
            "json",
            f"sha256:{image['config_sha256']}",
        ],
    )
    if tuple(command.argv for command in rendered) != exact_argv:
        raise BrokerUnavailable(
            "BROKER_BEHAVIOR_INVALID",
            "capability broker argv is not the closed v1 grammar",
        )
    return rendered[0], rendered[1]


def binary_stat_identity(status: os.stat_result) -> tuple[int, ...]:
    """Return every stable executable identity field, including ctime."""

    return (
        status.st_dev,
        status.st_ino,
        status.st_size,
        status.st_mtime_ns,
        status.st_mode,
        status.st_uid,
        status.st_nlink,
        status.st_ctime_ns,
    )


def validate_binary_status(status: os.stat_result) -> None:
    """Require an immutable root-owned ordinary executable."""

    mode = status.st_mode
    if (
        not stat.S_ISREG(mode)
        or status.st_uid != 0
        or status.st_nlink < 1
        or not 0 < status.st_size <= MAX_RUNTIME_BINARY_BYTES
        or mode & (stat.S_ISUID | stat.S_ISGID)
        or mode & (stat.S_IWGRP | stat.S_IWOTH)
        or not mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    ):
        raise BrokerUnavailable(
            "BROKER_BINARY_INVALID",
            "runtime binaries must be immutable root-owned regular executables",
        )


def open_verified_binary(path: str, expected_sha256: str) -> int:
    """Open, hash, and retain one exact runtime executable."""

    if (
        path not in {"/usr/bin/podman", "/usr/bin/crun", "/usr/bin/conmon"}
        or len(expected_sha256) != 64
        or any(character not in "0123456789abcdef" for character in expected_sha256)
    ):
        raise BrokerUnavailable(
            "BROKER_BINARY_IDENTITY_INVALID",
            "runtime binary identity is outside the closed profile",
        )
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_BINARY_UNAVAILABLE",
            "approved runtime binary is unavailable",
        ) from error
    try:
        before = os.fstat(descriptor)
        validate_binary_status(before)
        digest = hashlib.sha256()
        offset = 0
        while offset < before.st_size:
            chunk = os.pread(
                descriptor,
                min(1_048_576, before.st_size - offset),
                offset,
            )
            if not chunk:
                break
            digest.update(chunk)
            offset += len(chunk)
        after = os.fstat(descriptor)
        validate_binary_status(after)
        if (
            offset != before.st_size
            or binary_stat_identity(before) != binary_stat_identity(after)
        ):
            raise BrokerUnavailable(
                "BROKER_BINARY_CHANGED",
                "runtime binary changed while retained bytes were hashed",
            )
        if digest.hexdigest() != expected_sha256:
            raise BrokerUnavailable(
                "BROKER_BINARY_IDENTITY_MISMATCH",
                "runtime binary does not match the approved identity",
            )
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


def validate_static_runtime_elf(descriptor: int) -> None:
    """Require a static ELF64 x86-64 runtime before one-inode confinement."""

    try:
        status = os.fstat(descriptor)
        header = os.pread(descriptor, ELF64_HEADER_BYTES, 0)
        (
            identification,
            elf_type,
            machine,
            version,
            _entry,
            program_header_offset,
            _section_header_offset,
            _flags,
            header_size,
            program_header_size,
            program_header_count,
            _section_header_size,
            _section_header_count,
            _section_name_index,
        ) = struct.unpack("<16sHHIQQQIHHHHHH", header)
        table_size = program_header_size * program_header_count
        if (
            identification[:4] != b"\x7fELF"
            or identification[4:7] != b"\x02\x01\x01"
            or elf_type not in {2, 3}
            or machine != ELF_MACHINE_X86_64
            or version != 1
            or header_size != ELF64_HEADER_BYTES
            or program_header_size != ELF64_PROGRAM_HEADER_BYTES
            or not 0 < program_header_count <= MAX_ELF_PROGRAM_HEADERS
            or program_header_offset < ELF64_HEADER_BYTES
            or table_size > status.st_size
            or program_header_offset > status.st_size - table_size
        ):
            raise ValueError("invalid ELF64 runtime layout")
        program_headers = os.pread(
            descriptor,
            table_size,
            program_header_offset,
        )
        if len(program_headers) != table_size:
            raise ValueError("truncated ELF64 program header table")
        for offset in range(0, table_size, program_header_size):
            program_type = struct.unpack_from("<I", program_headers, offset)[0]
            if program_type == ELF_PROGRAM_INTERPRETER:
                raise BrokerUnavailable(
                    "BROKER_RUNTIME_LINKAGE_INVALID",
                    "approved Podman runtime must be statically linked",
                )
    except BrokerUnavailable:
        raise
    except (AttributeError, OSError, struct.error, ValueError) as error:
        raise BrokerUnavailable(
            "BROKER_RUNTIME_LINKAGE_INVALID",
            "approved Podman runtime is not a valid static ELF64 amd64 executable",
        ) from error


def _private_directory_status(status: os.stat_result, *, owner: int) -> bool:
    return (
        stat.S_ISDIR(status.st_mode)
        and status.st_uid == owner
        and stat.S_IMODE(status.st_mode) == 0o700
    )


def open_private_state_children(state_root_fd: int) -> Mapping[str, int]:
    """Retain prepopulated storage and create four private transient children."""

    owner = os.geteuid()
    try:
        root_status = os.fstat(state_root_fd)
        existing = os.listdir(state_root_fd)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_STATE_ROOT_INVALID",
            "retained state root is unavailable",
        ) from error
    if (
        not _private_directory_status(root_status, owner=owner)
        or set(existing) != {"storage"}
        or len(existing) != 1
    ):
        raise BrokerUnavailable(
            "BROKER_STATE_ROOT_INVALID",
            "retained state root must contain only a private image store",
        )
    opened: dict[str, int] = {}
    created: list[str] = []
    flags = (
        os.O_RDONLY
        | os.O_CLOEXEC
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        storage_fd = os.open("storage", flags, dir_fd=state_root_fd)
        opened["storage"] = storage_fd
        storage_status = os.fstat(storage_fd)
        if not _private_directory_status(storage_status, owner=owner):
            raise BrokerUnavailable(
                "BROKER_STATE_CHILD_INVALID",
                "retained image store is not a private owned directory",
            )
        for name in STATE_CHILDREN:
            if name == "storage":
                continue
            os.mkdir(name, mode=0o700, dir_fd=state_root_fd)
            created.append(name)
            descriptor = os.open(name, flags, dir_fd=state_root_fd)
            status = os.fstat(descriptor)
            if not _private_directory_status(status, owner=owner):
                os.close(descriptor)
                raise BrokerUnavailable(
                    "BROKER_STATE_CHILD_INVALID",
                    "retained state child is not a private owned directory",
                )
            opened[name] = descriptor
        return MappingProxyType({name: opened[name] for name in STATE_CHILDREN})
    except (BrokerUnavailable, OSError) as error:
        for descriptor in opened.values():
            os.close(descriptor)
        for name in reversed(created):
            try:
                os.rmdir(name, dir_fd=state_root_fd)
            except OSError:
                pass
        if isinstance(error, BrokerUnavailable):
            raise
        raise BrokerUnavailable(
            "BROKER_STATE_CHILD_INVALID",
            "private state children could not be created atomically",
        ) from error


def _read_small_at(root_fd: int, name: str, *, limit: int = 65_536) -> bytes:
    flags = os.O_RDONLY | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(name, flags, dir_fd=root_fd)
        try:
            value = os.read(descriptor, limit + 1)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "required delegated cgroup control is unavailable",
        ) from error
    if len(value) > limit:
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated cgroup control exceeds its byte ceiling",
        )
    return value


def _write_small_at(root_fd: int, name: str, value: bytes) -> None:
    flags = os.O_WRONLY | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(name, flags, dir_fd=root_fd)
        try:
            written = os.write(descriptor, value)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_CGROUP_CONTROL_FAILED",
            "delegated cgroup control write failed",
        ) from error
    if written != len(value):
        raise BrokerUnavailable(
            "BROKER_CGROUP_CONTROL_FAILED",
            "delegated cgroup control write was incomplete",
        )


def _word_set(raw: bytes) -> frozenset[str]:
    try:
        return frozenset(raw.decode("ascii", errors="strict").split())
    except UnicodeDecodeError as error:
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated cgroup controls are not ASCII",
        ) from error


def cgroup_is_empty(raw: bytes) -> bool:
    """Parse the exact unique `populated` field from cgroup.events."""

    try:
        lines = raw.decode("ascii", errors="strict").splitlines()
    except UnicodeDecodeError as error:
        raise BrokerUnavailable(
            "BROKER_CGROUP_EVENTS_INVALID",
            "cgroup events are not strict ASCII",
        ) from error
    populated: list[str] = []
    for line in lines:
        fields = line.split()
        if len(fields) != 2:
            raise BrokerUnavailable(
                "BROKER_CGROUP_EVENTS_INVALID",
                "cgroup events contain an invalid field",
            )
        if fields[0] == "populated":
            populated.append(fields[1])
    if len(populated) != 1 or populated[0] not in {"0", "1"}:
        raise BrokerUnavailable(
            "BROKER_CGROUP_EVENTS_INVALID",
            "cgroup events do not contain one exact populated field",
        )
    return populated[0] == "0"


def validate_cgroup2_descriptor(descriptor: int) -> None:
    """Require the delegated root to be a real cgroup-v2 filesystem handle."""

    result = _LinuxStatFs()
    library = ctypes.CDLL(None, use_errno=True)
    if library.fstatfs(ctypes.c_int(descriptor), ctypes.byref(result)) != 0:
        selected_errno = ctypes.get_errno()
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated cgroup root filesystem could not be identified",
        ) from OSError(selected_errno, os.strerror(selected_errno))
    if result.f_type != CGROUP2_SUPER_MAGIC:
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated command root is not a cgroup v2 filesystem",
        )


def validate_kernel_seccomp() -> None:
    """Prove the fixed Linux kernel exposes the required seccomp actions."""

    path = "/proc/sys/kernel/seccomp/actions_avail"
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        descriptor = os.open(path, flags)
        try:
            status = os.fstat(descriptor)
            raw = os.read(descriptor, 4097)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_SECCOMP_UNAVAILABLE",
            "kernel seccomp capability control is unavailable",
        ) from error
    if (
        not stat.S_ISREG(status.st_mode)
        or status.st_uid != 0
        or status.st_mode & (stat.S_IWGRP | stat.S_IWOTH)
        or len(raw) > 4096
    ):
        raise BrokerUnavailable(
            "BROKER_SECCOMP_UNAVAILABLE",
            "kernel seccomp capability control is not trusted",
        )
    try:
        actions = frozenset(raw.decode("ascii", errors="strict").split())
    except UnicodeDecodeError as error:
        raise BrokerUnavailable(
            "BROKER_SECCOMP_UNAVAILABLE",
            "kernel seccomp capability control is invalid",
        ) from error
    if not {"kill_process", "errno", "allow"}.issubset(actions):
        raise BrokerUnavailable(
            "BROKER_SECCOMP_UNAVAILABLE",
            "required kernel seccomp actions are unavailable",
        )


def create_command_cgroup(root_fd: int, name: str) -> CgroupHandle:
    """Create a fresh child only after delegation and cgroup.kill are proven."""

    if name not in {
        "capability-preflight-runtime-version",
        "capability-preflight-image-inspect",
    }:
        raise BrokerUnavailable(
            "BROKER_CGROUP_NAME_INVALID",
            "cgroup name is outside the closed operation set",
        )
    validate_cgroup2_descriptor(root_fd)
    controllers = _word_set(_read_small_at(root_fd, "cgroup.controllers"))
    enabled = _word_set(_read_small_at(root_fd, "cgroup.subtree_control"))
    if not REQUIRED_CGROUP_CONTROLLERS.issubset(controllers) or not (
        REQUIRED_CGROUP_CONTROLLERS.issubset(enabled)
    ):
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated cpu, memory, and pids cgroup v2 controllers are required",
        )
    created = False
    try:
        os.mkdir(name, mode=0o700, dir_fd=root_fd)
        created = True
        flags = (
            os.O_RDONLY
            | os.O_CLOEXEC
            | getattr(os, "O_DIRECTORY", 0)
            | getattr(os, "O_NOFOLLOW", 0)
        )
        child_fd = os.open(name, flags, dir_fd=root_fd)
    except OSError as error:
        if created:
            try:
                os.rmdir(name, dir_fd=root_fd)
            except OSError:
                pass
        raise BrokerUnavailable(
            "BROKER_CGROUP_CREATE_FAILED",
            "fresh delegated command cgroup could not be created",
        ) from error
    try:
        for control, flags in (
            ("cgroup.kill", os.O_WRONLY),
            ("cgroup.procs", os.O_WRONLY),
            ("cgroup.events", os.O_RDONLY),
        ):
            descriptor = os.open(
                control,
                flags | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0),
                dir_fd=child_fd,
            )
            os.close(descriptor)
        if not cgroup_is_empty(_read_small_at(child_fd, "cgroup.events")):
            raise BrokerUnavailable(
                "BROKER_CGROUP_CREATE_FAILED",
                "fresh delegated command cgroup is already populated",
            )
        return CgroupHandle(root_fd=root_fd, child_fd=child_fd, name=name)
    except BaseException:
        os.close(child_fd)
        try:
            os.rmdir(name, dir_fd=root_fd)
        except OSError:
            pass
        raise


def _prctl(option: int, value: int) -> None:
    library = ctypes.CDLL(None, use_errno=True)
    result = library.prctl(
        ctypes.c_int(option),
        ctypes.c_ulong(value),
        ctypes.c_ulong(0),
        ctypes.c_ulong(0),
        ctypes.c_ulong(0),
    )
    if result != 0:
        selected_errno = ctypes.get_errno()
        raise OSError(selected_errno, os.strerror(selected_errno))


def _enable_subreaper() -> None:
    try:
        _prctl(PR_SET_CHILD_SUBREAPER, 1)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_SUBREAPER_UNAVAILABLE",
            "broker could not become a child subreaper",
        ) from error


def _install_parent_death_signal(expected_parent: int) -> None:
    _prctl(PR_SET_PDEATHSIG, signal.SIGKILL)
    if os.getppid() != expected_parent:
        os.kill(os.getpid(), signal.SIGKILL)


def _linux_syscall(number: int, *arguments: object) -> int:
    library = ctypes.CDLL(None, use_errno=True)
    result = int(library.syscall(ctypes.c_long(number), *arguments))
    if result < 0:
        selected_errno = ctypes.get_errno()
        raise OSError(selected_errno, os.strerror(selected_errno))
    return result


def install_execute_landlock(executable_fd: int) -> None:
    """Restrict pathname-backed execution to the retained Podman inode."""

    if not sys.platform.startswith("linux"):
        raise BrokerUnavailable(
            "BROKER_EXEC_SANDBOX_UNAVAILABLE",
            "Landlock executable confinement requires Linux",
        )
    ruleset_fd = -1
    try:
        abi = _linux_syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            ctypes.c_void_p(),
            ctypes.c_size_t(0),
            ctypes.c_uint(LANDLOCK_CREATE_RULESET_VERSION),
        )
        if abi < LANDLOCK_MINIMUM_ABI:
            raise BrokerUnavailable(
                "BROKER_EXEC_SANDBOX_UNAVAILABLE",
                "kernel Landlock ABI cannot confine ambient executable lookup",
            )
        ruleset = _LandlockRulesetAttr(
            handled_access_fs=LANDLOCK_ACCESS_FS_EXECUTE,
        )
        ruleset_fd = _linux_syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            ctypes.byref(ruleset),
            ctypes.c_size_t(ctypes.sizeof(ruleset)),
            ctypes.c_uint(0),
        )
        path_rule = _LandlockPathBeneathAttr(
            allowed_access=LANDLOCK_ACCESS_FS_EXECUTE,
            parent_fd=executable_fd,
        )
        _linux_syscall(
            SYS_LANDLOCK_ADD_RULE,
            ctypes.c_int(ruleset_fd),
            ctypes.c_int(LANDLOCK_RULE_PATH_BENEATH),
            ctypes.byref(path_rule),
            ctypes.c_uint(0),
        )
        _prctl(PR_SET_NO_NEW_PRIVS, 1)
        _linux_syscall(
            SYS_LANDLOCK_RESTRICT_SELF,
            ctypes.c_int(ruleset_fd),
            ctypes.c_uint(0),
        )
    except BrokerUnavailable:
        raise
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_EXEC_SANDBOX_UNAVAILABLE",
            "Landlock could not deny pathname-backed helper execution",
        ) from error
    finally:
        if ruleset_fd >= 0:
            os.close(ruleset_fd)


def _anonymous_exec_seccomp_instructions() -> tuple[_SockFilter, ...]:
    """Return the closed amd64 classic-BPF executable-creation policy."""

    deny = SECCOMP_RET_ERRNO | errno.EPERM
    instructions = [
        _SockFilter(BPF_LD_W_ABS, 0, 0, SECCOMP_DATA_ARCH_OFFSET),
        _SockFilter(BPF_JMP_JEQ_K, 1, 0, AUDIT_ARCH_X86_64),
        _SockFilter(BPF_RET_K, 0, 0, SECCOMP_RET_KILL_PROCESS),
        _SockFilter(BPF_LD_W_ABS, 0, 0, SECCOMP_DATA_NR_OFFSET),
        _SockFilter(BPF_JMP_JGE_K, 0, 1, X32_SYSCALL_BIT),
        _SockFilter(BPF_RET_K, 0, 0, SECCOMP_RET_KILL_PROCESS),
    ]
    for number in (
        SYS_EXECVEAT,
        SYS_IO_URING_ENTER,
        SYS_IO_URING_REGISTER,
        SYS_IO_URING_SETUP,
        SYS_MEMFD_CREATE,
        SYS_MEMFD_SECRET,
        SYS_OPEN_BY_HANDLE_AT,
        SYS_PIDFD_GETFD,
        SYS_RECVMMSG,
        SYS_RECVMSG,
        SYS_USELIB,
    ):
        instructions.extend(
            (
                _SockFilter(BPF_JMP_JEQ_K, 0, 1, number),
                _SockFilter(BPF_RET_K, 0, 0, deny),
            )
        )
    for number, argument, mask in (
        (SYS_MMAP, 2, PROT_EXEC),
        (SYS_MPROTECT, 2, PROT_EXEC),
        (SYS_PKEY_MPROTECT, 2, PROT_EXEC),
        (SYS_SHMAT, 2, SHM_EXEC),
    ):
        instructions.extend(
            (
                _SockFilter(BPF_JMP_JEQ_K, 0, 4, number),
                _SockFilter(
                    BPF_LD_W_ABS,
                    0,
                    0,
                    SECCOMP_DATA_ARGS_OFFSET + argument * 8,
                ),
                _SockFilter(BPF_JMP_JSET_K, 0, 1, mask),
                _SockFilter(BPF_RET_K, 0, 0, deny),
                _SockFilter(BPF_RET_K, 0, 0, SECCOMP_RET_ALLOW),
            )
        )
    instructions.append(_SockFilter(BPF_RET_K, 0, 0, SECCOMP_RET_ALLOW))
    return tuple(instructions)


def _install_seccomp_program(instructions: Sequence[_SockFilter]) -> None:
    if not instructions or len(instructions) > 65_535:
        raise OSError(errno.EINVAL, "invalid classic seccomp program")
    filter_array = (_SockFilter * len(instructions))(*instructions)
    program = _SockFprog(
        length=len(instructions),
        filter=ctypes.cast(filter_array, ctypes.POINTER(_SockFilter)),
    )
    _linux_syscall(
        SYS_SECCOMP,
        ctypes.c_uint(SECCOMP_SET_MODE_FILTER),
        ctypes.c_uint(0),
        ctypes.byref(program),
    )


def install_in_memory_exec_seccomp() -> None:
    """Deny post-transition anonymous execution and executable mappings."""

    if (
        not sys.platform.startswith("linux")
        or os.uname().machine not in {"x86_64", "amd64"}
    ):
        raise BrokerUnavailable(
            "BROKER_EXEC_SANDBOX_UNAVAILABLE",
            "anonymous execution confinement requires Linux amd64",
        )
    try:
        _prctl(PR_SET_NO_NEW_PRIVS, 1)
        _install_seccomp_program(_anonymous_exec_seccomp_instructions())
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_EXEC_SANDBOX_UNAVAILABLE",
            "seccomp could not deny anonymous executable creation",
        ) from error


def install_command_exec_sandbox(executable_fd: int) -> None:
    """Install the complete pathname and in-memory execution transition."""

    install_execute_landlock(executable_fd)
    install_in_memory_exec_seccomp()


def _close_unlisted_descriptors(keep: set[int]) -> None:
    if resource is None:
        os._exit(126)
    maximum = resource.getrlimit(resource.RLIMIT_NOFILE)[0]
    if maximum == resource.RLIM_INFINITY:
        maximum = 65_536
    maximum = min(int(maximum), 1_048_576)
    start = 3
    for descriptor in sorted(item for item in keep if item >= 3):
        if start < descriptor:
            os.closerange(start, descriptor)
        start = descriptor + 1
    if start < maximum:
        os.closerange(start, maximum)


def _child_exec(
    command: RenderedCommand,
    *,
    gate_fd: int,
    stdout_fd: int,
    stderr_fd: int,
    expected_parent: int,
) -> None:
    try:
        os.setsid()
        _install_parent_death_signal(expected_parent)
        if os.read(gate_fd, 1) != b"1":
            os._exit(125)
        os.close(gate_fd)
        os.fchdir(command.cwd_fd)
        null_fd = os.open("/dev/null", os.O_RDONLY | os.O_CLOEXEC)
        os.dup2(null_fd, 0)
        os.dup2(stdout_fd, 1)
        os.dup2(stderr_fd, 2)
        if null_fd > 2:
            os.close(null_fd)
        for descriptor in command.inherited_fds:
            os.set_inheritable(descriptor, True)
        keep = {
            0,
            1,
            2,
            command.executable_fd,
            *command.inherited_fds,
        }
        _close_unlisted_descriptors(keep)
        install_command_exec_sandbox(command.executable_fd)
        os.execve(  # nosec B606
            _proc_fd(command.executable_fd),
            command.argv,
            command.environment,
        )
    except BrokerUnavailable as error:
        if error.code == "BROKER_EXEC_SANDBOX_UNAVAILABLE":
            try:
                os.write(2, b"BROKER_EXEC_SANDBOX_UNAVAILABLE\n")
            except OSError:
                pass
            os._exit(126)
        os._exit(127)
    except BaseException:  # noqa: BLE001 - the child must never unwind before exec
        os._exit(127)


def _peek_child_returncode(pid: int) -> int | None:
    """Observe exit without reaping so PID/PGID cannot be reused before cleanup."""

    try:
        status = os.waitid(
            os.P_PID,
            pid,
            os.WEXITED | os.WNOHANG | os.WNOWAIT,
        )
    except ChildProcessError:
        return 0
    if status is None or status.si_pid != pid:
        return None
    if status.si_code == os.CLD_EXITED:
        return int(status.si_status)
    return -int(status.si_status)


def _stream_child(
    pid: int,
    stdout_fd: int,
    stderr_fd: int,
    *,
    timeout_seconds: float,
    output_limit: int,
) -> CommandResult:
    accumulator = OutputAccumulator(limit=output_limit)
    selected = selectors.DefaultSelector()
    streams = {stdout_fd: accumulator.add_stdout, stderr_fd: accumulator.add_stderr}
    for descriptor in streams:
        os.set_blocking(descriptor, False)
        selected.register(descriptor, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout_seconds
    returncode: int | None = None
    try:
        while selected.get_map() or returncode is None:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise BrokerUnavailable(
                    "BROKER_RUNTIME_TIMEOUT",
                    "runtime exceeded the capability command deadline",
                )
            for key, _mask in selected.select(min(remaining, 0.05)):
                try:
                    chunk = os.read(key.fd, READ_CHUNK_BYTES)
                except BlockingIOError:
                    continue
                if chunk:
                    streams[key.fd](chunk)
                else:
                    selected.unregister(key.fd)
                    os.close(key.fd)
            if returncode is None:
                returncode = _peek_child_returncode(pid)
        return CommandResult(
            returncode=returncode,
            stdout=accumulator.stdout,
            stderr=accumulator.stderr,
        )
    finally:
        for key in list(selected.get_map().values()):
            selected.unregister(key.fd)
            try:
                os.close(key.fd)
            except OSError:
                pass
        selected.close()


def _reap_adopted_children() -> None:
    while True:
        try:
            pid, _status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return
        if pid <= 0:
            return


def cleanup_command_cgroup(
    cgroup: CgroupHandle,
    *,
    process_group: int | None,
    timeout_seconds: float,
) -> None:
    """Kill, prove empty, reap, and remove one exact command cgroup."""

    deadline = time.monotonic() + timeout_seconds
    cleanup_error: BrokerUnavailable | None = None
    try:
        _write_small_at(cgroup.child_fd, "cgroup.kill", b"1\n")
    except BrokerUnavailable as error:
        cleanup_error = error
    if process_group is not None:
        try:
            os.killpg(process_group, signal.SIGKILL)
        except OSError as error:
            if error.errno not in {errno.ESRCH, errno.EPERM} and cleanup_error is None:
                cleanup_error = BrokerUnavailable(
                    "BROKER_PROCESS_GROUP_KILL_FAILED",
                    "supplemental process-group termination failed",
                )
    empty = False
    while time.monotonic() < deadline:
        _reap_adopted_children()
        try:
            empty = cgroup_is_empty(
                _read_small_at(cgroup.child_fd, "cgroup.events")
            )
        except BrokerUnavailable as error:
            cleanup_error = cleanup_error or error
            break
        if empty:
            break
        time.sleep(0.01)
    try:
        os.close(cgroup.child_fd)
    finally:
        if empty:
            try:
                os.rmdir(cgroup.name, dir_fd=cgroup.root_fd)
            except OSError:
                cleanup_error = cleanup_error or BrokerUnavailable(
                    "BROKER_CGROUP_REMOVE_FAILED",
                    "empty delegated command cgroup could not be removed",
                )
    if not empty:
        cleanup_error = cleanup_error or BrokerUnavailable(
            "BROKER_CGROUP_CLEANUP_TIMEOUT",
            "delegated command cgroup did not become empty before the deadline",
        )
    if cleanup_error is not None:
        raise cleanup_error


def run_retained_command(
    command: RenderedCommand,
    *,
    cgroup_root_fd: int,
    timeout_ms: int,
    output_limit: int,
    cleanup_timeout_ms: int,
) -> CommandResult:
    """Fork, gate into a fresh cgroup, exec retained Podman, and fully clean up."""

    if (
        not sys.platform.startswith("linux")
        or not hasattr(os, "fork")
        or not hasattr(os, "waitid")
        or not hasattr(os, "WNOWAIT")
    ):
        raise BrokerUnavailable(
            "BROKER_PLATFORM_UNSUPPORTED",
            "retained-FD capability execution requires Linux",
        )
    _enable_subreaper()
    cgroup = create_command_cgroup(
        cgroup_root_fd,
        "capability-preflight-" + command.operation_id.replace("_", "-"),
    )
    gate_read = -1
    gate_write = -1
    stdout_read = -1
    stdout_write = -1
    stderr_read = -1
    stderr_write = -1
    pid: int | None = None
    result: CommandResult | None = None
    primary_error: BaseException | None = None
    try:
        gate_read, gate_write = os.pipe2(os.O_CLOEXEC)
        stdout_read, stdout_write = os.pipe2(os.O_CLOEXEC)
        stderr_read, stderr_write = os.pipe2(os.O_CLOEXEC)
        parent_pid = os.getpid()
        pid = os.fork()
        if pid == 0:
            os.close(gate_write)
            os.close(stdout_read)
            os.close(stderr_read)
            _child_exec(
                command,
                gate_fd=gate_read,
                stdout_fd=stdout_write,
                stderr_fd=stderr_write,
                expected_parent=parent_pid,
            )
            os._exit(127)
        os.close(gate_read)
        gate_read = -1
        os.close(stdout_write)
        stdout_write = -1
        os.close(stderr_write)
        stderr_write = -1
        _write_small_at(cgroup.child_fd, "cgroup.procs", f"{pid}\n".encode("ascii"))
        if os.write(gate_write, b"1") != 1:
            raise BrokerUnavailable(
                "BROKER_CHILD_RELEASE_FAILED",
                "retained runtime child could not be released",
            )
        os.close(gate_write)
        gate_write = -1
        result = _stream_child(
            pid,
            stdout_read,
            stderr_read,
            timeout_seconds=timeout_ms / 1000,
            output_limit=output_limit,
        )
        stdout_read = -1
        stderr_read = -1
    except Exception as error:  # noqa: BLE001 - cleanup must run for every failure
        primary_error = error
    finally:
        for descriptor in (
            gate_read,
            gate_write,
            stdout_read,
            stdout_write,
            stderr_read,
            stderr_write,
        ):
            if descriptor >= 0:
                try:
                    os.close(descriptor)
                except OSError:
                    pass
        try:
            cleanup_command_cgroup(
                cgroup,
                process_group=pid,
                timeout_seconds=cleanup_timeout_ms / 1000,
            )
        except Exception as cleanup_error:  # noqa: BLE001 - cleanup failure wins
            primary_error = cleanup_error
    if primary_error is not None:
        if isinstance(primary_error, BrokerUnavailable):
            raise primary_error
        raise BrokerUnavailable(
            "BROKER_RUNTIME_FAILED",
            "retained runtime command failed",
        ) from primary_error
    if result is None:
        raise BrokerUnavailable(
            "BROKER_RUNTIME_FAILED",
            "retained runtime command returned no result",
        )
    if result == CommandResult(
        returncode=126,
        stdout=b"",
        stderr=b"BROKER_EXEC_SANDBOX_UNAVAILABLE\n",
    ):
        raise BrokerUnavailable(
            "BROKER_EXEC_SANDBOX_UNAVAILABLE",
            "kernel execution sandbox could not be installed",
        )
    return result


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _sealed_memfd(name: str, raw: bytes) -> int:
    if (
        not sys.platform.startswith("linux")
        or not hasattr(os, "memfd_create")
        or fcntl is None
    ):
        raise BrokerUnavailable(
            "BROKER_PLATFORM_UNSUPPORTED",
            "sealed broker execution requires Linux memfd support",
        )
    if not raw or len(raw) > MAX_COMPONENT_BYTES:
        raise BrokerUnavailable(
            "BROKER_COMPONENT_SIZE_INVALID",
            "broker component exceeds its byte ceiling",
        )
    descriptor = os.memfd_create(
        name,
        flags=os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING,
    )
    try:
        offset = 0
        while offset < len(raw):
            written = os.write(descriptor, raw[offset:])
            if written <= 0:
                raise OSError("short anonymous component write")
            offset += written
        seals = (
            fcntl.F_SEAL_WRITE
            | fcntl.F_SEAL_GROW
            | fcntl.F_SEAL_SHRINK
            | fcntl.F_SEAL_SEAL
        )
        fcntl.fcntl(descriptor, fcntl.F_ADD_SEALS, seals)
        if fcntl.fcntl(descriptor, fcntl.F_GET_SEALS) & seals != seals:
            raise OSError("component seals were not retained")
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


def _read_fd(descriptor: int) -> bytes:
    status = os.fstat(descriptor)
    if not stat.S_ISREG(status.st_mode) or not 0 < status.st_size <= MAX_COMPONENT_BYTES:
        raise BrokerUnavailable(
            "BROKER_COMPONENT_INVALID",
            "sealed broker component is not a bounded regular file",
        )
    raw = os.pread(descriptor, status.st_size + 1, 0)
    if len(raw) != status.st_size:
        raise BrokerUnavailable(
            "BROKER_COMPONENT_CHANGED",
            "sealed broker component did not retain its exact size",
        )
    return raw


def _load_exact_backend(
    loader_raw: bytes,
    backend_raw: bytes,
    manifest_raw: bytes,
) -> dict[str, Any]:
    loader_name = "sealed_capability_loader"
    backend_name = "sealed_linux_oci_backend"
    loader_module = ModuleType(loader_name)
    backend_module = ModuleType(backend_name)
    sys.modules[loader_name] = loader_module
    try:
        exec(  # noqa: S102  # nosec B102
            compile(
                loader_raw.decode("utf-8", errors="strict"),
                "<sealed-capability-loader>",
                "exec",
                dont_inherit=True,
            ),
            loader_module.__dict__,
        )
        manifest = loader_module.__dict__["parse_import_manifest"](manifest_raw)
        loader_module.__dict__["validate_source_closure"](backend_raw, manifest)
        loader_module.__dict__["_validate_backend_structure"](backend_raw, manifest)
        sys.modules[backend_name] = backend_module
        exec(  # noqa: S102  # nosec B102
            compile(
                backend_raw.decode("utf-8", errors="strict"),
                "<sealed-linux-oci-backend>",
                "exec",
                dont_inherit=True,
            ),
            backend_module.__dict__,
        )
    except Exception as error:
        sys.modules.pop(loader_name, None)
        sys.modules.pop(backend_name, None)
        raise BrokerUnavailable(
            "BROKER_BACKEND_LOAD_FAILED",
            "approved backend could not be loaded from sealed bytes",
        ) from error
    return backend_module.__dict__


def _worker_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--isolated-broker-worker", action="store_true", required=True)
    for name in WORKER_COMPONENTS:
        parser.add_argument(f"--{name.replace('_', '-')}-fd", type=int, required=True)
        parser.add_argument(
            f"--{name.replace('_', '-')}-sha256",
            required=True,
        )
    parser.add_argument("--state-root-fd", type=int, required=True)
    parser.add_argument("--cgroup-root-fd", type=int, required=True)
    return parser


def _unavailable_payload(code: str, message: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "backend_kind": "linux_oci",
        "status": "unavailable",
        "conformance_status": "non-passing",
        "diagnostics": [
            {
                "code": code,
                "severity": "error",
                "message": message,
            }
        ],
    }


def _worker_error_payload(code: str, message: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "protocol": WORKER_PROTOCOL,
        "error": {
            "code": code,
            "message": message,
        },
    }


def _worker_success_payload(results: Sequence[CommandResult]) -> dict[str, Any]:
    if len(results) != len(OPERATION_IDS):
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "worker produced the wrong operation count",
        )
    return {
        "schema_version": 1,
        "protocol": WORKER_PROTOCOL,
        "results": {
            operation_id: {
                "returncode": result.returncode,
                "stdout_base64": base64.b64encode(result.stdout).decode("ascii"),
                "stderr_base64": base64.b64encode(result.stderr).decode("ascii"),
            }
            for operation_id, result in zip(OPERATION_IDS, results, strict=True)
        },
    }


def _isolated_worker(argv: Sequence[str]) -> int:
    arguments = _worker_parser().parse_args(argv)
    values: dict[str, bytes] = {}
    for name in WORKER_COMPONENTS:
        argument = name.replace("_", "-")
        descriptor = getattr(arguments, f"{name}_fd")
        raw = _read_fd(descriptor)
        if _sha256(raw) != getattr(arguments, f"{name}_sha256"):
            raise BrokerUnavailable(
                "BROKER_COMPONENT_DIGEST_MISMATCH",
                f"sealed broker component {argument} has the wrong digest",
            )
        values[name] = raw
    behavior = parse_behavior_manifest(
        values["behavior"],
        values["behavior_schema"],
    )
    identity = _strict_document(
        values["identity"],
        code="BROKER_IDENTITY_INVALID",
    )
    runtime = _identity_mapping(identity, "runtime")
    oci_runtime = _identity_mapping(identity, "oci_runtime")
    conmon = _identity_mapping(identity, "conmon")
    if (
        not sys.platform.startswith("linux")
        or os.uname().machine not in {"x86_64", "amd64"}
    ):
        raise BrokerUnavailable(
            "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            "Linux amd64 is required for the isolated capability worker",
        )
    if os.geteuid() == 0:
        raise BrokerUnavailable(
            "LINUX_OCI_ROOT_USER_FORBIDDEN",
            "Linux OCI capability preflight must run as a non-root user",
        )
    runtime_fd = -1
    oci_runtime_fd = -1
    conmon_fd = -1
    state_fds: Mapping[str, int] = MappingProxyType({})
    try:
        runtime_fd = open_verified_binary(
            str(runtime.get("path")),
            str(runtime.get("sha256")),
        )
        validate_static_runtime_elf(runtime_fd)
        oci_runtime_fd = open_verified_binary(
            str(oci_runtime.get("path")),
            str(oci_runtime.get("sha256")),
        )
        conmon_fd = open_verified_binary(
            str(conmon.get("path")),
            str(conmon.get("sha256")),
        )
        state_fds = open_private_state_children(arguments.state_root_fd)
        commands = render_operations(
            behavior,
            identity,
            runtime_fd=runtime_fd,
            oci_runtime_fd=oci_runtime_fd,
            state_fds=state_fds,
        )
        execution = behavior["execution"]
        cleanup = execution["cleanup"]
        results = [
            run_retained_command(
                command,
                cgroup_root_fd=arguments.cgroup_root_fd,
                timeout_ms=execution["timeout_ms"],
                output_limit=execution["combined_output_bytes"],
                cleanup_timeout_ms=cleanup["cleanup_timeout_ms"],
            )
            for command in commands
        ]
        print(
            json.dumps(
                _worker_success_payload(results),
                sort_keys=True,
                separators=(",", ":"),
            )
        )
        return 0
    finally:
        if runtime_fd >= 0:
            os.close(runtime_fd)
        if oci_runtime_fd >= 0:
            os.close(oci_runtime_fd)
        if conmon_fd >= 0:
            os.close(conmon_fd)
        for descriptor in state_fds.values():
            os.close(descriptor)


def _collect_worker(
    process: subprocess.Popen[bytes],
    *,
    timeout_seconds: float,
) -> tuple[bytes, bytes]:
    if process.stdout is None or process.stderr is None:
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "isolated broker worker streams are unavailable",
        )
    stdout_fd = process.stdout.fileno()
    stderr_fd = process.stderr.fileno()
    accumulator = OutputAccumulator(limit=MAX_WORKER_OUTPUT_BYTES)
    selected = selectors.DefaultSelector()
    for descriptor in (stdout_fd, stderr_fd):
        os.set_blocking(descriptor, False)
        selected.register(descriptor, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout_seconds
    try:
        while selected.get_map() or process.poll() is None:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except OSError:
                    process.kill()
                process.wait()
                raise BrokerUnavailable(
                    "BROKER_WORKER_TIMEOUT",
                    "isolated broker worker exceeded its deadline",
                )
            for key, _mask in selected.select(min(remaining, 0.05)):
                try:
                    chunk = os.read(key.fd, READ_CHUNK_BYTES)
                except BlockingIOError:
                    continue
                if chunk:
                    if key.fd == stdout_fd:
                        accumulator.add_stdout(chunk)
                    else:
                        accumulator.add_stderr(chunk)
                else:
                    selected.unregister(key.fd)
        process.wait()
        return accumulator.stdout, accumulator.stderr
    except Exception:
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except OSError:
                process.kill()
            process.wait()
        raise
    finally:
        selected.close()


def _emergency_cleanup_known_cgroups(cgroup_root_fd: int) -> None:
    """Clean the only two possible cgroups after an outer-worker failure."""

    flags = (
        os.O_RDONLY
        | os.O_CLOEXEC
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    for name in (
        "capability-preflight-runtime-version",
        "capability-preflight-image-inspect",
    ):
        try:
            child_fd = os.open(name, flags, dir_fd=cgroup_root_fd)
        except FileNotFoundError:
            continue
        except OSError as error:
            raise BrokerUnavailable(
                "BROKER_CGROUP_RECOVERY_FAILED",
                "broker parent could not retain a known command cgroup",
            ) from error
        cleanup_command_cgroup(
            CgroupHandle(
                root_fd=cgroup_root_fd,
                child_fd=child_fd,
                name=name,
            ),
            process_group=None,
            timeout_seconds=5.0,
        )


def _validate_protected_root_descriptors(
    state_root_fd: int,
    cgroup_root_fd: int,
) -> None:
    if os.geteuid() == 0:
        raise BrokerUnavailable(
            "LINUX_OCI_ROOT_USER_FORBIDDEN",
            "Linux OCI capability preflight must run as a non-root user",
        )
    if os.uname().machine not in {"x86_64", "amd64"}:
        raise BrokerUnavailable(
            "LINUX_OCI_HOST_PLATFORM_MISMATCH",
            "Linux amd64 is required for this OCI backend identity",
        )
    if (
        type(state_root_fd) is not int
        or type(cgroup_root_fd) is not int
        or state_root_fd < 3
        or cgroup_root_fd < 3
        or state_root_fd == cgroup_root_fd
    ):
        raise BrokerUnavailable(
            "BROKER_ROOT_DESCRIPTOR_INVALID",
            "distinct retained state and cgroup root descriptors are required",
        )
    try:
        state_status = os.fstat(state_root_fd)
        cgroup_status = os.fstat(cgroup_root_fd)
    except OSError as error:
        raise BrokerUnavailable(
            "BROKER_ROOT_DESCRIPTOR_INVALID",
            "protected root descriptor is unavailable",
        ) from error
    if not _private_directory_status(state_status, owner=os.geteuid()):
        raise BrokerUnavailable(
            "BROKER_STATE_ROOT_INVALID",
            "retained state root must be a private owned directory",
        )
    if not stat.S_ISDIR(cgroup_status.st_mode):
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated cgroup root must be a retained directory",
        )
    validate_cgroup2_descriptor(cgroup_root_fd)
    controllers = _word_set(_read_small_at(cgroup_root_fd, "cgroup.controllers"))
    enabled = _word_set(_read_small_at(cgroup_root_fd, "cgroup.subtree_control"))
    if not REQUIRED_CGROUP_CONTROLLERS.issubset(controllers) or not (
        REQUIRED_CGROUP_CONTROLLERS.issubset(enabled)
    ):
        raise BrokerUnavailable(
            "BROKER_CGROUP_DELEGATION_INVALID",
            "delegated cpu, memory, and pids cgroup v2 controllers are required",
        )
    validate_kernel_seccomp()


def _decode_worker_result(
    value: object,
    *,
    output_limit: int,
) -> CommandResult:
    if not isinstance(value, Mapping) or set(value) != {
        "returncode",
        "stdout_base64",
        "stderr_base64",
    }:
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "worker command result is not closed",
        )
    returncode = value.get("returncode")
    if type(returncode) is not int or not -(2**31) <= returncode < 2**31:
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "worker command return code is invalid",
        )
    encoded = (value.get("stdout_base64"), value.get("stderr_base64"))
    if not all(isinstance(item, str) for item in encoded):
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "worker command streams are invalid",
        )
    try:
        stdout = base64.b64decode(encoded[0], validate=True)
        stderr = base64.b64decode(encoded[1], validate=True)
    except (ValueError, TypeError) as error:
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "worker command stream encoding is invalid",
        ) from error
    if len(stdout) + len(stderr) > output_limit:
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "worker command streams exceed the authorized ceiling",
        )
    return CommandResult(returncode=returncode, stdout=stdout, stderr=stderr)


def _parse_worker_protocol(
    raw: bytes,
    *,
    returncode: int,
    output_limit: int,
) -> tuple[Mapping[str, CommandResult] | None, BrokerUnavailable | None]:
    document = _strict_document(raw, code="BROKER_WORKER_PROTOCOL_INVALID")
    if document.get("schema_version") != 1 or document.get(
        "protocol"
    ) != WORKER_PROTOCOL:
        raise BrokerUnavailable(
            "BROKER_WORKER_PROTOCOL_INVALID",
            "isolated worker protocol identity is invalid",
        )
    if returncode == 0:
        if set(document) != {"schema_version", "protocol", "results"}:
            raise BrokerUnavailable(
                "BROKER_WORKER_PROTOCOL_INVALID",
                "successful worker protocol is not closed",
            )
        results = document.get("results")
        if not isinstance(results, Mapping) or set(results) != set(OPERATION_IDS):
            raise BrokerUnavailable(
                "BROKER_WORKER_PROTOCOL_INVALID",
                "successful worker operation results are invalid",
            )
        return (
            MappingProxyType(
                {
                    operation_id: _decode_worker_result(
                        results[operation_id],
                        output_limit=output_limit,
                    )
                    for operation_id in OPERATION_IDS
                }
            ),
            None,
        )
    if returncode == 1:
        if set(document) != {"schema_version", "protocol", "error"}:
            raise BrokerUnavailable(
                "BROKER_WORKER_PROTOCOL_INVALID",
                "failed worker protocol is not closed",
            )
        error = document.get("error")
        if (
            not isinstance(error, Mapping)
            or set(error) != {"code", "message"}
            or not isinstance(error.get("code"), str)
            or not isinstance(error.get("message"), str)
            or not error["code"]
            or not error["message"]
        ):
            raise BrokerUnavailable(
                "BROKER_WORKER_PROTOCOL_INVALID",
                "failed worker error is invalid",
            )
        return None, BrokerUnavailable(error["code"], error["message"])
    raise BrokerUnavailable(
        "BROKER_WORKER_FAILED",
        "isolated broker worker failed",
    )


def run_authorized_preflight(
    bundle_path: Path,
    *,
    approved_digest: str,
    expected_commit_oid: str,
    expected_tree_oid: str,
    repository_root: Path,
    state_root_fd: int,
    cgroup_root_fd: int,
) -> tuple[dict[str, Any], int]:
    """Validate broker authority and run one sealed capability-only worker."""

    if not sys.platform.startswith("linux"):
        raise BrokerUnavailable(
            "BROKER_PLATFORM_UNSUPPORTED",
            "capability broker requires Linux",
        )
    import build_tool_conformance_authority as authority

    try:
        approved = authority.authorize_capability_broker(
            bundle_path,
            approved_digest=approved_digest,
            expected_commit_oid=expected_commit_oid,
            expected_tree_oid=expected_tree_oid,
            repository_root=repository_root,
        )
    except Exception as error:
        code = getattr(error, "code", "BROKER_AUTHORITY_INVALID")
        message = getattr(error, "message", "capability broker authority is invalid")
        raise BrokerUnavailable(code, message) from error
    _validate_protected_root_descriptors(state_root_fd, cgroup_root_fd)
    behavior = parse_behavior_manifest(
        approved.components["capability_broker_manifest"],
        approved.components["capability_broker_schema"],
    )
    identity = _strict_document(
        approved.components["linux_backend_identity"],
        code="BROKER_IDENTITY_INVALID",
    )
    backend = _load_exact_backend(
        approved.components["preflight_loader"],
        approved.components["linux_preflight_backend"],
        approved.components["preflight_import_manifest"],
    )
    authorization = {
        "authorization_scope": "linux_capability_preflight_broker_v1",
        "authority_sha256": approved.bundle_digest,
        "source_commit": expected_commit_oid,
        "source_tree": expected_tree_oid,
    }
    component_values = {
        "broker": approved.components["capability_broker"],
        "identity": approved.components["linux_backend_identity"],
        "behavior_schema": approved.components["capability_broker_schema"],
        "behavior": approved.components["capability_broker_manifest"],
    }
    sealed: dict[str, int] = {}
    try:
        for name, raw in component_values.items():
            sealed[name] = _sealed_memfd(f"build-tool-{name}", raw)
        command = [
            sys.executable,
            "-I",
            "-S",
            "-B",
            _proc_fd(sealed["broker"]),
            "--isolated-broker-worker",
        ]
        for name in WORKER_COMPONENTS:
            argument = name.replace("_", "-")
            command.extend(
                [
                    f"--{argument}-fd",
                    str(sealed[name]),
                    f"--{argument}-sha256",
                    _sha256(component_values[name]),
                ]
            )
        command.extend(
            [
                "--state-root-fd",
                str(state_root_fd),
                "--cgroup-root-fd",
                str(cgroup_root_fd),
            ]
        )
        pass_fds = (
            *sealed.values(),
            state_root_fd,
            cgroup_root_fd,
        )
        process = subprocess.Popen(  # nosec B603
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd="/",
            env={
                "HOME": "/nonexistent",
                "LANG": "C.UTF-8",
                "LC_ALL": "C.UTF-8",
                "PATH": "/nonexistent",
                "TZ": "UTC",
            },
            close_fds=True,
            pass_fds=pass_fds,
            start_new_session=True,
        )
        try:
            stdout, stderr = _collect_worker(
                process,
                timeout_seconds=WORKER_TIMEOUT_SECONDS,
            )
        finally:
            _emergency_cleanup_known_cgroups(cgroup_root_fd)
        if stderr:
            raise BrokerUnavailable(
                "BROKER_WORKER_PROTOCOL_INVALID",
                "isolated broker worker wrote to standard error",
            )
        worker_results, worker_error = _parse_worker_protocol(
            stdout,
            returncode=int(process.returncode),
            output_limit=int(behavior["execution"]["combined_output_bytes"]),
        )
        if worker_error is not None:
            return {
                **_unavailable_payload(worker_error.code, worker_error.message),
                **authorization,
            }, 1
        if worker_results is None:
            raise BrokerUnavailable(
                "BROKER_WORKER_PROTOCOL_INVALID",
                "isolated worker omitted command results",
            )
        backend_result = backend["CommandResult"]
        try:
            summary = backend["preflight_brokered"](
                identity,
                runtime_info=backend_result(
                    returncode=worker_results["runtime_version"].returncode,
                    stdout=worker_results["runtime_version"].stdout,
                    stderr=worker_results["runtime_version"].stderr,
                ),
                image_inspect=backend_result(
                    returncode=worker_results["image_inspect"].returncode,
                    stdout=worker_results["image_inspect"].stdout,
                    stderr=worker_results["image_inspect"].stderr,
                ),
                platform_name="linux",
                effective_uid=os.geteuid(),
            )
        except backend["LinuxOciUnavailable"] as error:
            return {
                **_unavailable_payload(error.code, error.message),
                **authorization,
            }, 1
        return {**summary, **authorization}, 0
    finally:
        for descriptor in sealed.values():
            os.close(descriptor)


def _external_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run exact retained-FD Linux capability preflight."
    )
    parser.add_argument("--authority-bundle", type=Path, required=True)
    parser.add_argument("--approved-authority-sha256", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-tree", required=True)
    parser.add_argument("--repository-root", type=Path, required=True)
    parser.add_argument("--state-root-fd", type=int, required=True)
    parser.add_argument("--cgroup-root-fd", type=int, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    selected = list(argv if argv is not None else sys.argv[1:])
    if "--isolated-broker-worker" in selected:
        try:
            return _isolated_worker(selected)
        except BrokerUnavailable as error:
            print(
                json.dumps(
                    _worker_error_payload(error.code, error.message),
                    sort_keys=True,
                    separators=(",", ":"),
                )
            )
            return 1
    try:
        arguments = _external_parser().parse_args(selected)
        output, exit_code = run_authorized_preflight(
            arguments.authority_bundle,
            approved_digest=arguments.approved_authority_sha256,
            expected_commit_oid=arguments.source_commit,
            expected_tree_oid=arguments.source_tree,
            repository_root=arguments.repository_root,
            state_root_fd=arguments.state_root_fd,
            cgroup_root_fd=arguments.cgroup_root_fd,
        )
        print(json.dumps(output, sort_keys=True, separators=(",", ":")))
        return exit_code
    except BrokerUnavailable as error:
        print(
            json.dumps(
                {
                    "code": error.code,
                    "message": error.message,
                    "status": "error",
                },
                sort_keys=True,
                separators=(",", ":"),
            ),
            file=sys.stderr,
        )
        return 2
    except SystemExit as error:
        return int(error.code)


if __name__ == "__main__":
    raise SystemExit(main())
