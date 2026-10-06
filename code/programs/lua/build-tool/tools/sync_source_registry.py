"""Compile the reviewed source-input registry into inert Lua package data.

The runtime imports the generated table; it never locates a repository fixture.
Both --check and --sync refuse linked source, target, or parent entries.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import struct
import tempfile
from pathlib import Path

HERE = Path(__file__).absolute().parent
PACKAGE = HERE.parent
REPOSITORY = PACKAGE.parents[3]
SOURCE = REPOSITORY / "code/specs/fixtures/build-tool-v1/language-source-input-registry.json"
TARGET = PACKAGE / "lib/build_tool/source_registry_data.lua"
DOMAIN = b"coding-adventures/build-tool-language-source-input-registry/v1\0"
EXPECTED_DIGEST = "5201a045ea3e2086fd9be316f2692743ca329f1d84f1c0983a0da47e96b3f621"


def checked_entry(path: Path, *, directory: bool) -> None:
    """Reject symlinks/reparse points before opening a privileged sync path."""
    entry = path.lstat()
    expected = stat.S_ISDIR if directory else stat.S_ISREG
    if (not expected(entry.st_mode) or path.is_symlink()
            or getattr(entry, "st_file_attributes", 0) & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)):
        raise ValueError(f"source registry sync requires a regular {'directory' if directory else 'file'}: {path}")


def checked_parents() -> None:
    for path in (
        REPOSITORY,
        REPOSITORY / "code",
        REPOSITORY / "code/specs",
        REPOSITORY / "code/specs/fixtures",
        SOURCE.parent,
        REPOSITORY / "code/programs",
        REPOSITORY / "code/programs/lua",
        PACKAGE,
        HERE,
        PACKAGE / "lib",
        TARGET.parent,
    ):
        checked_entry(path, directory=True)


def lua_string(value: str) -> str:
    """Render UTF-8 bytes, escaping every non-printable byte unambiguously."""
    result = ['"']
    for byte in value.encode("utf-8"):
        if byte == 34:
            result.append('\\"')
        elif byte == 92:
            result.append("\\\\")
        elif 32 <= byte < 127:
            result.append(chr(byte))
        else:
            result.append(f"\\{byte:03d}")
    result.append('"')
    return "".join(result)


def lua_value(value: object, depth: int = 0) -> str:
    pad = "  " * depth
    inner = "  " * (depth + 1)
    if isinstance(value, dict):
        fields = [f"{inner}[{lua_string(key)}] = {lua_value(item, depth + 1)},"
                  for key, item in sorted(value.items())]
        return "{\n" + "\n".join(fields) + f"\n{pad}}}" if fields else "{}"
    if isinstance(value, list):
        fields = [f"{inner}{lua_value(item, depth + 1)}," for item in value]
        return "{\n" + "\n".join(fields) + f"\n{pad}}}" if fields else "{}"
    if isinstance(value, str):
        return lua_string(value)
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    raise ValueError(f"unsupported registry value type: {type(value).__name__}")


def expected_output() -> bytes:
    checked_parents()
    checked_entry(SOURCE, directory=False)
    registry = json.loads(SOURCE.read_bytes())
    canonical = json.dumps(registry, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    digest = hashlib.sha256(DOMAIN + struct.pack(">Q", len(canonical)) + canonical).hexdigest()
    if digest != EXPECTED_DIGEST:
        raise ValueError(f"checked source registry digest changed: {digest}")
    return ("-- Generated from the reviewed language source-input registry.\n"
            "-- Do not edit; run tools/sync_source_registry.py --sync.\n"
            f"return {{ digest = {lua_string(digest)}, data = {lua_value(registry)} }}\n").encode()


def main() -> None:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--sync", action="store_true")
    options = parser.parse_args()
    expected = expected_output()
    try:
        checked_entry(TARGET, directory=False)
        exists = True
    except FileNotFoundError:
        exists = False
    if options.check:
        if not exists or TARGET.read_bytes() != expected:
            raise SystemExit("Lua source-input registry projection is stale")
        return
    handle, temporary_name = tempfile.mkstemp(prefix=".source-registry-", suffix=".lua", dir=TARGET.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(handle, "wb") as output:
            output.write(expected)
            output.flush()
            os.fsync(output.fileno())
        checked_parents()
        if exists:
            checked_entry(TARGET, directory=False)
        os.replace(temporary, TARGET)
    finally:
        temporary.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
