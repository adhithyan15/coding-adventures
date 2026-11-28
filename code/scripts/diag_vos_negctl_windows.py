#!/usr/bin/env python3
"""Run bounded probes for the VOS Windows negative-control timeout.

This is a diagnostic-only helper. It preserves raw process/pipe observations and
never treats a partial TAP stream as a successful negative control.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time
from typing import Any


FAULT = "field-flip-last"
FILTER = "/exhaustive/insert_oracle_lo"
PRODUCTION_TIMEOUT_SECS = 1200
DIAGNOSTIC_WRAPPER_TIMEOUT_SECS = 120
PROBE_TIMEOUT_SECS = 180
PROBE_KILL_RESERVE_SECS = 40
CONCURRENT_CHILD_DEADLINE_SECS = 135
CONCURRENT_CLEANUP_DEADLINE_SECS = 150
CONCURRENT_READER_DEADLINE_SECS = 165
CONCURRENT_REPORT_RESERVE_SECS = 15
READER_JOIN_SECS = 8
CAPTURE_LIMIT = 64 * 1024
SAMPLE_INTERVAL_SECS = 5


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
        self.marker_tail = b""
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
                    searchable = self.marker_tail + chunk
                    if self.first_tap_plan_s is None and b"1.." in searchable:
                        self.first_tap_plan_s = now
                    if self.first_not_ok_s is None and b"not ok" in searchable:
                        self.first_not_ok_s = now
                    if self.first_bail_out_s is None and b"Bail out!" in searchable:
                        self.first_bail_out_s = now
                    self.marker_tail = searchable[-16:]
                    head_room = CAPTURE_LIMIT // 2 - len(self.head)
                    head_part = chunk[:head_room]
                    if head_part:
                        self.head.extend(head_part)
                    self.tail.extend(chunk[len(head_part) :])
                    if len(self.tail) > CAPTURE_LIMIT // 2:
                        del self.tail[: len(self.tail) - CAPTURE_LIMIT // 2]
        finally:
            with self.lock:
                self.eof_s = time.perf_counter() - started

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
        "$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process | "
        "Select-Object ProcessId,ParentProcessId,Name | ConvertTo-Json -Compress",
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


def run_probe(name: str, argv: list[str], cwd: Path, env: dict[str, str]) -> dict[str, Any]:
    started = time.perf_counter()
    proc = subprocess.Popen(
        argv,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
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
            kill_result = kill_tree(proc.pid)
            try:
                proc.wait(timeout=7)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=3)
            break
        try:
            proc.wait(timeout=min(1, remaining))
        except subprocess.TimeoutExpired:
            pass
    snapshot_at = time.perf_counter()
    snapshots.append({"at_s": round(snapshot_at - started, 3), **process_snapshot(proc.pid)})
    reader_deadline = time.perf_counter() + READER_JOIN_SECS
    for reader in readers:
        reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
    ended = time.perf_counter()

    readers_done = all(not reader.is_alive() for reader in readers)
    elapsed_s = round(ended - started, 3)
    captured_stdout = stdout.output()
    valid_fault_observation = (
        proc.returncode == 1
        and not timed_out
        and not captured_stdout.startswith("Bail out!")
        and "1..1" in captured_stdout
        and "not ok" in captured_stdout
        and readers_done
        and stdout.eof_s is not None
        and stderr.eof_s is not None
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
        "outer_timeout": timed_out,
        "over_budget": elapsed_s > PROBE_TIMEOUT_SECS,
        "outer_kill": kill_result,
        "stdout": {
            "bytes": stdout.total_bytes,
            "first_byte_s": stdout.first_byte_s,
            "last_byte_s": stdout.last_byte_s,
            "eof_s": stdout.eof_s,
            "first_tap_plan_s": stdout.first_tap_plan_s,
            "first_not_ok_s": stdout.first_not_ok_s,
            "first_bail_out_s": stdout.first_bail_out_s,
            "reader_finished": not any(t.is_alive() for t in readers[:1]),
            "captured": stdout.output(),
        },
        "stderr": {
            "bytes": stderr.total_bytes,
            "first_byte_s": stderr.first_byte_s,
            "last_byte_s": stderr.last_byte_s,
            "eof_s": stderr.eof_s,
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
    env: dict[str, str],
    target_directory: str,
) -> dict[str, Any]:
    """Run the exact faulted Cargo command alongside one independent Cargo test."""
    phase_started = time.perf_counter()
    gate = threading.Barrier(3)
    launched: dict[str, tuple[subprocess.Popen[bytes], float]] = {}
    launch_errors: dict[str, str] = {}

    def launch(label: str, argv: list[str]) -> None:
        try:
            gate.wait(timeout=5)
            started = time.perf_counter()
            process = subprocess.Popen(
                argv,
                cwd=cwd,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
            )
            launched[label] = (process, started)
        except Exception as exc:  # retain launch failures in the artifact
            launch_errors[label] = f"{type(exc).__name__}: {exc}"

    launch_threads = [
        threading.Thread(target=launch, args=("target", target_argv), daemon=True),
        threading.Thread(target=launch, args=("sibling", sibling_argv), daemon=True),
    ]
    for thread in launch_threads:
        thread.start()
    gate.wait(timeout=5)
    for thread in launch_threads:
        thread.join(timeout=5)

    processes: dict[str, subprocess.Popen[bytes]] = {}
    starts: dict[str, float] = {}
    captures: dict[str, tuple[StreamCapture, StreamCapture]] = {}
    readers: list[threading.Thread] = []
    reader_map: dict[str, tuple[threading.Thread, threading.Thread]] = {}
    for label, (process, started) in launched.items():
        processes[label] = process
        starts[label] = started
        out_capture, err_capture = StreamCapture(), StreamCapture()
        captures[label] = (out_capture, err_capture)
        assert process.stdout is not None and process.stderr is not None
        for capture, stream in ((out_capture, process.stdout), (err_capture, process.stderr)):
            reader = threading.Thread(target=capture.read, args=(stream, started), daemon=True)
            reader.start()
            readers.append(reader)
        reader_map[label] = (readers[-2], readers[-1])

    snapshots: list[dict[str, Any]] = []
    hard_child_deadline = phase_started + CONCURRENT_CHILD_DEADLINE_SECS
    next_sample = phase_started
    timed_out_labels: list[str] = []
    exit_times: dict[str, float] = {}
    while processes and any(proc.poll() is None for proc in processes.values()):
        now = time.perf_counter()
        for label, proc in processes.items():
            if proc.poll() is not None:
                exit_times.setdefault(label, time.perf_counter())
        if now >= next_sample and hard_child_deadline - now > 8:
            for label, process in processes.items():
                snapshots.append(
                    {
                        "label": label,
                        "at_phase_s": round(now - phase_started, 3),
                        **process_snapshot(process.pid),
                    }
                )
            next_sample = now + SAMPLE_INTERVAL_SECS
        if now >= hard_child_deadline:
            timed_out_labels = [label for label, proc in processes.items() if proc.poll() is None]
            break
        time.sleep(0.2)

    kill_results: dict[str, str] = {}
    if timed_out_labels:
        kill_threads = {
            label: threading.Thread(
                target=lambda key=label: kill_results.__setitem__(key, kill_tree(processes[key].pid)),
                daemon=True,
            )
            for label in timed_out_labels
        }
        for thread in kill_threads.values():
            thread.start()
        kill_join_deadline = phase_started + CONCURRENT_CHILD_DEADLINE_SECS + 8
        for label in timed_out_labels:
            processes[label].poll()
        for thread in kill_threads.values():
            thread.join(timeout=max(0, kill_join_deadline - time.perf_counter()))
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

    for label, process in processes.items():
        if process.poll() is not None:
            exit_times.setdefault(label, time.perf_counter())

    # One last bounded process snapshot per root, then one shared reader-join
    # budget. This leaves time inside the 180s phase budget for JSON writing.
    for label, process in processes.items():
        snapshots.append(
            {
                "label": label,
                "at_phase_s": round(time.perf_counter() - phase_started, 3),
                **process_snapshot(process.pid),
            }
        )
    reader_deadline = phase_started + CONCURRENT_READER_DEADLINE_SECS
    for reader in readers:
        reader.join(timeout=max(0, reader_deadline - time.perf_counter()))
    ended = time.perf_counter()

    def process_record(label: str) -> dict[str, Any]:
        proc = processes.get(label)
        if proc is None:
            return {"launch_error": launch_errors.get(label, "process was not launched")}
        stdout, stderr = captures[label]
        started = starts[label]
        return {
            "argv": target_argv if label == "target" else sibling_argv,
            "root_pid": proc.pid,
            "root_process_name": Path((target_argv if label == "target" else sibling_argv)[0]).name,
            "root_parent_pid": os.getpid(),
            "started_at_phase_s": round(started - phase_started, 4),
            "cargo_target_dir_override": None,
            "cargo_resolved_target_directory": target_directory,
            "exit_code": proc.returncode,
            "outer_timeout": label in timed_out_labels,
            "outer_kill": kill_results.get(label),
            "stdout": {
                "bytes": stdout.total_bytes,
                "first_byte_s": stdout.first_byte_s,
                "last_byte_s": stdout.last_byte_s,
                "eof_s": stdout.eof_s,
                "first_tap_plan_s": stdout.first_tap_plan_s,
                "first_not_ok_s": stdout.first_not_ok_s,
                "first_bail_out_s": stdout.first_bail_out_s,
                "reader_finished": not reader_map[label][0].is_alive(),
                "captured": stdout.output(),
            },
            "stderr": {
                "bytes": stderr.total_bytes,
                "first_byte_s": stderr.first_byte_s,
                "last_byte_s": stderr.last_byte_s,
                "eof_s": stderr.eof_s,
                "reader_finished": not reader_map[label][1].is_alive(),
                "captured": stderr.output(),
            },
            "readers_finished": not reader_map[label][0].is_alive() and not reader_map[label][1].is_alive(),
        }

    records = {label: process_record(label) for label in ("target", "sibling")}
    intervals = [(starts[label], exit_times.get(label, ended)) for label in processes]
    overlap_s = 0.0
    if len(intervals) == 2:
        overlap_s = max(0.0, min(intervals[0][1], intervals[1][1]) - max(intervals[0][0], intervals[1][0]))
    elapsed_s = round(ended - phase_started, 3)
    target = records["target"]
    target_out = target.get("stdout", {}).get("captured", "")
    target_start = starts.get("target", phase_started)
    sibling_start = starts.get("sibling", phase_started)
    target_exit_phase = exit_times.get("target")
    sibling_exit_phase = exit_times.get("sibling")
    target_events = target.get("stdout", {})
    target_first_tap_phase = (
        target_start - phase_started + target_events["first_tap_plan_s"]
        if target_events.get("first_tap_plan_s") is not None
        else None
    )
    target_first_not_ok_phase = (
        target_start - phase_started + target_events["first_not_ok_s"]
        if target_events.get("first_not_ok_s") is not None
        else None
    )
    target_last_byte_phase = (
        target_start - phase_started + target_events["last_byte_s"]
        if target_events.get("last_byte_s") is not None
        else None
    )
    target_events.update(
        {
            "first_tap_plan_phase_s": target_first_tap_phase,
            "first_not_ok_phase_s": target_first_not_ok_phase,
            "last_byte_phase_s": target_last_byte_phase,
            "process_exit_phase_s": target_exit_phase - phase_started if target_exit_phase is not None else None,
        }
    )
    records["sibling"].update(
        {
            "start_phase_s": sibling_start - phase_started,
            "exit_phase_s": sibling_exit_phase - phase_started if sibling_exit_phase is not None else None,
        }
    )
    sibling_covers_tap = (
        target_first_tap_phase is not None
        and sibling_start <= phase_started + target_first_tap_phase
        and sibling_exit_phase is not None
        and sibling_exit_phase >= phase_started + target_first_tap_phase
    )
    sibling_covers_not_ok = (
        target_first_not_ok_phase is not None
        and sibling_start <= phase_started + target_first_not_ok_phase
        and sibling_exit_phase is not None
        and sibling_exit_phase >= phase_started + target_first_not_ok_phase
    )
    sibling_covers_last_byte = (
        target_last_byte_phase is not None
        and sibling_start <= phase_started + target_last_byte_phase
        and sibling_exit_phase is not None
        and sibling_exit_phase >= phase_started + target_last_byte_phase
    )
    sibling_covers_target_exit = (
        target_exit_phase is not None
        and sibling_start <= target_exit_phase
        and sibling_exit_phase is not None
        and sibling_exit_phase >= target_exit_phase
    )
    target["valid_negative_control"] = (
        target.get("exit_code") == 1
        and target.get("stdout", {}).get("first_tap_plan_s") is not None
        and target.get("stdout", {}).get("first_not_ok_s") is not None
        and "Bail out!" not in target_out
        and "negctl: fault caught" in target_out
        and not target.get("outer_timeout", False)
        and target.get("readers_finished", False)
        and target.get("stdout", {}).get("eof_s") is not None
        and target.get("stderr", {}).get("eof_s") is not None
    )
    records["sibling"]["valid_control"] = (
        records["sibling"].get("exit_code") == 0
        and records["sibling"].get("readers_finished", False)
        and records["sibling"].get("stdout", {}).get("eof_s") is not None
        and records["sibling"].get("stderr", {}).get("eof_s") is not None
    )
    if elapsed_s > PROBE_TIMEOUT_SECS:
        status = "incomplete-over-budget"
    elif launch_errors or timed_out_labels or overlap_s <= 0:
        status = "inconclusive"
    elif not sibling_covers_tap or not sibling_covers_not_ok or not sibling_covers_last_byte:
        status = "inconclusive-no-sustained-overlap-through-target-output"
    elif not records["sibling"]["valid_control"] or not records["target"]["valid_negative_control"]:
        status = "inconclusive"
    else:
        status = "not-reproduced-under-one-sibling"
    return {
        "name": name,
        "phase": "two-cargo-workers-shared-default-target",
        "cargo_target_directory": target_directory,
        "cargo_target_dir_override": None,
        "cargo_build_jobs_override_present": "CARGO_BUILD_JOBS" in os.environ,
        "cargo_home_configured": "CARGO_HOME" in os.environ,
        "sibling_package": "coding_adventures_test_bootstrap",
        "sibling_command": sibling_argv,
        "sibling_is_in_original_build_plan": True,
        "launch_errors": launch_errors,
        "elapsed_s": elapsed_s,
        "over_budget": elapsed_s > PROBE_TIMEOUT_SECS,
        "outer_probe_timeout_s": PROBE_TIMEOUT_SECS,
        "report_reserve_s": CONCURRENT_REPORT_RESERVE_SECS,
        "overlap_s": round(overlap_s, 3),
        "sibling_covers_target_first_tap": sibling_covers_tap,
        "sibling_covers_target_first_not_ok": sibling_covers_not_ok,
        "sibling_covers_target_last_byte": sibling_covers_last_byte,
        "sibling_covers_target_exit": sibling_covers_target_exit,
        "timed_out_labels": timed_out_labels,
        "process_tree_samples": snapshots,
        "target": records["target"],
        "sibling": records["sibling"],
        "readers_finished": all(not reader.is_alive() for reader in readers),
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
            result = run_probe(name, argv, cwd, env)
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
        baseline = run_probe("shared-target-baseline", default_wrapped, cwd, default_env)
        baseline["phase"] = "single-worker-default-target"
        baseline["cargo_target_directory"] = target_path
        baseline["cargo_target_dir_override"] = None
        baseline["valid_negative_control"] = baseline["valid_fault_observation"] and (
            "negctl: fault caught" in baseline["stdout"]["captured"]
        )
        manifest["probes"].append(baseline)
        print(f"shared-target-baseline: exit-{baseline['exit_code']}, {baseline['elapsed_s']:.3f}s", flush=True)

        sibling_cargo = ["cargo", "test", "--locked", "-p", "coding_adventures_test_bootstrap"]
        print("\n=== shared-target-sibling-concurrent ===", flush=True)
        concurrent = run_concurrent_probe(
            "shared-target-sibling-concurrent",
            default_wrapped,
            sibling_cargo,
            cwd,
            sibling_env,
            target_path,
        )
        manifest["probes"].append(concurrent)
        print(
            f"shared-target-sibling-concurrent: target-exit-{concurrent['target']['exit_code']}, "
            f"sibling-exit-{concurrent['sibling']['exit_code']}, {concurrent['elapsed_s']:.3f}s",
            flush=True,
        )
    except Exception as exc:  # the report is retained even when setup fails
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
                        f"- `{probe['name']}`: {probe['interpretation']}; overlap "
                        f"`{probe['overlap_s']}s`; target exit `{probe['target'].get('exit_code')}`, "
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
    complete = len(manifest["probes"]) == 6 and all(
        probe["readers_finished"]
        and not probe["over_budget"]
        and any("processes" in sample for sample in probe["process_tree_samples"])
        for probe in manifest["probes"]
    )
    complete = complete and all(
        probe.get("valid_fault_observation", False)
        for probe in manifest["probes"]
        if "target" not in probe
    )
    complete = complete and manifest["probes"][-1].get("interpretation") == "not-reproduced-under-one-sibling"
    return 0 if complete else 2


if __name__ == "__main__":
    raise SystemExit(main())
