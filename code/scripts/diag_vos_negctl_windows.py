#!/usr/bin/env python3
"""Run four bounded probes for the VOS Windows negative-control timeout.

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
    return {
        "name": name,
        "argv": argv,
        "cwd": str(cwd),
        "root_pid": proc.pid,
        "root_process_name": Path(argv[0]).name,
        "root_parent_pid": os.getpid(),
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
        "process_tree_samples": snapshots,
    }


def build_test_binary(repo_root: Path, target_dir: Path) -> Path:
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
    build_env["CARGO_TARGET_DIR"] = str(target_dir)
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
    target_dir = artifact_dir / "target"
    manifest: dict[str, Any] = {
        "candidate_sha": args.candidate_sha,
        "platform": sys.platform,
        "fault": FAULT,
        "filter": FILTER,
        "production_wrapper_default_timeout_s": PRODUCTION_TIMEOUT_SECS,
        "diagnostic_wrapper_timeout_s": DIAGNOSTIC_WRAPPER_TIMEOUT_SECS,
        "outer_probe_timeout_s": PROBE_TIMEOUT_SECS,
        "build": {"started": True},
        "probes": [],
    }
    try:
        executable = build_test_binary(repo_root, target_dir)
        manifest["build"] = {"exit_code": 0, "executable": str(executable)}
        rust_root = repo_root / "code" / "packages" / "rust"
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
        base_env["CARGO_TARGET_DIR"] = str(target_dir)
        prebuilt_env = base_env.copy()
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
                summary.write(
                    f"- `{probe['name']}`: exit `{probe['exit_code']}`, "
                    f"outer-timeout `{probe['outer_timeout']}`, over-budget `{probe['over_budget']}`, "
                    f"{probe['elapsed_s']}s, "
                    f"stdout/stderr EOF `{probe['stdout']['eof_s']}` / `{probe['stderr']['eof_s']}`.\n"
                )
            if "setup_error" in manifest:
                summary.write(f"\nSetup error: `{manifest['setup_error']}`\n")
    print(f"\nDiagnostic report: {report_path}", flush=True)
    complete = len(manifest["probes"]) == 4 and all(
        probe["readers_finished"]
        and not probe["over_budget"]
        and any("processes" in sample for sample in probe["process_tree_samples"])
        for probe in manifest["probes"]
    )
    return 0 if complete else 2


if __name__ == "__main__":
    raise SystemExit(main())
