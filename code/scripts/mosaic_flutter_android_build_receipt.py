#!/usr/bin/env python3
"""Emit bounded, fixed-field identity for the opt-in Flutter Android build."""

from __future__ import annotations

import hashlib
import os
import re
import selectors
import shutil
import signal
import subprocess
import sys
import time
import zipfile
from pathlib import Path


MAX_DEX_OUTPUT = 1_000_000
MAX_TOOL_STDERR = 64_000
MAX_REGISTRANT_SOURCE = 512_000
TOOL_TIMEOUT_SECONDS = 15


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def apk_member_status(apk: Path, member: str) -> str:
    try:
        with zipfile.ZipFile(apk) as archive:
            return "present" if member in archive.namelist() else "absent"
    except (OSError, zipfile.BadZipFile):
        return "unavailable"


def _run_bounded(argv: list[str], stdout_limit: int) -> tuple[int, bytes] | None:
    """Read bounded tool output and kill only its private process group on error."""
    try:
        process = subprocess.Popen(
            argv,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=(os.name == "posix"),
        )
    except OSError:
        return None
    assert process.stdout is not None and process.stderr is not None
    stdout_fd = process.stdout.fileno()
    stderr_fd = process.stderr.fileno()
    streams = {stdout_fd: bytearray(), stderr_fd: bytearray()}
    limits = {stdout_fd: stdout_limit, stderr_fd: MAX_TOOL_STDERR}
    selector = selectors.DefaultSelector()
    deadline = time.monotonic() + TOOL_TIMEOUT_SECONDS
    done = False

    def stop_owned_group() -> None:
        if os.name == "posix":
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except OSError:
                try:
                    process.kill()
                except OSError:
                    pass
        else:
            try:
                process.kill()
            except OSError:
                pass
        try:
            process.wait(timeout=0.5)
        except (OSError, subprocess.TimeoutExpired):
            pass

    try:
        selector.register(process.stdout, selectors.EVENT_READ)
        selector.register(process.stderr, selectors.EVENT_READ)
        failed = False
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                failed = True
                break
            ready = selector.select(remaining)
            if not ready:
                failed = True
                break
            for key, _ in ready:
                fd = key.fileobj.fileno()
                chunk = os.read(fd, min(16_384, limits[fd] + 1 - len(streams[fd])))
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                streams[fd].extend(chunk)
                if len(streams[fd]) > limits[fd]:
                    failed = True
                    break
            if failed:
                break
        if failed:
            stop_owned_group()
            done = True
            return None
        remaining = max(0.0, deadline - time.monotonic())
        try:
            return_code = process.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            stop_owned_group()
            done = True
            return None
        done = True
        return return_code, bytes(streams[stdout_fd])
    except Exception:
        if not done:
            stop_owned_group()
            done = True
        return None
    finally:
        selector.close()
        for stream in (process.stdout, process.stderr):
            if not stream.closed:
                stream.close()
        if not done:
            stop_owned_group()

def generated_registrant_status(path: Path) -> str:
    try:
        if path.stat().st_size > MAX_REGISTRANT_SOURCE:
            return "unavailable"
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return "unavailable"
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    text = re.sub(r"(?m)^\s*//.*$", "", text)
    target_import = re.search(
        r"(?m)^\s*import\s+'package:path_provider_android/path_provider_android\.dart'\s+as\s+path_provider_android;\s*$",
        text,
    )
    if target_import is None:
        if re.search(r"(?m)^\s*import\s+'package:path_provider_android/", text):
            return "unavailable"
        return "absent"
    android_arm = re.search(
        r"(?ms)^\s*if\s*\(Platform\.isAndroid\)\s*\{(.*?)^\s*\}\s*else\s+if\s*\(",
        text,
    )
    if android_arm is None:
        return "unavailable"
    registered = re.search(
        r"(?m)^\s*path_provider_android\.PathProviderAndroid\.registerWith\(\);\s*$",
        android_arm.group(1),
    )
    return "present" if registered else "absent"


def package_id_status(aapt2: Path, apk: Path) -> str:
    result = _run_bounded([str(aapt2), "dump", "badging", str(apk)], 64_000)
    if result is None or result[0] != 0:
        return "unavailable"
    try:
        output = result[1].decode("utf-8")
    except UnicodeError:
        return "unavailable"
    match = re.search(r"^package: name='([A-Za-z0-9._]+)'(?: |$)", output, re.MULTILINE)
    if match is None or len(match.group(1)) > 255:
        return "unavailable"
    return match.group(1)


def _constructed_registers(dex_text: str, class_name: str) -> set[str]:
    descriptor = "L" + class_name.replace(".", "/") + ";"
    allocated = set(re.findall(
        r"(?m)^\s*new-instance\s+(v[0-9]+),\s+" + re.escape(descriptor) + r"\s*$",
        dex_text,
    ))
    constructed = set(re.findall(
        r"(?m)^\s*invoke-direct\s+\{(v[0-9]+)\},\s+"
        + re.escape(descriptor)
        + r"-><init>\([^\n]*\)V\s*$",
        dex_text,
    ))
    return allocated & constructed


def _selected_register_method(dex_text: str) -> str | None:
    expected = re.compile(
        r"^\s*\.method\s+(?:[A-Za-z_$-]+\s+)*"
        r"registerWith\(Lio/flutter/embedding/engine/FlutterEngine;\)V\s*$"
    )
    lines = dex_text.splitlines()
    selected: list[str] = []
    start: int | None = None
    for index, line in enumerate(lines):
        if line.strip().startswith(".method "):
            if start is not None:
                return None
            if expected.fullmatch(line):
                start = index
        elif line.strip() == ".end method" and start is not None:
            selected.append("\n".join(lines[start : index + 1]))
            start = None
    if start is not None or len(selected) != 1:
        return None
    return selected[0]


def _has_supported_smali_instruction(method: str) -> bool:
    return any(
        re.fullmatch(r"\s*new-instance\s+v[0-9]+,\s+L[^\s;]+;\s*", line)
        or re.fullmatch(
            r"\s*invoke-direct\s+\{v[0-9]+\},\s+L[^\s;]+;-><init>\([^\n]*\)V\s*",
            line,
        )
        for line in method.splitlines()
    )


def dex_registrant_statuses(apk: Path, android_home: str) -> tuple[str, str]:
    analyzer = shutil.which("apkanalyzer")
    if analyzer is None and android_home:
        candidate = Path(android_home) / "cmdline-tools" / "latest" / "bin" / "apkanalyzer"
        try:
            if candidate.is_file() and candidate.stat().st_mode & 0o111:
                analyzer = str(candidate)
        except OSError:
            pass
    if analyzer is None:
        return "unavailable", "unavailable"
    result = _run_bounded(
        [
            analyzer,
            "dex",
            "code",
            "--class",
            "io.flutter.plugins.GeneratedPluginRegistrant",
            "--method",
            "registerWith(Lio/flutter/embedding/engine/FlutterEngine;)V",
            str(apk),
        ],
        MAX_DEX_OUTPUT,
    )
    if result is None or result[0] != 0:
        return "unavailable", "unavailable"
    try:
        text = result[1].decode("utf-8")
    except UnicodeError:
        return "unavailable", "unavailable"
    # Refuse to label an unfamiliar/empty analyzer format as absence. Parse
    # only the exact selected method, even if a tool version emits the class.
    method = _selected_register_method(text)
    if method is None or not _has_supported_smali_instruction(method):
        return "unavailable", "unavailable"
    statuses: list[str] = []
    for class_name in (
        "com.github.dart_lang.jni.JniPlugin",
        "com.github.dart_lang.jni_flutter.JniFlutterPlugin",
    ):
        descriptor = "L" + class_name.replace(".", "/") + ";"
        if _constructed_registers(method, class_name):
            statuses.append("present")
        elif descriptor in method:
            # An unfamiliar instruction/string form may name the class but
            # cannot establish an executable constructor call.
            statuses.append("unavailable")
        else:
            statuses.append("absent")
    return statuses[0], statuses[1]


def file_hash_status(path: Path) -> str:
    try:
        return sha256(path)
    except OSError:
        return "unavailable"


def main(argv: list[str]) -> int:
    unavailable = (
        "MOSAIC_BUILD_DIAG v=1 apk_sha256=unavailable lock_sha256=unavailable "
        "package_id=unavailable x86_64_libdartjni=unavailable "
        "dart_registrant_source=unavailable dex_jni_plugin=unavailable "
        "dex_jni_flutter_plugin=unavailable"
    )
    if len(argv) != 6:
        print(unavailable)
        return 0
    try:
        apk, lock, registrant, aapt2 = (Path(value) for value in argv[1:5])
        android_home = argv[5]
        apk_hash = file_hash_status(apk)
        lock_hash = file_hash_status(lock)
        package_id = package_id_status(aapt2, apk)
        x86_jni = apk_member_status(apk, "lib/x86_64/libdartjni.so")
        dart_registrant = generated_registrant_status(registrant)
        dex_jni, dex_jni_flutter = dex_registrant_statuses(apk, android_home)
        print(
            "MOSAIC_BUILD_DIAG v=1 "
            f"apk_sha256={apk_hash} lock_sha256={lock_hash} package_id={package_id} "
            f"x86_64_libdartjni={x86_jni} dart_registrant_source={dart_registrant} "
            f"dex_jni_plugin={dex_jni} dex_jni_flutter_plugin={dex_jni_flutter}"
        )
    except Exception:
        print(unavailable)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
