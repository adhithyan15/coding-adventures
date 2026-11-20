#!/usr/bin/env python3
"""Bounded Windows diagnostics for the vos-sched-any negctl timeout.

This is an investigation helper only. It does not change the negctl pass rule,
the package BUILD, or the 1200-second default. The diagnostic run sets a lower
180-second deadline; a timeout is recorded as inconclusive/failure, never as a
caught fault.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

FAULT_ENV = {
    "WARRANT_EVIDENCE_FAULT": "field-flip",
    "WARRANT_EVIDENCE_ATTACK": "alpha_w",
    "WARRANT_EVIDENCE_FILTER": "/exhaustive/dispatch_covers_run",
}
DIAGNOSTIC_TIMEOUT_SECONDS = 180


def stamp(start: float) -> str:
    return f"+{time.monotonic() - start:.3f}s"


def process_tree(root_pid: int) -> str:
    """Return descendant PID/name/parent only; do not collect command lines."""
    if os.name != "nt":
        return "process-tree-query-unavailable-on-this-host"
    ps = r"""
$all = @(Get-CimInstance Win32_Process)
$ids = @([uint32]$env:DIAG_ROOT_PID)
do {
  $new = @($all | Where-Object { $ids -contains [uint32]$_.ParentProcessId -and $ids -notcontains [uint32]$_.ProcessId } | ForEach-Object { [uint32]$_.ProcessId })
  $ids += $new
} while ($new.Count -gt 0)
$rows = @($all | Where-Object { $ids -contains [uint32]$_.ProcessId } | Select-Object ProcessId,ParentProcessId,Name)
ConvertTo-Json -InputObject $rows -Compress
"""
    env = os.environ.copy()
    env["DIAG_ROOT_PID"] = str(root_pid)
    query_start = time.monotonic()
    result = subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        errors="replace",
        timeout=5,
        env=env,
        check=False,
    )
    elapsed = time.monotonic() - query_start
    return result.stdout.strip() or f"process-tree-query-exit={result.returncode} elapsed={elapsed:.3f}s stderr={result.stderr[-500:]}"


def run_observed(
    label: str,
    argv: list[str],
    cwd: Path,
    timeout: int,
    extra_env: dict[str, str] | None = None,
) -> tuple[int, bytes, bytes, bool, bool]:
    start = time.monotonic()
    env = os.environ.copy()
    env.update(FAULT_ENV)
    if extra_env:
        env.update(extra_env)
    proc = subprocess.Popen(
        argv,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=0,
        creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
    )
    print(f"{label} spawn {stamp(start)} root_pid={proc.pid}", flush=True)
    chunks: dict[str, list[bytes]] = {"stdout": [], "stderr": []}
    counts = {"stdout": 0, "stderr": 0}
    observed = {"stdout": 0, "stderr": 0}
    events: dict[str, dict[str, float | None]] = {
        name: {"first_byte": None, "last_byte": None, "tap_first_byte": None, "eof": None}
        for name in ("stdout", "stderr")
    }
    tap_scan_tail = b""
    event_lock = threading.Lock()
    output_cap = 4 * 1024 * 1024

    def pump(name: str, stream) -> None:
        nonlocal tap_scan_tail
        while True:
            data = stream.read(4096)
            if not data:
                with event_lock:
                    events[name]["eof"] = time.monotonic()
                    print(f"{label} {name}_eof {stamp(start)}", flush=True)
                return
            with event_lock:
                if events[name]["first_byte"] is None:
                    events[name]["first_byte"] = time.monotonic()
                    print(f"{label} {name}_first_byte {stamp(start)}", flush=True)
                observed[name] += len(data)
                events[name]["last_byte"] = time.monotonic()
                if name == "stdout" and events[name]["tap_first_byte"] is None:
                    if b"TAP version 14" in tap_scan_tail + data:
                        events[name]["tap_first_byte"] = time.monotonic()
                        print(f"{label} stdout_tap_first_byte {stamp(start)}", flush=True)
                    tap_scan_tail = (tap_scan_tail + data)[-32:]
                remaining = output_cap - counts[name]
                if remaining > 0:
                    kept = data[:remaining]
                    chunks[name].append(kept)
                    counts[name] += len(kept)

    readers = [
        threading.Thread(target=pump, args=("stdout", proc.stdout), daemon=True),
        threading.Thread(target=pump, args=("stderr", proc.stderr), daemon=True),
    ]
    for reader in readers:
        reader.start()

    deadline = start + timeout
    next_tree = start
    timed_out = False
    exit_at: float | None = None
    while True:
        now = time.monotonic()
        code = proc.poll()
        if code is not None and exit_at is None:
            exit_at = time.monotonic()
            print(f"{label} root_exit {stamp(start)} code={code}", flush=True)
        with event_lock:
            first_seen = {name: item["first_byte"] for name, item in events.items()}
            eof_seen = {name: item["eof"] for name, item in events.items()}
        if now >= next_tree and (code is None or any(v is None for v in eof_seen.values())):
            try:
                print(f"{label} process_tree {stamp(start)} {process_tree(proc.pid)}", flush=True)
            except subprocess.TimeoutExpired:
                print(f"{label} process_tree {stamp(start)} query_timeout", flush=True)
            next_tree = now + 10
            now = time.monotonic()
            code = proc.poll()
            if code is not None and exit_at is None:
                exit_at = now
                print(f"{label} root_exit {stamp(start)} code={code}", flush=True)
            with event_lock:
                eof_seen = {name: item["eof"] for name, item in events.items()}

        if code is not None and all(v is not None for v in eof_seen.values()):
            if now > deadline or (exit_at is not None and exit_at > deadline) or any(
                value is not None and value > deadline for value in eof_seen.values()
            ):
                timed_out = True
                print(f"{label} completion observed after diagnostic deadline; failure/inconclusive", flush=True)
            break
        if now >= deadline:
            timed_out = True
            print(f"{label} diagnostic_deadline {stamp(start)}; failure/inconclusive", flush=True)
            if proc.poll() is None:
                if os.name == "nt":
                    try:
                        subprocess.run(
                            ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT,
                            timeout=20,
                            check=False,
                        )
                    except subprocess.TimeoutExpired:
                        print(f"{label} taskkill timed out; diagnostic remains failure/inconclusive", flush=True)
                else:
                    proc.kill()
                try:
                    proc.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    print(f"{label} root_not_reaped after timeout; no stale-PID kill attempted", flush=True)
            break
        time.sleep(0.1)

    for reader in readers:
        reader.join(timeout=10)
    with event_lock:
        first_seen = {name: item["first_byte"] for name, item in events.items()}
        last_seen = {name: item["last_byte"] for name, item in events.items()}
        tap_seen = {name: item["tap_first_byte"] for name, item in events.items()}
        eof_seen = {name: item["eof"] for name, item in events.items()}

    def since_start(value: float | None) -> str:
        return "not-observed" if value is None else f"{value - start:.3f}s"

    print(
        f"{label} summary root_exit={since_start(exit_at)} "
        f"stdout_first={since_start(first_seen['stdout'])} stdout_last={since_start(last_seen['stdout'])} stdout_eof={since_start(eof_seen['stdout'])} "
        f"stdout_tap_first={since_start(tap_seen['stdout'])} "
        f"stderr_first={since_start(first_seen['stderr'])} stderr_last={since_start(last_seen['stderr'])} stderr_eof={since_start(eof_seen['stderr'])} "
        f"stdout_bytes={observed['stdout']} stderr_bytes={observed['stderr']} "
        f"stdout_kept={counts['stdout']} stderr_kept={counts['stderr']} output_cap={output_cap} "
        f"capture_complete={all(v is not None for v in eof_seen.values()) and observed == counts} timed_out={timed_out}",
        flush=True,
    )

    capture_complete = all(v is not None for v in eof_seen.values()) and observed == counts
    return (
        proc.returncode if proc.returncode is not None else -1,
        b"".join(chunks["stdout"]),
        b"".join(chunks["stderr"]),
        timed_out,
        capture_complete,
    )


def direct_tap_matches(stdout: bytes, code: int, timed_out: bool, capture_complete: bool) -> bool:
    text = stdout.decode("utf-8", errors="replace")
    lines = [line.rstrip("\r") for line in text.splitlines()]
    expected_messages = {
        '  message: "exhaustive suite falsified the claim; repro-fnv: 4ed3ce480a58b0c9"',
        '  message: "exhaustive suite falsified the claim; counterexample: [Fixed(U8, 0), Fixed(U8, 0)]"',
    }
    observed_messages = [line for line in lines if line.startswith("  message: ")]
    return (
        not timed_out
        and capture_complete
        and code == 1
        and lines.count("TAP version 14") == 1
        and lines.count("1..1") == 1
        and lines.count("not ok 1 - warrant/vos_sched_any/exhaustive/dispatch_covers_run") == 1
        and sum(line.startswith(("ok ", "not ok ")) for line in lines) == 1
        and lines.count('  failure_id: "property-counterexample"') == 1
        and len(observed_messages) == 1
        and observed_messages[0] in expected_messages
        and lines.count("  ...") == 1
        and "Bail out!" not in text
    )


def raw_probe_accepted(
    stdout: bytes, stderr: bytes, code: int, timed_out: bool, capture_complete: bool
) -> bool:
    return direct_tap_matches(stdout, code, timed_out, capture_complete) and b"Bail out!" not in stderr


def write_capture(path: Path, label: str, stdout: bytes, stderr: bytes) -> None:
    path.mkdir(parents=True, exist_ok=True)
    (path / f"{label}.stdout.txt").write_bytes(stdout)
    (path / f"{label}.stderr.txt").write_bytes(stderr)


def main() -> int:
    workspace = Path(__file__).resolve().parents[1] / "packages" / "rust"
    package_dir = workspace / "vos-sched-any"
    negctl = Path(__file__).resolve().parent / "warrant_negctl.py"
    out_dir = Path(os.environ.get("DIAG_OUT", "diagnostic-output")).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    cargo = os.environ.get("CARGO", "cargo")

    print("diagnostic exact env=" + json.dumps(FAULT_ENV, sort_keys=True), flush=True)
    build_start = time.monotonic()
    capture_cap = 32 * 1024 * 1024
    with tempfile.TemporaryFile() as build_stdout, tempfile.TemporaryFile() as build_stderr:
        try:
            build = subprocess.run(
                [cargo, "test", "--locked", "-p", "vos-sched-any", "--features", "ghost", "--test", "warrant_evidence", "--no-run", "--message-format=json"],
                cwd=workspace,
                stdin=subprocess.DEVNULL,
                stdout=build_stdout,
                stderr=build_stderr,
                timeout=1500,
                check=False,
            )
        except subprocess.TimeoutExpired:
            print(f"prebuild diagnostic timeout after 1500s; inconclusive; elapsed={time.monotonic() - build_start:.3f}s", flush=True)
            return 10
        stdout_size, stderr_size = build_stdout.tell(), build_stderr.tell()
        print(f"prebuild exit={build.returncode} elapsed={time.monotonic() - build_start:.3f}s stdout_bytes={stdout_size} stderr_bytes={stderr_size}", flush=True)
        if stdout_size > capture_cap or stderr_size > capture_cap:
            print("prebuild compiler output exceeded the 32 MiB retention cap; diagnostic inconclusive", flush=True)
            return 10
        build_stdout.seek(0)
        build_stderr.seek(0)
        build_out = build_stdout.read(capture_cap)
        build_err = build_stderr.read(capture_cap)
    (out_dir / "cargo-no-run.stdout.jsonl").write_bytes(build_out)
    (out_dir / "cargo-no-run.stderr.txt").write_bytes(build_err)
    if build.returncode != 0:
        return 10

    executables: list[str] = []
    target_identity_errors: list[str] = []
    expected_manifest = (package_dir / "Cargo.toml").resolve()
    expected_test = (package_dir / "tests" / "warrant_evidence.rs").resolve()
    for raw in build_out.splitlines():
        try:
            row = json.loads(raw)
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        if row.get("reason") != "compiler-artifact":
            continue
        target = row.get("target") or {}
        profile = row.get("profile") or {}
        path = row.get("executable")
        if target.get("name") == "warrant_evidence":
            manifest = row.get("manifest_path")
            source = target.get("src_path")
            if (
                target.get("kind") != ["test"]
                or profile.get("test") is not True
                or not manifest
                or Path(manifest).resolve() != expected_manifest
                or not source
                or Path(source).resolve() != expected_test
                or not path
            ):
                target_identity_errors.append(json.dumps(row, sort_keys=True))
            else:
                executables.append(path)
    if target_identity_errors or len(executables) != 1 or not Path(executables[0]).is_file():
        print("prebuild did not yield a unique existing warrant_evidence test executable", flush=True)
        if target_identity_errors:
            print("target identity mismatch: " + " | ".join(target_identity_errors), flush=True)
        return 11
    executable = executables[0]
    print(f"warrant_evidence_executable={executable}", flush=True)

    failures: list[str] = []

    # Direct prebuilt executable: rules out Cargo compilation/lock acquisition
    # while preserving the same real integration-test binary and Warrant env.
    code, stdout, stderr, timed_out, complete = run_observed(
        "direct-prebuilt", [executable], package_dir, DIAGNOSTIC_TIMEOUT_SECONDS
    )
    write_capture(out_dir, "direct-prebuilt", stdout, stderr)
    if not raw_probe_accepted(stdout, stderr, code, timed_out, complete):
        failures.append(f"direct-prebuilt failed/inconclusive: code={code} timeout={timed_out}")

    # Warm Cargo, but outside the negctl wrapper, to separate Cargo's own
    # completion/pipe behavior from the checker process.
    cargo_test = [cargo, "test", "-q", "-p", "vos-sched-any", "--features", "ghost", "--test", "warrant_evidence"]
    code, stdout, stderr, timed_out, complete = run_observed(
        "cargo-direct", cargo_test, package_dir, DIAGNOSTIC_TIMEOUT_SECONDS
    )
    write_capture(out_dir, "cargo-direct", stdout, stderr)
    if not raw_probe_accepted(stdout, stderr, code, timed_out, complete):
        failures.append(f"cargo-direct failed/inconclusive: code={code} timeout={timed_out}")

    # Official negctl with the exact prebuilt target. Its own diagnostic timeout
    # is lower than the unchanged 1200s default; timeout remains failure.
    wrapper_env = {"WARRANT_NEGCTL_TIMEOUT_SECS": str(DIAGNOSTIC_TIMEOUT_SECONDS)}
    direct_negctl = [sys.executable, str(negctl), "--", executable]
    code, stdout, stderr, timed_out, complete = run_observed(
        "official-negctl-prebuilt", direct_negctl, package_dir, DIAGNOSTIC_TIMEOUT_SECONDS + 30, wrapper_env
    )
    write_capture(out_dir, "official-negctl-prebuilt", stdout, stderr)
    if timed_out or not complete or code != 0 or stdout.strip() != b"negctl: fault caught (exit 1, TAP plan satisfied, 'not ok' present)" or stderr:
        failures.append(f"official negctl on prebuilt binary failed/inconclusive: code={code} timeout={timed_out}")

    # Cargo control, now warm from --no-run: compare command-wrapper/target-lock
    # behavior against the direct invocation above.
    code, stdout, stderr, timed_out, complete = run_observed(
        "official-negctl-cargo", [sys.executable, str(negctl), "--", *cargo_test], package_dir, DIAGNOSTIC_TIMEOUT_SECONDS + 30, wrapper_env
    )
    write_capture(out_dir, "official-negctl-cargo", stdout, stderr)
    if timed_out or not complete or code != 0 or stdout.strip() != b"negctl: fault caught (exit 1, TAP plan satisfied, 'not ok' present)" or stderr:
        failures.append(f"official negctl on Cargo invocation failed/inconclusive: code={code} timeout={timed_out}")
    if failures:
        print("diagnostic failures: " + "; ".join(failures), flush=True)
        return 12
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
