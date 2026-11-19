#!/usr/bin/env python3
"""Emit bounded, fixed-field identity for the opt-in Flutter Android build."""

from __future__ import annotations

import hashlib
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path


MAX_DEX_OUTPUT = 1_000_000
DEX_TIMEOUT_SECONDS = 15


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


def generated_registrant_status(path: Path) -> str:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return "unavailable"
    return (
        "present"
        if "path_provider_android" in text
        and "PathProviderAndroid.registerWith()" in text
        else "absent"
    )


def package_id_status(aapt2: Path, apk: Path) -> str:
    try:
        result = subprocess.run(
            [str(aapt2), "dump", "badging", str(apk)],
            check=False,
            capture_output=True,
            text=True,
            timeout=DEX_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    if result.returncode != 0:
        return "unavailable"
    match = re.search(r"^package: name='([A-Za-z0-9._]+)'(?: |$)", result.stdout, re.MULTILINE)
    if match is None or len(match.group(1)) > 255:
        return "unavailable"
    return match.group(1)


def _constructed_registers(dex_text: str, class_name: str) -> set[str]:
    descriptor = "L" + class_name.replace(".", "/") + ";"
    allocated = set(
        re.findall(r"new-instance\s+(v[0-9]+),\s+" + re.escape(descriptor), dex_text)
    )
    constructed = set(
        re.findall(
            r"invoke-direct\s+\{(v[0-9]+)\},\s+"
            + re.escape(descriptor)
            + r"-><init>\(",
            dex_text,
        )
    )
    return allocated & constructed


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
    try:
        result = subprocess.run(
            [
                analyzer,
                "dex",
                "code",
                "--class",
                "io.flutter.plugins.GeneratedPluginRegistrant",
                str(apk),
            ],
            check=False,
            capture_output=True,
            timeout=DEX_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable", "unavailable"
    if result.returncode != 0 or len(result.stdout) > MAX_DEX_OUTPUT:
        return "unavailable", "unavailable"
    try:
        text = result.stdout.decode("utf-8")
    except UnicodeError:
        return "unavailable", "unavailable"
    # Refuse to label an unfamiliar/empty analyzer format as absence. The
    # method body must expose a registration method and executable DEX ops.
    if "registerWith" not in text or not re.search(r"\b(new-instance|invoke-direct)\b", text):
        return "unavailable", "unavailable"
    jni = _constructed_registers(text, "com.github.dart_lang.jni.JniPlugin")
    jni_flutter = _constructed_registers(
        text, "com.github.dart_lang.jni_flutter.JniFlutterPlugin"
    )
    return ("present" if jni else "absent", "present" if jni_flutter else "absent")


def file_hash_status(path: Path) -> str:
    try:
        return sha256(path)
    except OSError:
        return "unavailable"


def main(argv: list[str]) -> int:
    if len(argv) != 6:
        print(
            "MOSAIC_BUILD_DIAG v=1 apk_sha256=unavailable lock_sha256=unavailable "
            "package_id=unavailable x86_64_libdartjni=unavailable "
            "dart_registrant_source=unavailable dex_jni_plugin=unavailable "
            "dex_jni_flutter_plugin=unavailable"
        )
        return 0
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
