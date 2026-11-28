"""Run bounded probes for the VOS Windows negative-control timeout.

This is a diagnostic-only helper. It preserves raw process/pipe observations and
never treats a partial TAP stream as a successful negative control.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

FAULT = "field-flip-last"
FILTER = "/exhaustive/insert_oracle_lo"
PRODUCTION_TIMEOUT_SECS = 1200
DIAGNOSTIC_WRAPPER_TIMEOUT_SECS = 120
PROBE_TIMEOUT_SECS = 180
PROBE_KILL_RESERVE_SECS = 40
CONCURRENT_CHILD_DEADLINE_SECS = 120
CONCURRENT_CLEANUP_DEADLINE_SECS = 145
CONCURRENT_VERIFICATION_DEADLINE_SECS = 157
CONCURRENT_READER_DEADLINE_SECS = 165
CONCURRENT_REPORT_RESERVE_SECS = 15
READER_JOIN_SECS = 8
CAPTURE_LIMIT = 64 * 1024
SAMPLE_INTERVAL_SECS = 5
MARKER_LINE_LIMIT = 256


class StreamCapture:
    """Bounded head/tail capture with byte and EOF timings."""

    def __init__(self) -> None:
        self.total_bytes = 0
        self.first_byte_s: float | None = None
        self.last_byte_s: float | None = None
        self.eof_s: float | None = None
        self.first_tap_plan_s: float | None = None
        self.first_not_ok_s: float | None = None
        self.first_bail_out_s: float | None = None
        self.first_not_ok_number: int | None = None
        self.tap_plan_count = 0
        self.tap_plan_value: int | None = None
        self.tap_point_count = 0
        self.tap_not_ok_count = 0
        self.reader_error: str | None = None
        self.marker_line = b""
        self.marker_line_overflow = False
        self.head = bytearray()
        self.tail = bytearray()
        self.lock = threading.Lock()

    def read(self, stream: Any, started: float) -> None:
        try:
            while chunk := os.read(stream.fileno(), 8192):
                now = time.perf_counter() - started
                with self.lock:
                    if self.first_byte_s is None:
                        self.first_byte_s = now
                    self.last_byte_s = now
                    self.total_bytes += len(chunk)
                    self._consume_marker_bytes(chunk, now)
                    head_room = CAPTURE_LIMIT // 2 - len(self.head)
                    head_part = chunk[:head_room]
                    if head_part:
                        self.head.extend(head_part)
                    self.tail.extend(chunk[len(head_part) :])
                    if len(self.tail) > CAPTURE_LIMIT // 2:
                        del self.tail[: len(self.tail) - CAPTURE_LIMIT // 2]
        except OSError as exc:
            with self.lock:
                self.reader_error = f"{type(exc).__name__}: {exc}"
        finally:
            with self.lock:
                if self.eof_s is None and self.reader_error is None and self.marker_line:
                    self._record_complete_line(
                        self.marker_line,
                        self.marker_line_overflow,
                        time.perf_counter() - started,
                    )
                self.eof_s = time.perf_counter() - started

    def _consume_marker_bytes(self, chunk: bytes, now: float) -> None:
        pieces = (self.marker_line + chunk).split(b"\n")
        previous_overflow = self.marker_line_overflow
        for index, line in enumerate(pieces[:-1]):
            overflow = previous_overflow if index == 0 else False
            overflow = overflow or len(line) > MARKER_LINE_LIMIT
            self._record_complete_line(line[:MARKER_LINE_LIMIT], overflow, now)
        trailing = pieces[-1]
        self.marker_line_overflow = (
            (previous_overflow if len(pieces) == 1 else False)
            or len(trailing) > MARKER_LINE_LIMIT
        )
        self.marker_line = trailing[:MARKER_LINE_LIMIT]

    def _record_complete_line(self, line: bytes, overflow: bool, now: float) -> None:
        line = line.removesuffix(b"\r")
        plan = re.fullmatch(rb"1\.\.([0-9]+)", line) if not overflow else None
        if plan is not None:
            self.tap_plan_count += 1
            if self.tap_plan_count == 1:
                self.tap_plan_value = int(plan.group(1))
                self.first_tap_plan_s = now
        point = re.match(rb"^(not ok|ok)[ \t]+([0-9]+)(?:[ \t]|$)", line)
        if point is not None:
            self.tap_point_count += 1
            if point.group(1) == b"not ok":
                self.tap_not_ok_count += 1
                if self.first_not_ok_s is None:
                    self.first_not_ok_s = now
                    self.first_not_ok_number = int(point.group(2))
        if self.first_bail_out_s is None and line.startswith(b"Bail out!"):
            self.first_bail_out_s = now

    def output(self) -> str:
        with self.lock:
            if self.total_bytes <= CAPTURE_LIMIT:
                data = bytes(self.head + self.tail)
            else:
                data = (
                    bytes(self.head)
                    + f"\n[... {self.total_bytes - CAPTURE_LIMIT} bytes omitted ...]\n".encode()
                    + bytes(self.tail)
                )
        return data.decode("utf-8", errors="replace")


def process_snapshot(root_pid: int) -> dict[str, Any]:
    """Return descendant PID/name/parent rows, without command lines or env."""
    command = [
        "powershell.exe",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        (
            "$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process | "
            "Select-Object ProcessId,ParentProcessId,Name | ConvertTo-Json -Compress"
        ),
    ]
    try:
        result = subprocess.run(command, capture_output=True, timeout=3, check=False)
        if result.returncode != 0:
            return {"error": f"process query exit {result.returncode}"}
        parsed = json.loads(result.stdout.decode("utf-8", errors="replace"))
        rows = parsed if isinstance(parsed, list) else [parsed]
        by_pid = {int(row["ProcessId"]): row for row in rows if row}
        selected = {root_pid}
        changed = True
        while changed:
            changed = False
            for pid, row in by_pid.items():
                if int(row["ParentProcessId"]) in selected and pid not in selected:
                    selected.add(pid)
                    changed = True
        return {
            "processes": [
                {
                    "pid": pid,
                    "parent_pid": int(by_pid[pid]["ParentProcessId"]),
                    "name": str(by_pid[pid]["Name"]),
                }
                for pid in sorted(selected)
                if pid in by_pid
            ]
        }
    except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
        return {"error": f"{type(exc).__name__}: {exc}"}


def kill_tree(pid: int) -> str:
    try:
        result = subprocess.run(
            ["taskkill.exe", "/F", "/T", "/PID", str(pid)],
            capture_output=True,
            timeout=5,
            check=False,
        )
        return f"taskkill exit={result.returncode}"
    except (OSError, subprocess.TimeoutExpired) as exc:
        return f"taskkill error: {type(exc).__name__}: {exc}"


def terminate_and_verify_tree(
    proc: subprocess.Popen[bytes],
    verification_deadline: float,
) -> tuple[str, bool, list[dict[str, Any]]]:
    """Kill a process tree and confirm no captured descendants remain."""
    kill_result = kill_tree(proc.pid)
    try:
        proc.wait(timeout=max(0.1, min(7, verification_deadline - time.perf_counter())))
    except subprocess.TimeoutExpired:
        proc.kill()
        try:
            proc.wait(timeout=min(3, max(0.1, verification_deadline - time.perf_counter())))
        except subprocess.TimeoutExpired:
            pass

    snapshots: list[dict[str, Any]] = []
    snapshot = process_snapshot(proc.pid)
    snapshots.append(snapshot)
    if "processes" not in snapshot:
        return kill_result, False, snapshots
    survivors = snapshot["processes"]
    for row in survivors:
        if time.perf_counter() + 5 >= verification_deadline:
            return kill_result, False, snapshots
        kill_result += f"; descendant {row['pid']}: {kill_tree(int(row['pid']))}"
    if time.perf_counter() + 3 >= verification_deadline:
        return kill_result, False, snapshots
    final_snapshot = process_snapshot(proc.pid)
    snapshots.append(final_snapshot)
    return kill_result, final_snapshot.get("processes") == [], snapshots


def run_probe(
    name: str,
    argv: list[str],
    cwd: Path,
    env: dict[str, str],
    *,
    expected_exit_code: int = 1,
    wrapper_judge_expected: bool = False,
) -> dict[str, Any]:
    started = time.perf_counter()
    proc = subprocess.Popen(
        argv,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
    )
    stdout = StreamCapture()
    stderr = StreamCapture()
    assert proc.stdout is not None and proc.stderr is not None
    readers = [
        threading.Thread(target=stdout.read, args=(proc.stdout, started), daemon=True),
        threading.Thread(target=stderr.read, args=(proc.stderr, started), daemon=True),
    ]
    for reader in readers:
        reader.start()

    snapshots: list[dict[str, Any]] = []
    timed_out = False
    kill_result: str | None = None
    cleanup_verified = True
    next_sample = started
    # Reserve time for taskkill and pipe-reader shutdown, keeping total probe
    # wall time under the advertised 180-second cap.
    deadline = started + PROBE_TIMEOUT_SECS - PROBE_KILL_RESERVE_SECS
    while proc.poll() is None:
        now = time.perf_counter()
        if now >= next_sample and deadline - now > 5:
            snapshots.append({"at_s": round(now - started, 3), **process_snapshot(proc.pid)})
            next_sample = now + SAMPLE_INTERVAL_SECS
        remaining = deadline - now
        if remaining <= 0:
            timed_out = True
            kill_result, cleanup_verified, cleanup_snapshots = terminate_and_verify_tree(
                proc,
                started + PROBE_TIMEOUT_SECS - 10,
            )
            snapshots.extend(
                {"at_s": round(time.perf_counter() - started, 3), **item}
                for item in cleanup_snapshots
            )
            break
        try:
            proc.wait(timeout=min(1, remaining))
        except subprocess.TimeoutExpired:
            pass
    if not timed_out:
        kill_result, cleanup_verified, cleanup_snapshots = terminate_and_verify_tree(
            proc,
            started + PROBE_TIMEOUT_SECS - 10,
        )
        snapshots.extend(
            {"at_s": round(time.perf_counter() - started, 3), **item}
            for item in cleanup_snapshots
        )
    reader_deadline = started + PROBE_TIMEOUT_SECS - 5
    for reader in readers:
        reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
    if any(reader.is_alive() for reader in readers) and not timed_out:
        kill_result, cleanup_verified, cleanup_snapshots = terminate_and_verify_tree(
            proc,
            started + PROBE_TIMEOUT_SECS - 10,
        )
        snapshots.extend(
            {"at_s": round(time.perf_counter() - started, 3), **item}
            for item in cleanup_snapshots
        )
        for reader in readers:
            reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
    if any(reader.is_alive() for reader in readers):
        for stream in (proc.stdout, proc.stderr):
            if stream is not None:
                stream.close()
        for reader in readers:
            reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
    else:
        for stream in (proc.stdout, proc.stderr):
            if stream is not None:
                stream.close()
    ended = time.perf_counter()

    readers_done = all(not reader.is_alive() for reader in readers)
    elapsed_s = round(ended - started, 3)
    captured_stdout = stdout.output()
    if wrapper_judge_expected:
        # The wrapper buffers and judges child stdout, then publishes a concise
        # decision. Its stream does not contain the child's TAP or timing.
        oracle_valid = (
            "negctl: fault caught" in captured_stdout
            and "NEGATIVE CONTROL FAILED" not in captured_stdout
        )
    else:
        oracle_valid = (
            stdout.tap_plan_count == 1
            and stdout.tap_plan_value == 1
            and stdout.tap_point_count == 1
            and stdout.tap_not_ok_count == 1
            and stdout.first_not_ok_number == 1
            and stdout.first_bail_out_s is None
        )
    valid_fault_observation = (
        proc.returncode == expected_exit_code
        and not timed_out
        and cleanup_verified
        and oracle_valid
        and readers_done
        and stdout.eof_s is not None
        and stderr.eof_s is not None
        and stdout.reader_error is None
        and stderr.reader_error is None
    )
    return {
        "name": name,
        "argv": argv,
        "cwd": str(cwd),
        "root_pid": proc.pid,
        "root_process_name": Path(argv[0]).name,
        "root_parent_pid": os.getpid(),
        "cargo_target_dir_override": None,
        "elapsed_s": elapsed_s,
        "exit_code": proc.returncode,
        "expected_exit_code": expected_exit_code,
        "wrapper_judge_expected": wrapper_judge_expected,
        "outer_timeout": timed_out,
        "over_budget": elapsed_s > PROBE_TIMEOUT_SECS,
        "outer_kill": kill_result,
        "cleanup_verified": cleanup_verified,
        "stdout": {
            "bytes": stdout.total_bytes,
            "first_byte_s": stdout.first_byte_s,
            "last_byte_s": stdout.last_byte_s,
            "eof_s": stdout.eof_s,
            "first_tap_plan_s": stdout.first_tap_plan_s,
            "tap_plan_count": stdout.tap_plan_count,
            "tap_plan_value": stdout.tap_plan_value,
            "tap_point_count": stdout.tap_point_count,
            "first_not_ok_s": stdout.first_not_ok_s,
            "first_not_ok_number": stdout.first_not_ok_number,
            "tap_not_ok_count": stdout.tap_not_ok_count,
            "first_bail_out_s": stdout.first_bail_out_s,
            "reader_error": stdout.reader_error,
            "reader_finished": not any(t.is_alive() for t in readers[:1]),
            "captured": stdout.output(),
        },
        "stderr": {
            "bytes": stderr.total_bytes,
            "first_byte_s": stderr.first_byte_s,
            "last_byte_s": stderr.last_byte_s,
            "eof_s": stderr.eof_s,
            "reader_error": stderr.reader_error,
            "reader_finished": not any(t.is_alive() for t in readers[1:]),
            "captured": stderr.output(),
        },
        "readers_finished": readers_done,
        "valid_fault_observation": valid_fault_observation,
        "process_tree_samples": snapshots,
    }


def run_concurrent_probe(
    name: str,
    target_argv: list[str],
    sibling_argv: list[str],
    cwd: Path,
    target_env: dict[str, str],
    sibling_env: dict[str, str],
    target_directory: str,
    stream_observation_layer: str,
    wrapper_judge_expected: bool,
) -> dict[str, Any]:
    """Run one faulted Cargo process with one independent Cargo sibling."""
    phase_started = time.perf_counter()
    launch_specs = (
        ("target", target_argv, target_env),
        ("sibling", sibling_argv, sibling_env),
    )
    processes: dict[str, subprocess.Popen[bytes]] = {}
    launch_starts: dict[str, float] = {}
    launch_returns: dict[str, float] = {}
    launch_errors: dict[str, str] = {}
    captures: dict[str, tuple[StreamCapture, StreamCapture]] = {}
    readers: list[threading.Thread] = []
    reader_map: dict[str, tuple[threading.Thread, threading.Thread]] = {}

    # Launch sequentially and immediately. This avoids a detached Popen thread
    # that could create an untracked late process after the diagnostic returns.
    for label, argv, child_env in launch_specs:
        launch_starts[label] = time.perf_counter()
        try:
            proc = subprocess.Popen(
                argv,
                cwd=cwd,
                env=child_env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
            )
            launch_returns[label] = time.perf_counter()
            processes[label] = proc
            assert proc.stdout is not None and proc.stderr is not None
            out_capture, err_capture = StreamCapture(), StreamCapture()
            captures[label] = (out_capture, err_capture)
            started_readers = []
            for capture, stream in ((out_capture, proc.stdout), (err_capture, proc.stderr)):
                reader = threading.Thread(target=capture.read, args=(stream, phase_started), daemon=True)
                reader.start()
                readers.append(reader)
                started_readers.append(reader)
            reader_map[label] = (started_readers[0], started_readers[1])
        except (OSError, ValueError, RuntimeError) as exc:  # keep spawn failures visible in the report
            launch_errors[label] = f"{type(exc).__name__}: {exc}"
            break

    snapshots: list[dict[str, Any]] = []
    alive_samples: dict[str, list[float]] = {label: [] for label in processes}
    exit_times: dict[str, float] = {}
    timed_out_labels: list[str] = []
    next_sample = phase_started
    hard_child_deadline = phase_started + CONCURRENT_CHILD_DEADLINE_SECS

    while processes:
        now = time.perf_counter()
        running = False
        for label, proc in processes.items():
            observed_at = time.perf_counter()
            if proc.poll() is None:
                alive_samples[label].append(observed_at)
                running = True
            else:
                exit_times.setdefault(label, observed_at)
        if not running:
            break
        if now >= hard_child_deadline:
            timed_out_labels = [label for label, proc in processes.items() if proc.poll() is None]
            break
        if now >= next_sample and hard_child_deadline - now > 8:
            for label, proc in processes.items():
                snapshots.append(
                    {
                        "label": label,
                        "at_phase_s": round(time.perf_counter() - phase_started, 4),
                        **process_snapshot(proc.pid),
                    }
                )
            next_sample = time.perf_counter() + SAMPLE_INTERVAL_SECS
        time.sleep(0.2)

    kill_results: dict[str, str] = {}
    cleanup_verified: dict[str, bool] = {label: True for label in processes}
    if timed_out_labels:
        for label in timed_out_labels:
            kill_results[label] = kill_tree(processes[label].pid)
        cleanup_deadline = phase_started + CONCURRENT_CLEANUP_DEADLINE_SECS
        for label in timed_out_labels:
            proc = processes[label]
            try:
                proc.wait(timeout=max(0.1, cleanup_deadline - time.perf_counter()))
            except subprocess.TimeoutExpired:
                proc.kill()
                try:
                    proc.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    pass

    for label, proc in processes.items():
        if proc.poll() is not None:
            exit_times.setdefault(label, time.perf_counter())

    # Verify timed-out process trees, not only their root processes. Any query
    # error or surviving descendant makes the diagnostic incomplete.
    verification_deadline = phase_started + CONCURRENT_VERIFICATION_DEADLINE_SECS
    for label, proc in processes.items():
        if label in timed_out_labels and time.perf_counter() >= verification_deadline:
            cleanup_verified[label] = False
            continue
        snapshot = process_snapshot(proc.pid)
        snapshots.append(
            {
                "label": label,
                "at_phase_s": round(time.perf_counter() - phase_started, 4),
                "cleanup_verification": label in timed_out_labels,
                **snapshot,
            }
        )
        if "processes" not in snapshot:
            cleanup_verified[label] = False
            continue
        survivors = snapshot["processes"]
        if not survivors:
            cleanup_verified[label] = True
            continue
        cleanup_verified[label] = False
        for row in survivors:
            if time.perf_counter() + 5 >= verification_deadline:
                break
            survivor_pid = int(row["pid"])
            kill_results[f"{label}_survivor_{survivor_pid}"] = kill_tree(survivor_pid)
        if time.perf_counter() + 3 < verification_deadline:
            final_snapshot = process_snapshot(proc.pid)
            snapshots.append(
                {
                    "label": label,
                    "at_phase_s": round(time.perf_counter() - phase_started, 4),
                    "cleanup_verification": True,
                    **final_snapshot,
                }
            )
            cleanup_verified[label] = final_snapshot.get("processes") == []

    reader_deadline = phase_started + CONCURRENT_READER_DEADLINE_SECS
    for reader in readers:
        reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
    for label, proc in processes.items():
        if any(reader.is_alive() for reader in reader_map[label]):
            for stream in (proc.stdout, proc.stderr):
                if stream is not None:
                    stream.close()
            for reader in reader_map[label]:
                reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
        else:
            for stream in (proc.stdout, proc.stderr):
                if stream is not None:
                    stream.close()
    ended = time.perf_counter()

    def process_record(label: str) -> dict[str, Any]:
        proc = processes.get(label)
        if proc is None:
            return {"launch_error": launch_errors.get(label, "process was not launched")}
        stdout, stderr = captures[label]
        argv = target_argv if label == "target" else sibling_argv
        return {
            "argv": argv,
            "root_pid": proc.pid,
            "root_process_name": Path(argv[0]).name,
            "root_parent_pid": os.getpid(),
            "launch_started_phase_s": round(launch_starts[label] - phase_started, 4),
            "launch_returned_phase_s": round(launch_returns[label] - phase_started, 4),
            "cargo_target_dir_override": None,
            "cargo_resolved_target_directory": target_directory,
            "exit_code": proc.returncode,
            "outer_timeout": label in timed_out_labels,
            "outer_kill": kill_results.get(label),
            "cleanup_verified": cleanup_verified.get(label, False),
            "stdout": {
                "bytes": stdout.total_bytes,
                "first_byte_s": stdout.first_byte_s,
                "last_byte_s": stdout.last_byte_s,
                "eof_s": stdout.eof_s,
                "first_tap_plan_s": stdout.first_tap_plan_s,
                "tap_plan_count": stdout.tap_plan_count,
                "tap_plan_value": stdout.tap_plan_value,
                "tap_point_count": stdout.tap_point_count,
                "first_not_ok_s": stdout.first_not_ok_s,
                "first_not_ok_number": stdout.first_not_ok_number,
                "tap_not_ok_count": stdout.tap_not_ok_count,
                "first_bail_out_s": stdout.first_bail_out_s,
                "reader_error": stdout.reader_error,
                "reader_finished": not reader_map[label][0].is_alive(),
                "captured": stdout.output(),
            },
            "stderr": {
                "bytes": stderr.total_bytes,
                "first_byte_s": stderr.first_byte_s,
                "last_byte_s": stderr.last_byte_s,
                "eof_s": stderr.eof_s,
                "reader_finished": not reader_map[label][1].is_alive(),
                "reader_error": stderr.reader_error,
                "captured": stderr.output(),
            },
            "readers_finished": not reader_map[label][0].is_alive() and not reader_map[label][1].is_alive(),
        }

    records = {label: process_record(label) for label in ("target", "sibling")}
    alive_intervals = {
        label: (samples[0], samples[-1]) if samples else None
        for label, samples in alive_samples.items()
    }
    provable_overlap_s = 0.0
    if all(alive_intervals.get(label) for label in ("target", "sibling")):
        target_interval = alive_intervals["target"]
        sibling_interval = alive_intervals["sibling"]
        assert target_interval is not None and sibling_interval is not None
        provable_overlap_s = max(
            0.0,
            min(target_interval[1], sibling_interval[1]) - max(target_interval[0], sibling_interval[0]),
        )

    elapsed_s = round(ended - phase_started, 3)
    target = records["target"]
    target_out = target.get("stdout", {}).get("captured", "")
    target_events = target.get("stdout", {})
    target_first_tap_phase = target_events.get("first_tap_plan_s")
    target_first_not_ok_phase = target_events.get("first_not_ok_s")
    target_last_byte_phase = target_events.get("last_byte_s")
    target_exit_phase = exit_times.get("target")
    target_events.update(
        {
            "first_tap_plan_phase_s": target_first_tap_phase,
            "first_not_ok_phase_s": target_first_not_ok_phase,
            "last_byte_phase_s": target_last_byte_phase,
            "process_exit_phase_s": target_exit_phase - phase_started if target_exit_phase is not None else None,
        }
    )
    sibling_interval = alive_intervals.get("sibling")

    def sibling_alive_at(target_event_phase: float | None) -> bool:
        return (
            target_event_phase is not None
            and sibling_interval is not None
            and sibling_interval[0] <= phase_started + target_event_phase <= sibling_interval[1]
        )

    sibling_covers_tap = sibling_alive_at(target_first_tap_phase)
    sibling_covers_not_ok = sibling_alive_at(target_first_not_ok_phase)
    sibling_covers_last_byte = sibling_alive_at(target_last_byte_phase)
    target_exit_interval = None
    target_samples = alive_samples.get("target", [])
    if target_exit_phase is not None:
        target_exit_interval = (
            target_samples[-1] if target_samples else launch_returns.get("target"),
            target_exit_phase,
        )
    sibling_covers_target_exit = (
        target_exit_interval is not None
        and sibling_interval is not None
        and target_exit_interval[0] is not None
        and sibling_interval[0] <= target_exit_interval[0]
        and sibling_interval[1] >= target_exit_interval[1]
    )
    sibling_exit_phase = exit_times.get("sibling")
    records["sibling"].update(
        {
            "exit_phase_s": sibling_exit_phase - phase_started if sibling_exit_phase is not None else None,
            "alive_interval_phase_s": [round(point - phase_started, 4) for point in sibling_interval]
            if sibling_interval
            else None,
        }
    )

    if wrapper_judge_expected:
        target_oracle_valid = (
            "negctl: fault caught" in target_out
            and "NEGATIVE CONTROL FAILED" not in target_out
        )
        expected_target_exit = 0
    else:
        # The retained Windows probe observed Cargo return the failing harness
        # process's status (1) for this command and fault selector.
        target_oracle_valid = (
            target_events.get("tap_plan_count") == 1
            and target_events.get("tap_plan_value") == 1
            and target_events.get("tap_point_count") == 1
            and target_events.get("tap_not_ok_count") == 1
            and target_events.get("first_not_ok_number") == 1
            and target_events.get("first_bail_out_s") is None
        )
        expected_target_exit = 1
    target["valid_negative_control"] = (
        target.get("exit_code") == expected_target_exit
        and target_oracle_valid
        and not target.get("outer_timeout", False)
        and target.get("cleanup_verified", False)
        and target.get("readers_finished", False)
        and target_events.get("eof_s") is not None
        and target.get("stderr", {}).get("eof_s") is not None
        and target_events.get("reader_error") is None
        and target.get("stderr", {}).get("reader_error") is None
    )
    records["sibling"]["valid_control"] = (
        records["sibling"].get("exit_code") == 0
        and records["sibling"].get("readers_finished", False)
        and records["sibling"].get("stdout", {}).get("eof_s") is not None
        and records["sibling"].get("stderr", {}).get("eof_s") is not None
        and records["sibling"].get("stdout", {}).get("reader_error") is None
        and records["sibling"].get("stderr", {}).get("reader_error") is None
    )

    cleanup_complete = all(cleanup_verified.values()) and all(
        record.get("cleanup_verified", False) for record in records.values() if "root_pid" in record
    )
    if elapsed_s > PROBE_TIMEOUT_SECS:
        status = "incomplete-over-budget"
    elif not cleanup_complete or not all(record.get("readers_finished", False) for record in records.values() if "root_pid" in record):
        status = "inconclusive-cleanup-or-reader-unverified"
    elif timed_out_labels:
        status = "inconclusive-timeout"
    elif launch_errors or provable_overlap_s <= 0:
        status = "inconclusive-no-proven-overlap"
    elif wrapper_judge_expected:
        status = (
            "wrapper-control-completed-child-timing-hidden"
            if records["sibling"]["valid_control"] and target["valid_negative_control"]
            else "inconclusive-wrapper-control"
        )
    elif not sibling_covers_tap or not sibling_covers_not_ok or not sibling_covers_last_byte:
        status = "inconclusive-no-sustained-overlap-through-target-output"
    elif not records["sibling"]["valid_control"] or not target["valid_negative_control"]:
        status = "inconclusive-invalid-control"
    else:
        status = "not-reproduced-under-one-sibling"

    return {
        "name": name,
        "phase": "two-cargo-workers-shared-default-target",
        "target_stream_observation_layer": stream_observation_layer,
        "wrapper_judge_expected": wrapper_judge_expected,
        "cargo_target_directory": target_directory,
        "cargo_target_dir_override": None,
        "launch_model": "sequential Popen calls; no asynchronous launcher threads",
        "launch_skew_s": round(launch_returns.get("sibling", phase_started) - launch_returns.get("target", phase_started), 4),
        "sibling_package": "rust/mathml",
        "sibling_command": sibling_argv,
        "sibling_plan_provenance": {
            "evidence_source": "source-audited retained original failed-run build plan",
            "ci_run_id": 46047723015,
            "ci_job_id": 140677847764,
            "plan_artifact_sha256": "f01438fc714ddb42f612206e9d2ef9665c063ee7e8ebab0fc9b0b8d4dd7aa111",
            "target_package": "rust/vos-sched-any",
            "sibling_package": "rust/mathml",
            "dependency_level": 1,
            "independence_basis": "same topological build level; no dependency path between these nodes in the retained plan",
            "build_command": sibling_argv,
            "original_build_elapsed_s": 19.9,
        },
        "launch_errors": launch_errors,
        "elapsed_s": elapsed_s,
        "over_budget": elapsed_s > PROBE_TIMEOUT_SECS,
        "outer_probe_timeout_s": PROBE_TIMEOUT_SECS,
        "report_reserve_s": CONCURRENT_REPORT_RESERVE_SECS,
        "provable_overlap_s": round(provable_overlap_s, 4),
        "alive_intervals_phase_s": {
            label: [round(point - phase_started, 4) for point in interval] if interval else None
            for label, interval in alive_intervals.items()
        },
        "exit_observation_intervals_phase_s": {
            label: [
                round(
                    (alive_samples[label][-1] if alive_samples[label] else launch_returns.get(label, phase_started))
                    - phase_started,
                    4,
                ),
                round(exit_times[label] - phase_started, 4),
            ]
            if label in exit_times
            else None
            for label in processes
        },
        "sibling_covers_target_first_tap": sibling_covers_tap,
        "sibling_covers_target_first_not_ok": sibling_covers_not_ok,
        "sibling_covers_target_last_byte": sibling_covers_last_byte,
        "sibling_covers_target_exit": sibling_covers_target_exit,
        "target_exit_observation_interval_phase_s": [
            round(point - phase_started, 4) for point in target_exit_interval
        ] if target_exit_interval else None,
        "timed_out_labels": timed_out_labels,
        "process_tree_samples": snapshots,
        "target": records["target"],
        "sibling": records["sibling"],
        "readers_finished": all(not reader.is_alive() for reader in readers),
        "cleanup_verified": cleanup_complete,
        "interpretation": status,
    }

def build_test_binary(repo_root: Path) -> Path:
    cargo_root = repo_root / "code" / "packages" / "rust"
    command = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "vos-sched-any",
        "--features",
        "ghost",
        "--test",
        "warrant_evidence",
        "--no-run",
        "--message-format=json-render-diagnostics",
    ]
    build_env = os.environ.copy()
    build_env.pop("CARGO_TARGET_DIR", None)
    build_env["CARGO_TERM_COLOR"] = "never"
    proc = subprocess.Popen(
        command,
        cwd=cargo_root,
        env=build_env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    executable: Path | None = None
    assert proc.stdout is not None
    for line in proc.stdout:
        print(line, end="", flush=True)
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if item.get("reason") != "compiler-artifact":
            continue
        target = item.get("target", {})
        if target.get("name") == "warrant_evidence" and "test" in target.get("kind", []):
            candidate = item.get("executable")
            if candidate:
                executable = Path(candidate)
    code = proc.wait(timeout=1500)
    if code != 0:
        raise RuntimeError(f"cargo --no-run failed with exit {code}")
    if executable is None or not executable.is_file():
        raise RuntimeError("cargo succeeded but did not report the warrant_evidence test executable")
    return executable


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    args = parser.parse_args()

    if os.name != "nt":
        parser.error("this diagnostic must run on Windows")
    repo_root = args.repo_root.resolve()
    artifact_dir = args.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    manifest: dict[str, Any] = {
        "candidate_sha": args.candidate_sha,
        "platform": sys.platform,
        "fault": FAULT,
        "filter": FILTER,
        "production_wrapper_default_timeout_s": PRODUCTION_TIMEOUT_SECS,
        "diagnostic_wrapper_timeout_s": DIAGNOSTIC_WRAPPER_TIMEOUT_SECS,
        "outer_probe_timeout_s": PROBE_TIMEOUT_SECS,
        "timeout_scope": "180s for each process probe and for the paired sibling phase, including cleanup and report-write reserve",
        "stream_observation_note": (
            "warrant_negctl buffers child output, so wrapper-phase TAP times are wrapper publication times; "
            "the paired direct-Cargo phase records the forwarded process stream without the wrapper"
        ),
        "build": {"started": True},
        "probes": [],
    }
    try:
        executable = build_test_binary(repo_root)
        rust_root = repo_root / "code" / "packages" / "rust"
        metadata = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=rust_root,
            env={key: value for key, value in os.environ.items() if key != "CARGO_TARGET_DIR"},
            capture_output=True,
            timeout=15,
            check=True,
            text=True,
            encoding="utf-8",
        )
        default_target_path = json.loads(metadata.stdout)["target_directory"]
        manifest["build"] = {
            "exit_code": 0,
            "executable": str(executable),
            "cargo_target_directory": default_target_path,
            "cargo_target_dir_override": None,
        }
        cwd = rust_root / "vos-sched-any"
        cargo_cmd = [
            "cargo",
            "test",
            "-q",
            "-p",
            "vos-sched-any",
            "--features",
            "ghost",
            "--test",
            "warrant_evidence",
        ]
        wrapper = repo_root / "code" / "scripts" / "warrant_negctl.py"
        base_env = os.environ.copy()
        base_env.update({"WARRANT_EVIDENCE_FAULT": FAULT, "WARRANT_EVIDENCE_FILTER": FILTER})
        base_env.pop("CARGO_TARGET_DIR", None)
        wrapped_env = base_env.copy()
        wrapped_env["WARRANT_NEGCTL_TIMEOUT_SECS"] = str(DIAGNOSTIC_WRAPPER_TIMEOUT_SECS)
        probes = [
            ("direct-prebuilt", [str(executable)], base_env),
            ("direct-cargo", cargo_cmd, base_env),
            ("negctl-prebuilt", [sys.executable, str(wrapper), "--", str(executable)], wrapped_env),
            ("negctl-cargo", [sys.executable, str(wrapper), "--", *cargo_cmd], wrapped_env),
        ]
        for name, argv, env in probes:
            print(f"\n=== {name} ===", flush=True)
            wrapper_call = name.startswith("negctl-")
            result = run_probe(
                name,
                argv,
                cwd,
                env,
                expected_exit_code=0 if wrapper_call else 1,
                wrapper_judge_expected=wrapper_call,
            )
            manifest["probes"].append(result)
            state = "outer-timeout" if result["outer_timeout"] else f"exit-{result['exit_code']}"
            print(f"{name}: {state}, {result['elapsed_s']:.3f}s", flush=True)

        # Reproduce the original build-tool condition: two independent Cargo
        # workers use the workspace's default target directory concurrently.
        # The earlier four probes and both comparison phases use the default
        # workspace target, matching build-tool Cargo workers.
        default_env = os.environ.copy()
        default_env.pop("CARGO_TARGET_DIR", None)
        default_env.update({"WARRANT_EVIDENCE_FAULT": FAULT, "WARRANT_EVIDENCE_FILTER": FILTER})
        default_env["WARRANT_NEGCTL_TIMEOUT_SECS"] = str(DIAGNOSTIC_WRAPPER_TIMEOUT_SECS)
        sibling_env = os.environ.copy()
        sibling_env.pop("CARGO_TARGET_DIR", None)
        for key in ("WARRANT_EVIDENCE_FAULT", "WARRANT_EVIDENCE_FILTER", "WARRANT_NEGCTL_TIMEOUT_SECS"):
            sibling_env.pop(key, None)
        default_target = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=rust_root,
            env=default_env,
            capture_output=True,
            timeout=15,
            check=True,
            text=True,
            encoding="utf-8",
        )
        target_path = json.loads(default_target.stdout)["target_directory"]
        default_cargo = [
            "cargo", "test", "-q", "-p", "vos-sched-any", "--features", "ghost",
            "--test", "warrant_evidence",
        ]
        default_wrapped = [sys.executable, str(wrapper), "--", *default_cargo]
        print("\n=== shared-target-baseline ===", flush=True)
        baseline = run_probe(
            "shared-target-baseline",
            default_wrapped,
            cwd,
            default_env,
            expected_exit_code=0,
            wrapper_judge_expected=True,
        )
        baseline["phase"] = "single-worker-default-target"
        baseline["cargo_target_directory"] = target_path
        baseline["cargo_target_dir_override"] = None
        baseline["valid_negative_control"] = baseline["valid_fault_observation"] and (
            "negctl: fault caught" in baseline["stdout"]["captured"]
        )
        manifest["probes"].append(baseline)
        print(f"shared-target-baseline: exit-{baseline['exit_code']}, {baseline['elapsed_s']:.3f}s", flush=True)

        sibling_cargo = ["cargo", "test", "--locked", "-p", "mathml", "--", "--nocapture"]
        print("\n=== shared-target-sibling-concurrent ===", flush=True)
        concurrent = run_concurrent_probe(
            "shared-target-sibling-concurrent",
            default_wrapped,
            sibling_cargo,
            cwd,
            default_env,
            sibling_env,
            target_path,
            "wrapper-publication-after-cargo-child-exit-or-timeout",
            True,
        )
        manifest["probes"].append(concurrent)
        print(
            f"shared-target-sibling-concurrent: target-exit-{concurrent['target']['exit_code']}, "
            f"sibling-exit-{concurrent['sibling']['exit_code']}, {concurrent['elapsed_s']:.3f}s",
            flush=True,
        )
        direct_env = default_env.copy()
        direct_env.pop("WARRANT_NEGCTL_TIMEOUT_SECS", None)
        direct_concurrent = run_concurrent_probe(
            "shared-target-direct-cargo-sibling-concurrent",
            default_cargo,
            sibling_cargo,
            cwd,
            direct_env,
            sibling_env,
            target_path,
            "direct-cargo-process-stream",
            False,
        )
        manifest["probes"].append(direct_concurrent)
        print(
            f"shared-target-direct-cargo-sibling-concurrent: target-exit-"
            f"{direct_concurrent['target'].get('exit_code')}, sibling-exit-"
            f"{direct_concurrent['sibling'].get('exit_code')}, "
            f"{direct_concurrent['elapsed_s']:.3f}s",
            flush=True,
        )
    except Exception as exc:  # noqa: BLE001 - keep the report when any setup step fails
        manifest["setup_error"] = f"{type(exc).__name__}: {exc}"
        print(manifest["setup_error"], file=sys.stderr, flush=True)
    report_path = artifact_dir / "vos-negctl-windows-diagnostic.json"
    report_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as summary:
            summary.write("## VOS Windows negative-control diagnostic\n\n")
            summary.write(f"- Candidate: `{args.candidate_sha}`\n")
            summary.write(f"- Exact injected fault/filter: `{FAULT}` / `{FILTER}`\n")
            summary.write(f"- Production wrapper default remains {PRODUCTION_TIMEOUT_SECS}s; diagnostic wrapper is bounded to {DIAGNOSTIC_WRAPPER_TIMEOUT_SECS}s.\n")
            summary.write("- A timeout, missing EOF, or incomplete TAP is an observation, never a successful control.\n\n")
            for probe in manifest["probes"]:
                if "target" in probe:
                    summary.write(
                        f"- `{probe['name']}` ({probe['target_stream_observation_layer']}): "
                        f"{probe['interpretation']}; provable overlap "
                        f"`{probe['provable_overlap_s']}s`; target exit `{probe['target'].get('exit_code')}`, "
                        f"sibling exit `{probe['sibling'].get('exit_code')}`; target first TAP/`not ok`/last byte/exit "
                        f"`{probe['target'].get('stdout', {}).get('first_tap_plan_phase_s')}`/"
                        f"`{probe['target'].get('stdout', {}).get('first_not_ok_phase_s')}`/"
                        f"`{probe['target'].get('stdout', {}).get('last_byte_phase_s')}`/"
                        f"`{probe['target'].get('stdout', {}).get('process_exit_phase_s')}`; sibling covers TAP/"
                        f"`not ok`/last byte/exit `"
                        f"{probe['sibling_covers_target_first_tap']}`/"
                        f"`{probe['sibling_covers_target_first_not_ok']}`/"
                        f"`{probe['sibling_covers_target_last_byte']}`/"
                        f"`{probe['sibling_covers_target_exit']}`.\n"
                    )
                else:
                    summary.write(
                        f"- `{probe['name']}`: exit `{probe['exit_code']}`, "
                        f"outer-timeout `{probe['outer_timeout']}`, over-budget `{probe['over_budget']}`, "
                        f"{probe['elapsed_s']}s, valid fault observation `"
                        f"{probe['valid_fault_observation']}`, stdout/stderr EOF `"
                        f"{probe['stdout']['eof_s']}` / `{probe['stderr']['eof_s']}`, "
                        f"first `not ok` `{probe['stdout']['first_not_ok_s']}`.\n"
                    )
            if "setup_error" in manifest:
                summary.write(f"\nSetup error: `{manifest['setup_error']}`\n")
    print(f"\nDiagnostic report: {report_path}", flush=True)
    complete = len(manifest["probes"]) == 7 and all(
        probe["readers_finished"]
        and not probe["over_budget"]
        and probe.get("cleanup_verified", False)
        and any("processes" in sample for sample in probe["process_tree_samples"])
        for probe in manifest["probes"]
    )
    complete = complete and all(
        probe.get("valid_fault_observation", False)
        for probe in manifest["probes"]
        if "target" not in probe
    )
    paired = [probe for probe in manifest["probes"] if "target" in probe]
    complete = complete and len(paired) == 2
    if len(paired) == 2:
        complete = complete and paired[0].get("interpretation") == "wrapper-control-completed-child-timing-hidden"
        complete = complete and paired[1].get("interpretation") == "not-reproduced-under-one-sibling"
    return 0 if complete else 2


if __name__ == "__main__":
    raise SystemExit(main())
