#!/usr/bin/env python3
"""Bound the opt-in Android Flutter failure collector and own its children.

The caller supplies one absolute Linux boot-clock deadline measured before
starting this interpreter. The trusted collector runs in a fresh session. This
helper becomes its child subreaper, keeps the session leader unreaped until
the final group signal, then reaps adopted descendants inside that same bound.
"""

from __future__ import annotations

import argparse
import ctypes
import errno
import os
import select
import signal
import sys
import time
from dataclasses import dataclass
from typing import Sequence

PR_SET_CHILD_SUBREAPER = 36
POLL_SECONDS = 0.01
TERM_GRACE_SECONDS = 0.70
REAP_RESERVE_SECONDS = 0.20

RESULT_COMPLETE = 0
RESULT_TIMED_OUT_CLEANED = 124
RESULT_DESCENDANTS_CLEANED = 125
RESULT_CLEANUP_INCOMPLETE = 126
RESULT_UNAVAILABLE = 127
RESULT_WORKER_FAILED_CLEANED = 128

_cancel_signal: int | None = None


@dataclass(frozen=True)
class ProcStat:
    pid: int
    state: str
    ppid: int
    pgrp: int
    session: int
    starttime: int


def _on_cancel(signum: int, _frame: object) -> None:
    global _cancel_signal
    _cancel_signal = signum


def _clock() -> float:
    clock_id = getattr(time, "CLOCK_BOOTTIME", None)
    if clock_id is None:
        raise RuntimeError("CLOCK_BOOTTIME is unavailable")
    return time.clock_gettime(clock_id)


def _enable_subreaper() -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    result = libc.prctl(
        ctypes.c_int(PR_SET_CHILD_SUBREAPER),
        ctypes.c_ulong(1),
        ctypes.c_ulong(0),
        ctypes.c_ulong(0),
        ctypes.c_ulong(0),
    )
    if result != 0:
        error_number = ctypes.get_errno()
        raise OSError(error_number, os.strerror(error_number))


def _read_stat(pid: int) -> ProcStat | None:
    try:
        with open(f"/proc/{pid}/stat", encoding="ascii") as stream:
            raw = stream.read()
    except FileNotFoundError:
        return None
    close_paren = raw.rfind(")")
    if close_paren < 0:
        raise ValueError("malformed proc stat command field")
    fields = raw[close_paren + 2 :].split()
    if len(fields) < 20:
        raise ValueError("truncated proc stat fields")
    return ProcStat(
        pid=int(raw[:close_paren].split()[0]),
        state=fields[0],
        ppid=int(fields[1]),
        pgrp=int(fields[2]),
        session=int(fields[3]),
        starttime=int(fields[19]),
    )


def _peek_exit(pid: int) -> int | None:
    """Read a child's exit status without freeing its PID/PGID."""

    result = os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if result is None or result.si_pid != pid:
        return None
    if result.si_code == os.CLD_EXITED:
        return int(result.si_status)
    return -int(result.si_status)


def _signal_owned(leader_pid: int, leader_starttime: int, signum: int) -> bool:
    """Signal only the pinned private process group."""

    try:
        leader = _read_stat(leader_pid)
    except (OSError, ValueError):
        return False
    if (
        leader is None
        or leader.starttime != leader_starttime
        or leader.pgrp != leader_pid
        or leader.session != leader_pid
    ):
        return False
    try:
        os.killpg(leader_pid, signum)
    except ProcessLookupError:
        return True
    except OSError as error:
        return error.errno == errno.ESRCH
    return True


def _reap_to_deadline(leader_pid: int, deadline: float) -> tuple[bool, bool]:
    """Reap all adopted descendants; report incomplete on deadline or error."""

    descendants_reaped = False
    while _clock() < deadline:
        # The worker leader has already been pinned through its final group
        # signal. Reaping any child is now safe: no future signal uses its PGID.
        while True:
            if _clock() >= deadline:
                return False, descendants_reaped
            try:
                pid, _status = os.waitpid(-1, os.WNOHANG)
            except ChildProcessError:
                return True, descendants_reaped
            except OSError:
                return False, descendants_reaped
            if pid <= 0:
                break
            descendants_reaped = descendants_reaped or pid != leader_pid
        # waitpid returning 0 means a child is still alive. Keep polling; the
        # collector's verified private group was already signalled.
        time.sleep(min(POLL_SECONDS, max(0.0, deadline - _clock())))
    return False, descendants_reaped


def _wait_ready(fd: int, deadline: float) -> bool:
    while _clock() < deadline:
        remaining = max(0.0, deadline - _clock())
        readable, _, _ = select.select([fd], [], [], min(POLL_SECONDS, remaining))
        if readable:
            try:
                return os.read(fd, 1) == b"1"
            except OSError:
                return False
    return False


def _child_exec(command: Sequence[str], ready_fd: int, release_fd: int) -> None:
    try:
        os.setsid()
        os.write(ready_fd, b"1")
        os.close(ready_fd)
        # Do not execute the collector until the parent has validated the
        # private session identity and confirmed that work time remains.
        if os.read(release_fd, 1) != b"1":
            os._exit(125)
        os.close(release_fd)
        os.execvpe(command[0], list(command), os.environ.copy())
    except BaseException:  # noqa: BLE001 - a forked child cannot unwind safely
        try:
            os.write(ready_fd, b"0")
        except OSError:
            pass
        os._exit(127)


def _reap_unisolated_child(pid: int, deadline: float) -> bool:
    """Fail safely if setsid/exec setup failed before group ownership."""

    try:
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    except OSError:
        return False
    while _clock() < deadline:
        try:
            waited, _status = os.waitpid(pid, os.WNOHANG)
        except ChildProcessError:
            return True
        except OSError:
            return False
        if waited == pid:
            return True
        time.sleep(min(POLL_SECONDS, max(0.0, deadline - _clock())))
    return False


def supervise(command: Sequence[str], deadline_cs: int) -> int:
    global _cancel_signal
    _cancel_signal = None
    if (
        not sys.platform.startswith("linux")
        or not hasattr(os, "fork")
        or not hasattr(os, "waitid")
        or not hasattr(os, "WNOWAIT")
    ):
        print("diagnostic_supervisor=unavailable")
        return RESULT_UNAVAILABLE
    try:
        deadline = deadline_cs / 100.0
        work_deadline = deadline - 1.0
        if _clock() >= work_deadline:
            raise RuntimeError("deadline already elapsed")
        _enable_subreaper()
    except (OSError, RuntimeError, ValueError):
        print("diagnostic_supervisor=unavailable")
        return RESULT_UNAVAILABLE

    old_handlers: dict[int, object] = {}
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        old_handlers[signum] = signal.signal(signum, _on_cancel)
    old_handlers[signal.SIGCHLD] = signal.signal(signal.SIGCHLD, signal.SIG_DFL)

    def restore_handlers() -> None:
        for signum, handler in old_handlers.items():
            signal.signal(signum, handler)

    created_pipes: list[int] = []
    try:
        ready_read, ready_write = os.pipe()
        created_pipes.extend((ready_read, ready_write))
        release_read, release_write = os.pipe()
        created_pipes.extend((release_read, release_write))
    except OSError:
        for fd in created_pipes:
            os.close(fd)
        restore_handlers()
        print("diagnostic_supervisor=worker_pipe_failed")
        return RESULT_UNAVAILABLE
    try:
        leader_pid = os.fork()
    except OSError:
        for fd in (ready_read, ready_write, release_read, release_write):
            os.close(fd)
        restore_handlers()
        print("diagnostic_supervisor=worker_fork_failed")
        return RESULT_UNAVAILABLE
    if leader_pid == 0:
        os.close(ready_read)
        os.close(release_write)
        _child_exec(command, ready_write, release_read)
        os._exit(127)

    os.close(ready_write)
    os.close(release_read)
    released = False
    descendants_reaped = False
    try:
        ready = _wait_ready(ready_read, work_deadline)
        os.close(ready_read)
        ready_read = -1
        if ready:
            initial = _read_stat(leader_pid)
        else:
            initial = None
        if (
            not ready
            or initial is None
            or initial.pid != leader_pid
            or initial.pgrp != leader_pid
            or initial.session != leader_pid
            or _clock() >= work_deadline
            or _cancel_signal is not None
        ):
            os.close(release_write)
            release_write = -1
            cleaned = _reap_unisolated_child(leader_pid, deadline)
            label = "startup_failed_cleaned" if cleaned else "startup_cleanup_incomplete"
            restore_handlers()
            print(f"diagnostic_supervisor={label}")
            return RESULT_UNAVAILABLE if cleaned else RESULT_CLEANUP_INCOMPLETE

        leader_starttime = initial.starttime
        # Treat an interrupted write as potentially released. The outer error
        # path then uses the already verified group identity, never a raw PID.
        released = True
        os.write(release_write, b"1")
        os.close(release_write)
        release_write = -1

        worker_exit: int | None = None
        worker_timed_out = False
        ownership_lost = False
        observer_failed = False
        try:
            while _clock() < work_deadline:
                if _cancel_signal is not None:
                    worker_timed_out = True
                    break
                worker_exit = _peek_exit(leader_pid)
                if worker_exit is not None:
                    break
                time.sleep(min(POLL_SECONDS, max(0.0, work_deadline - _clock())))
            if worker_exit is None:
                worker_exit = _peek_exit(leader_pid)
            if worker_exit is None:
                worker_timed_out = True
        except ChildProcessError:
            # ECHILD means the leader is no longer ours to pin. Never signal
            # its numeric process-group ID after this ownership loss.
            ownership_lost = True
        except BaseException:  # noqa: BLE001 - cleanup must precede process exit
            observer_failed = True

        if ownership_lost:
            restore_handlers()
            print("diagnostic_supervisor=worker_ownership_lost")
            return RESULT_CLEANUP_INCOMPLETE

        if observer_failed:
            # No waitpid has run; the verified leader is still our unreaped
            # child, so its private group remains pinned for this one cleanup.
            try:
                cleanup_ok = _signal_owned(
                    leader_pid, leader_starttime, signal.SIGKILL
                )
            except BaseException:  # noqa: BLE001 - pinned but cleanup is uncertain
                cleanup_ok = False
            try:
                reaped, _ = _reap_to_deadline(leader_pid, deadline)
                cleanup_ok = cleanup_ok and reaped
            except BaseException:  # noqa: BLE001 - never signal after reaping starts
                cleanup_ok = False
            restore_handlers()
            print("diagnostic_supervisor=post_release_exception")
            return RESULT_CLEANUP_INCOMPLETE

        # The signal phase ends before the first waitpid. Once reaping begins,
        # no code path may signal this process-group number again.
        try:
            if worker_timed_out:
                cleanup_ok = _signal_owned(leader_pid, leader_starttime, signal.SIGTERM)
                kill_at = min(deadline - REAP_RESERVE_SECONDS, _clock() + TERM_GRACE_SECONDS)
                while _clock() < kill_at and _cancel_signal is None:
                    time.sleep(min(POLL_SECONDS, max(0.0, kill_at - _clock())))
            else:
                cleanup_ok = True
            final_signal_ok = _signal_owned(
                leader_pid, leader_starttime, signal.SIGKILL
            )
            cleanup_ok = cleanup_ok and final_signal_ok
        except BaseException:  # noqa: BLE001 - still pre-reap; try one pinned final signal
            cleanup_ok = False
            try:
                final_signal_ok = _signal_owned(
                    leader_pid, leader_starttime, signal.SIGKILL
                )
                cleanup_ok = cleanup_ok and final_signal_ok
            except BaseException:  # noqa: BLE001 - report incomplete without PID fallback
                pass

        try:
            cleaned, descendants_reaped = _reap_to_deadline(leader_pid, deadline)
            cleanup_ok = cleanup_ok and cleaned
        except BaseException:  # noqa: BLE001 - after reaping starts, never signal again
            cleanup_ok = False
    except BaseException:  # noqa: BLE001 - pre-release worker is held at the gate
        if ready_read >= 0:
            os.close(ready_read)
        if release_write >= 0:
            os.close(release_write)
        if released:
            try:
                cleanup_ok = _signal_owned(
                    leader_pid, leader_starttime, signal.SIGKILL
                )
            except BaseException:  # noqa: BLE001 - ownership remains pinned
                cleanup_ok = False
            try:
                reaped, _ = _reap_to_deadline(leader_pid, deadline)
                cleanup_ok = reaped and cleanup_ok
            except BaseException:  # noqa: BLE001 - never signal after reap begins
                cleanup_ok = False
            restore_handlers()
            print("diagnostic_supervisor=startup_exception_cleanup_" + ("complete" if cleanup_ok else "incomplete"))
            return RESULT_UNAVAILABLE if cleanup_ok else RESULT_CLEANUP_INCOMPLETE
        # Before release the command has not executed and cannot have children;
        # this direct-child cleanup therefore cannot target an unrelated PID.
        cleaned = _reap_unisolated_child(leader_pid, deadline)
        label = "startup_failed_cleaned" if cleaned else "startup_cleanup_incomplete"
        restore_handlers()
        print(f"diagnostic_supervisor={label}")
        return RESULT_UNAVAILABLE if cleaned else RESULT_CLEANUP_INCOMPLETE

    restore_handlers()

    if not cleanup_ok:
        print("diagnostic_supervisor=cleanup_incomplete")
        return RESULT_CLEANUP_INCOMPLETE
    if worker_timed_out:
        print("diagnostic_supervisor=timed_out_cleaned")
        return RESULT_TIMED_OUT_CLEANED
    if descendants_reaped:
        print("diagnostic_supervisor=descendants_cleaned")
        return RESULT_DESCENDANTS_CLEANED
    if worker_exit == 0:
        print("diagnostic_supervisor=completed")
        return RESULT_COMPLETE
    print("diagnostic_supervisor=worker_failed_cleaned")
    return RESULT_WORKER_FAILED_CLEANED


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--deadline-boottime-cs", type=int, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        print("diagnostic_supervisor=command_missing")
        return RESULT_UNAVAILABLE
    return supervise(command, args.deadline_boottime_cs)


if __name__ == "__main__":
    raise SystemExit(main())
