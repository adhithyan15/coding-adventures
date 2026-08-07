#!/usr/bin/env python3
"""Linux cgroup-v2 guardian for ADJ trusted helper commands.

The verifier owns the write end of a private control pipe.  EOF requests the
same bounded cleanup path used after normal command exit, so verifier death
does not strand descendants that leave the command's original process group.
"""

from __future__ import annotations

import argparse
import ctypes
import errno
import json
import os
import selectors
import signal
import sys
import time
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any

CGROUP2_SUPER_MAGIC = 0x63677270
GUARDIAN_CONTRACT = "adj-stdlib/process-guardian/v1"
MAX_CONTROL_BYTES = 4096
PR_SET_CHILD_SUBREAPER = 36
POSIX_SIGKILL = getattr(signal, "SIGKILL", 9)


class GuardianError(RuntimeError):
    """A fail-closed containment setup or cleanup error."""

    def __init__(
        self,
        code: str,
        message: str,
        *,
        cleanup_causes: tuple[GuardianError, ...] = (),
    ) -> None:
        super().__init__(message)
        self.code = code
        self.cleanup_causes = cleanup_causes

    def with_cleanup(self, *causes: GuardianError) -> GuardianError:
        return GuardianError(
            self.code,
            str(self),
            cleanup_causes=(*self.cleanup_causes, *causes),
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "cleanup_causes": [cause.to_dict() for cause in self.cleanup_causes],
            "error": str(self),
            "error_code": self.code,
        }


def _append_failure(
    primary: GuardianError | None, failure: GuardianError
) -> GuardianError:
    return failure if primary is None else primary.with_cleanup(failure)


def _guardian_failure(
    error: BaseException, *, code: str, message: str
) -> GuardianError:
    if isinstance(error, GuardianError):
        return error
    failure = GuardianError(code, message)
    failure.__cause__ = error
    return failure


@dataclass(frozen=True)
class CgroupHandle:
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


def canonical_json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def _open_at(root_fd: int, name: str, flags: int) -> int:
    return os.open(
        name,
        flags | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0),
        dir_fd=root_fd,
    )


def _read_small_at(root_fd: int, name: str) -> bytes:
    try:
        descriptor = _open_at(root_fd, name, os.O_RDONLY)
        try:
            value = os.read(descriptor, MAX_CONTROL_BYTES + 1)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise GuardianError(
            "CGROUP_CONTROL_READ_FAILED",
            f"could not read delegated cgroup control {name}",
        ) from error
    if len(value) > MAX_CONTROL_BYTES:
        raise GuardianError(
            "CGROUP_CONTROL_OVERSIZE",
            f"delegated cgroup control {name} exceeds its byte ceiling",
        )
    return value


def _write_small_at(root_fd: int, name: str, value: bytes) -> None:
    try:
        descriptor = _open_at(root_fd, name, os.O_WRONLY)
        try:
            written = os.write(descriptor, value)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise GuardianError(
            "CGROUP_CONTROL_WRITE_FAILED",
            f"could not write delegated cgroup control {name}",
        ) from error
    if written != len(value):
        raise GuardianError(
            "CGROUP_CONTROL_SHORT_WRITE",
            f"delegated cgroup control {name} accepted a short write",
        )


def cgroup_is_empty(raw: bytes) -> bool:
    try:
        lines = raw.decode("ascii", errors="strict").splitlines()
    except UnicodeDecodeError as error:
        raise GuardianError(
            "CGROUP_EVENTS_INVALID", "cgroup.events is not strict ASCII"
        ) from error
    populated: list[str] = []
    for line in lines:
        fields = line.split()
        if len(fields) != 2:
            raise GuardianError(
                "CGROUP_EVENTS_INVALID", "cgroup.events contains an invalid field"
            )
        if fields[0] == "populated":
            populated.append(fields[1])
    if len(populated) != 1 or populated[0] not in {"0", "1"}:
        raise GuardianError(
            "CGROUP_EVENTS_INVALID",
            "cgroup.events must contain one exact populated field",
        )
    return populated[0] == "0"


def validate_cgroup2_descriptor(descriptor: int) -> None:
    result = _LinuxStatFs()
    library = ctypes.CDLL(None, use_errno=True)
    if library.fstatfs(ctypes.c_int(descriptor), ctypes.byref(result)) != 0:
        selected_errno = ctypes.get_errno()
        raise GuardianError(
            "CGROUP_DELEGATION_INVALID",
            "delegated cgroup root filesystem could not be identified",
        ) from OSError(selected_errno, os.strerror(selected_errno))
    if result.f_type != CGROUP2_SUPER_MAGIC:
        raise GuardianError(
            "CGROUP_DELEGATION_INVALID",
            "delegated command root is not a cgroup v2 filesystem",
        )


def _enable_subreaper() -> None:
    library = ctypes.CDLL(None, use_errno=True)
    result = library.prctl(
        ctypes.c_int(PR_SET_CHILD_SUBREAPER),
        ctypes.c_ulong(1),
        ctypes.c_ulong(0),
        ctypes.c_ulong(0),
        ctypes.c_ulong(0),
    )
    if result != 0:
        selected_errno = ctypes.get_errno()
        raise GuardianError(
            "SUBREAPER_UNAVAILABLE", "guardian could not become a child subreaper"
        ) from OSError(selected_errno, os.strerror(selected_errno))


def create_command_cgroup(root_fd: int) -> CgroupHandle:
    validate_cgroup2_descriptor(root_fd)
    name = f"adj-provenance-{os.getpid()}-{time.monotonic_ns():x}"
    created = False
    try:
        os.mkdir(name, mode=0o700, dir_fd=root_fd)
        created = True
        child_fd = _open_at(
            root_fd,
            name,
            os.O_RDONLY | getattr(os, "O_DIRECTORY", 0),
        )
    except OSError as error:
        primary = GuardianError(
            "CGROUP_CREATE_FAILED", "fresh command cgroup could not be created"
        )
        primary.__cause__ = error
        if created:
            try:
                os.rmdir(name, dir_fd=root_fd)
            except OSError as rollback_error:
                primary = primary.with_cleanup(
                    _guardian_failure(
                        rollback_error,
                        code="CGROUP_ROLLBACK_REMOVE_FAILED",
                        message="partial command cgroup could not be removed",
                    )
                )
        raise primary from error
    try:
        for control, flags in (
            ("cgroup.kill", os.O_WRONLY),
            ("cgroup.procs", os.O_WRONLY),
            ("cgroup.events", os.O_RDONLY),
        ):
            descriptor = _open_at(child_fd, control, flags)
            os.close(descriptor)
        if not cgroup_is_empty(_read_small_at(child_fd, "cgroup.events")):
            raise GuardianError(
                "CGROUP_CREATE_FAILED", "fresh command cgroup is already populated"
            )
        return CgroupHandle(root_fd=root_fd, child_fd=child_fd, name=name)
    except (GuardianError, OSError) as error:
        primary = _guardian_failure(
            error,
            code="CGROUP_CONTROL_VALIDATION_FAILED",
            message="fresh command cgroup controls could not be validated",
        )
        try:
            os.close(child_fd)
        except OSError as rollback_error:
            primary = primary.with_cleanup(
                _guardian_failure(
                    rollback_error,
                    code="CGROUP_ROLLBACK_CLOSE_FAILED",
                    message="partial command cgroup descriptor could not be closed",
                )
            )
        try:
            os.rmdir(name, dir_fd=root_fd)
        except OSError as rollback_error:
            primary = primary.with_cleanup(
                _guardian_failure(
                    rollback_error,
                    code="CGROUP_ROLLBACK_REMOVE_FAILED",
                    message="partial command cgroup could not be removed",
                )
            )
        raise primary from error


def _reap_children(deadline: float) -> bool:
    while True:
        if time.monotonic() >= deadline:
            return False
        try:
            child, _status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return True
        if child <= 0:
            return True


def cleanup_command_cgroup(
    cgroup: CgroupHandle,
    deadline: float,
    *,
    process_group: int | None = None,
) -> None:
    cleanup_error: GuardianError | None = None
    try:
        _write_small_at(cgroup.child_fd, "cgroup.kill", b"1\n")
    except GuardianError as error:
        cleanup_error = error
    if process_group is not None:
        try:
            os.killpg(process_group, POSIX_SIGKILL)
        except OSError as error:
            if error.errno != errno.ESRCH:
                failure = GuardianError(
                    "PROCESS_GROUP_KILL_FAILED", "supplemental process-group termination failed"
                )
                failure.__cause__ = error
                cleanup_error = _append_failure(cleanup_error, failure)
    empty = False
    while time.monotonic() < deadline:
        try:
            empty = cgroup_is_empty(_read_small_at(cgroup.child_fd, "cgroup.events"))
        except GuardianError as error:
            cleanup_error = _append_failure(cleanup_error, error)
            break
        if empty:
            break
        time.sleep(0.01)
    try:
        os.close(cgroup.child_fd)
    except OSError as error:
        failure = GuardianError(
            "CGROUP_DESCRIPTOR_CLOSE_FAILED",
            "command cgroup descriptor could not be closed",
        )
        failure.__cause__ = error
        cleanup_error = _append_failure(cleanup_error, failure)
    if empty:
        try:
            os.rmdir(cgroup.name, dir_fd=cgroup.root_fd)
        except OSError as error:
            failure = GuardianError(
                "CGROUP_REMOVE_FAILED",
                "empty command cgroup could not be removed",
            )
            failure.__cause__ = error
            cleanup_error = _append_failure(cleanup_error, failure)
    if not empty:
        cleanup_error = _append_failure(
            cleanup_error,
            GuardianError(
                "CGROUP_CLEANUP_TIMEOUT",
                "command cgroup did not become empty before the cleanup deadline",
            ),
        )
    if cleanup_error is not None:
        raise cleanup_error


def _child_exec(command: Sequence[str], gate_fd: int) -> None:
    try:
        os.setsid()
        if os.read(gate_fd, 1) != b"1":
            os._exit(125)
        os.close(gate_fd)
        null_fd = os.open("/dev/null", os.O_RDONLY | os.O_CLOEXEC)
        os.dup2(null_fd, 0)
        if null_fd > 2:
            os.close(null_fd)
        os.execvpe(command[0], list(command), os.environ.copy())
    except BaseException:  # noqa: BLE001 - a pre-exec child cannot unwind safely
        os._exit(127)


def _wait_status_returncode(status: int) -> int:
    return os.waitstatus_to_exitcode(status)


def _monitor(control_fd: int, child_pid: int) -> tuple[int | None, bool]:
    selected = selectors.DefaultSelector()
    os.set_blocking(control_fd, False)
    selected.register(control_fd, selectors.EVENT_READ)
    try:
        while True:
            waited, status = os.waitpid(child_pid, os.WNOHANG)
            if waited == child_pid:
                return _wait_status_returncode(status), False
            for _key, _mask in selected.select(0.05):
                try:
                    control = os.read(control_fd, 1)
                except BlockingIOError:
                    continue
                if control != b"":
                    raise GuardianError(
                        "CONTROL_PROTOCOL_INVALID",
                        "guardian control pipe accepts EOF only",
                    )
                return None, True
    finally:
        selected.close()


def supervise(
    command: Sequence[str],
    *,
    control_fd: int,
    status_fd: int | None = None,
    cgroup_root: str,
    cleanup_timeout_seconds: float,
) -> dict[str, Any]:
    if not sys.platform.startswith("linux") or not hasattr(os, "fork"):
        raise GuardianError(
            "PLATFORM_UNSUPPORTED", "strict process guarding requires Linux"
        )
    if not command:
        raise GuardianError("COMMAND_INVALID", "guarded command must not be empty")
    if cleanup_timeout_seconds <= 0:
        raise GuardianError(
            "CLEANUP_BOUND_INVALID", "guardian cleanup bound must be positive"
        )
    root_flags = (
        os.O_RDONLY
        | os.O_CLOEXEC
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        root_fd = os.open(cgroup_root, root_flags)
    except OSError as error:
        raise GuardianError(
            "CGROUP_DELEGATION_INVALID", "delegated cgroup root could not be opened"
        ) from error
    cgroup: CgroupHandle | None = None
    gate_read = -1
    gate_write = -1
    child_pid: int | None = None
    returncode: int | None = None
    verifier_gone = False
    primary_error: GuardianError | None = None
    try:
        _enable_subreaper()
        cgroup = create_command_cgroup(root_fd)
        gate_read, gate_write = os.pipe2(os.O_CLOEXEC)
        child_pid = os.fork()
        if child_pid == 0:
            os.close(gate_write)
            os.close(control_fd)
            if status_fd is not None:
                os.close(status_fd)
            os.close(root_fd)
            _child_exec(command, gate_read)
            os._exit(127)
        os.close(gate_read)
        gate_read = -1
        _write_small_at(
            cgroup.child_fd, "cgroup.procs", f"{child_pid}\n".encode("ascii")
        )
        if os.write(gate_write, b"1") != 1:
            raise GuardianError(
                "CHILD_RELEASE_FAILED", "contained command could not be released"
            )
        os.close(gate_write)
        gate_write = -1
        returncode, verifier_gone = _monitor(control_fd, child_pid)
    except GuardianError as error:
        primary_error = error
    except OSError as error:
        primary_error = GuardianError(
            "GUARDIAN_RUNTIME_FAILED", "guardian process operation failed"
        )
        primary_error.__cause__ = error
    finally:
        cleanup_deadline = time.monotonic() + cleanup_timeout_seconds
        for descriptor in (gate_read, gate_write):
            if descriptor >= 0:
                try:
                    os.close(descriptor)
                except OSError:
                    pass
        if cgroup is not None:
            try:
                cleanup_command_cgroup(
                    cgroup,
                    cleanup_deadline,
                    process_group=child_pid if returncode is None else None,
                )
            except GuardianError as cleanup_error:
                primary_error = cleanup_error.with_cleanup(
                    *((primary_error,) if primary_error is not None else ())
                )
        if child_pid is not None:
            while True:
                try:
                    waited, status = os.waitpid(child_pid, os.WNOHANG)
                except ChildProcessError:
                    break
                if waited == child_pid:
                    if returncode is None:
                        returncode = _wait_status_returncode(status)
                    break
                if time.monotonic() >= cleanup_deadline:
                    break
                time.sleep(0.01)
            if returncode is None:
                primary_error = _append_failure(
                    primary_error,
                    GuardianError(
                        "ROOT_REAP_FAILED",
                        "guarded root was not reaped before the deadline",
                    ),
                )
        if not _reap_children(cleanup_deadline):
            primary_error = _append_failure(
                primary_error,
                GuardianError(
                    "ADOPTED_REAP_TIMEOUT",
                    "adopted descendants were not reaped before the cleanup deadline",
                ),
            )
        try:
            os.close(root_fd)
        except OSError as error:
            failure = GuardianError(
                "CGROUP_ROOT_CLOSE_FAILED",
                "delegated cgroup root descriptor could not be closed",
            )
            failure.__cause__ = error
            primary_error = _append_failure(primary_error, failure)
    if primary_error is not None:
        raise primary_error
    assert returncode is not None
    return {
        "cleanup_confirmed": True,
        "contract": GUARDIAN_CONTRACT,
        "returncode": returncode,
        "verifier_gone": verifier_gone,
    }


def _write_status(status_fd: int, value: dict[str, Any]) -> None:
    raw = canonical_json_bytes(value)
    if len(raw) > MAX_CONTROL_BYTES:
        raise GuardianError(
            "STATUS_OVERSIZE", "guardian status exceeds its byte ceiling"
        )
    if os.write(status_fd, raw) != len(raw):
        raise GuardianError(
            "STATUS_SHORT_WRITE", "guardian status write was incomplete"
        )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--control-fd", type=int, required=True)
    parser.add_argument("--status-fd", type=int, required=True)
    parser.add_argument("--cgroup-root", required=True)
    parser.add_argument("--cleanup-timeout-seconds", type=float, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    command = list(args.command)
    if command[:1] == ["--"]:
        command = command[1:]
    try:
        status = supervise(
            command,
            control_fd=args.control_fd,
            status_fd=args.status_fd,
            cgroup_root=args.cgroup_root,
            cleanup_timeout_seconds=args.cleanup_timeout_seconds,
        )
        exit_code = 0
    except GuardianError as error:
        status = {
            "cleanup_confirmed": False,
            "contract": GUARDIAN_CONTRACT,
            **error.to_dict(),
        }
        exit_code = 125
    try:
        _write_status(args.status_fd, status)
    except (GuardianError, OSError):
        exit_code = 126
    finally:
        for descriptor in (args.control_fd, args.status_fd):
            try:
                os.close(descriptor)
            except OSError:
                pass
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
