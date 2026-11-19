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
MAX_TRACKED_DESCENDANTS = 256
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


def _proc_table() -> dict[int, ProcStat]:
    table: dict[int, ProcStat] = {}
    with os.scandir("/proc") as entries:
        for entry in entries:
            if not entry.name.isdigit():
                continue
            pid = int(entry.name)
            try:
                stat = _read_stat(pid)
            except (OSError, ValueError):
                continue
            if stat is not None:
                table[pid] = stat
    return table


def _owned_tree(
    table: dict[int, ProcStat], leader_pid: int, supervisor_pid: int
) -> tuple[list[ProcStat], bool]:
    """Enumerate the worker tree and any descendants adopted by this helper."""

    if leader_pid not in table:
        return [], False
    owned = {leader_pid}
    changed = True
    while changed:
        changed = False
        for pid, stat in table.items():
            if pid in owned:
                continue
            if stat.ppid in owned or stat.ppid == supervisor_pid:
                owned.add(pid)
                changed = True
                if len(owned) > MAX_TRACKED_DESCENDANTS + 1:
                    return [table[item] for item in list(owned)[: MAX_TRACKED_DESCENDANTS + 1] if item in table], True
    return [table[pid] for pid in owned if pid in table], False


def _peek_exit(pid: int) -> int | None:
    """Read a child's exit status without freeing its PID/PGID."""

    result = os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if result is None or result.si_pid != pid:
        return None
    if result.si_code == os.CLD_EXITED:
        return int(result.si_status)
    return -int(result.si_status)


def _signal_owned(
    table: dict[int, ProcStat], leader_pid: int, leader_starttime: int, signum: int
) -> bool:
    """Signal the pinned private group and exact descendants that escaped it."""

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
    supervisor_pid = os.getpid()
    owned, truncated = _owned_tree(table, leader_pid, supervisor_pid)
    if truncated:
        return False
    complete = True
    try:
        os.killpg(leader_pid, signum)
    except ProcessLookupError:
        pass
    except OSError as error:
        if error.errno != errno.ESRCH:
            complete = False

    for stat in owned:
        if stat.pid == leader_pid or stat.state == "Z":
            continue
        if stat.pgrp == leader_pid:
            continue  # already covered by the private-group signal
        try:
            current = _read_stat(stat.pid)
        except (OSError, ValueError):
            complete = False
            continue
        if current is None:
            continue
        if current.starttime != stat.starttime:
            complete = False
            continue
        try:
            os.kill(stat.pid, signum)
        except ProcessLookupError:
            pass
        except OSError as error:
            if error.errno != errno.ESRCH:
                complete = False
    return complete


def _kill_adopted_escaped(table: dict[int, ProcStat], leader_pid: int) -> bool:
    """Kill current adopted descendants outside the already-signaled group."""

    complete = True
    supervisor_pid = os.getpid()
    for stat in table.values():
        if stat.ppid != supervisor_pid or stat.pid == leader_pid or stat.state == "Z":
            continue
        if stat.pgrp == leader_pid:
            continue
        try:
            current = _read_stat(stat.pid)
        except (OSError, ValueError):
            complete = False
            continue
        if current is None:
            continue
        if current.starttime != stat.starttime:
            complete = False
            continue
        try:
            os.kill(stat.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except OSError as error:
            if error.errno != errno.ESRCH:
                complete = False
    return complete


def _reap_to_deadline(
    leader_pid: int,
    leader_starttime: int,
    deadline: float,
    *,
    group_signal_sent: bool,
) -> bool:
    """Reap all adopted descendants; report incomplete on deadline or error."""

    while _clock() < deadline:
        try:
            table = _proc_table()
            if not _kill_adopted_escaped(table, leader_pid):
                cleanup_problem = True
            else:
                cleanup_problem = False
        except (OSError, ValueError):
            table = {}
            cleanup_problem = True

        # The worker leader has already been pinned through its final group
        # signal. Reaping any child is now safe: no future signal uses its PGID.
        reaped_any = False
        while True:
            try:
                pid, _status = os.waitpid(-1, os.WNOHANG)
            except ChildProcessError:
                return not cleanup_problem
            except OSError:
                cleanup_problem = True
                break
            if pid <= 0:
                break
            reaped_any = True
        if not cleanup_problem and not reaped_any:
            # waitpid returning 0 means a child is still alive. Keep polling;
            # its exact group was already signaled, and escaped children are
            # rechecked above under their start-time identities.
            pass
        time.sleep(min(POLL_SECONDS, max(0.0, deadline - _clock())))
    return False


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


def _child_exec(command: Sequence[str], ready_fd: int) -> None:
    try:
        os.setsid()
        os.write(ready_fd, b"1")
        os.close(ready_fd)
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
        if _clock() >= deadline:
            raise RuntimeError("deadline already elapsed")
        _enable_subreaper()
    except (OSError, RuntimeError, ValueError):
        print("diagnostic_supervisor=unavailable")
        return RESULT_UNAVAILABLE

    old_handlers: dict[int, object] = {}
    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        old_handlers[signum] = signal.signal(signum, _on_cancel)

    ready_read, ready_write = os.pipe()
    try:
        leader_pid = os.fork()
    except OSError:
        os.close(ready_read)
        os.close(ready_write)
        print("diagnostic_supervisor=worker_fork_failed")
        return RESULT_UNAVAILABLE
    if leader_pid == 0:
        os.close(ready_read)
        _child_exec(command, ready_write)
        os._exit(127)

    os.close(ready_write)
    try:
        ready = _wait_ready(ready_read, deadline - 1.0)
    finally:
        os.close(ready_read)
    try:
        initial = _read_stat(leader_pid)
    except (OSError, ValueError):
        initial = None
    if (
        not ready
        or initial is None
        or initial.pid != leader_pid
        or initial.pgrp != leader_pid
        or initial.session != leader_pid
    ):
        cleaned = _reap_unisolated_child(leader_pid, deadline)
        label = "startup_failed_cleaned" if cleaned else "startup_cleanup_incomplete"
        print(f"diagnostic_supervisor={label}")
        return RESULT_UNAVAILABLE if cleaned else RESULT_CLEANUP_INCOMPLETE

    leader_starttime = initial.starttime
    work_deadline = deadline - 1.0
    worker_exit: int | None = None
    worker_timed_out = False
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
    except (ChildProcessError, OSError, ValueError):
        # Ownership uncertainty is not permission to signal a numeric PGID.
        print("diagnostic_supervisor=worker_ownership_lost")
        return RESULT_CLEANUP_INCOMPLETE

    cleanup_deadline = deadline
    cleanup_ok = True
    try:
        table = _proc_table()
        owned, truncated = _owned_tree(table, leader_pid, os.getpid())
        if truncated:
            cleanup_ok = False
        descendants_present = any(stat.pid != leader_pid for stat in owned)

        if worker_timed_out:
            if not _signal_owned(
                table, leader_pid, leader_starttime, signal.SIGTERM
            ):
                cleanup_ok = False
            kill_at = min(
                deadline - REAP_RESERVE_SECONDS,
                _clock() + TERM_GRACE_SECONDS,
            )
            while _clock() < kill_at:
                if _cancel_signal is not None:
                    # A second external signal does not change ownership; it
                    # only advances escalation inside the original deadline.
                    break
                time.sleep(min(POLL_SECONDS, max(0.0, kill_at - _clock())))
            table = _proc_table()
            if not _signal_owned(
                table, leader_pid, leader_starttime, signal.SIGKILL
            ):
                cleanup_ok = False
        else:
            # Even after natural leader exit, kill only its still-pinned group
            # before reaping its PID. This closes any stray collector children.
            if not _signal_owned(
                table, leader_pid, leader_starttime, signal.SIGKILL
            ):
                cleanup_ok = False

        cleaned = _reap_to_deadline(
            leader_pid,
            leader_starttime,
            cleanup_deadline,
            group_signal_sent=True,
        )
        cleanup_ok = cleanup_ok and cleaned
    except (OSError, ValueError, ChildProcessError):
        cleanup_ok = False

    for signum, handler in old_handlers.items():
        signal.signal(signum, handler)

    if not cleanup_ok:
        print("diagnostic_supervisor=cleanup_incomplete")
        return RESULT_CLEANUP_INCOMPLETE
    if worker_timed_out:
        print("diagnostic_supervisor=timed_out_cleaned")
        return RESULT_TIMED_OUT_CLEANED
    if descendants_present:
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
