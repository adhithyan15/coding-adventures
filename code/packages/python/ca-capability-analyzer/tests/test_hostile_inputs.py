"""Hostile-input, edge-case, and cross-stack regression tests (VER08 / VER15)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from coding_adventures_ca_capability_analyzer.analyzer import analyze_package
from coding_adventures_ca_capability_analyzer.cli import main
from coding_adventures_ca_capability_analyzer.manifest import (
    MAX_MANIFEST_ITEMS,
    load_manifest,
    normalize_capability,
)
from coding_adventures_ca_capability_analyzer.scanner import scan_source
from coding_adventures_ca_capability_analyzer.types import (
    CAP_DYNAMIC_EVAL,
    CAP_ENV_READ,
    CAP_ENV_WRITE,
    CAP_FFI_UNSAFE,
    CAP_FS_READ,
    CAP_FS_WRITE,
    CAP_PROCESS_EXEC,
    CAP_RAND_READ,
    CAP_THREAD_SPAWN,
    CAP_TIME_READ,
    CAP_TIME_SLEEP,
)


def test_hostile_manifest_inputs_fail_closed(tmp_path: Path) -> None:
    p = tmp_path / "required_capabilities.json"

    # Duplicate JSON keys rejected (CWE-20 / CWE-1321)
    p.write_text(
        '{"version": 1, "version": 2, "capabilities": []}',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="Duplicate JSON key"):
        load_manifest(p)

    # Oversized capabilities list
    p.write_text(
        json.dumps(
            {"version": 1, "capabilities": ["fs:read"] * (MAX_MANIFEST_ITEMS + 1)}
        ),
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="exceeds max items"):
        load_manifest(p)

    # Oversized token length & non-string category/action
    with pytest.raises(ValueError, match="exceeds max length"):
        normalize_capability("x" * 200)
    with pytest.raises(ValueError, match="must be strings"):
        normalize_capability({"category": 123, "action": "read"})
    with pytest.raises(ValueError, match="exceeds max length"):
        normalize_capability({"category": "x" * 200, "action": "read"})


def test_cross_stack_capability_normalization_and_evasion_sinks() -> None:
    assert normalize_capability("time:clock") == CAP_TIME_READ
    assert normalize_capability("random:entropy") == CAP_RAND_READ
    assert normalize_capability("process:spawn") == CAP_PROCESS_EXEC
    assert normalize_capability("net:socket") == "net:client"
    assert normalize_capability({"category": "env", "action": "read"}) == CAP_ENV_READ
    assert normalize_capability({"category": "env", "action": "write"}) == CAP_ENV_WRITE
    assert normalize_capability({"category": "time", "action": "read"}) == CAP_TIME_READ
    assert (
        normalize_capability({"category": "time", "action": "sleep"}) == CAP_TIME_SLEEP
    )
    assert normalize_capability({"category": "rand", "action": "read"}) == CAP_RAND_READ
    assert (
        normalize_capability({"category": "thread", "action": "spawn"})
        == CAP_THREAD_SPAWN
    )

    src = """
import os
import time
import random
import secrets
import threading
import cffi
import marshal
import yaml
import importlib
from pathlib import Path

def f(mod_name: str, p: Path):
    __import__("os")
    importlib.import_module(mod_name)
    marshal.loads(b"")
    yaml.unsafe_load("{}")
    _ = os.environ["HOME"]
    os.environ["X"] = "1"
    os.getenv("PATH")
    os.putenv("A", "B")
    _ = time.time()
    time.sleep(0.01)
    _ = random.random()
    _ = secrets.token_hex(8)
    _ = os.urandom(4)
    _ = threading.Thread(target=lambda: None)
    ffi = cffi.FFI()
    p.replace(Path("dest.txt"))
    return p.exists()
"""
    res = scan_source(src, "evasion.py")
    for cap in (
        CAP_DYNAMIC_EVAL,
        CAP_ENV_READ,
        CAP_ENV_WRITE,
        CAP_TIME_READ,
        CAP_TIME_SLEEP,
        CAP_RAND_READ,
        CAP_THREAD_SPAWN,
        CAP_FFI_UNSAFE,
        CAP_FS_READ,
        CAP_FS_WRITE,
    ):
        assert cap in res.capabilities, f"missing {cap} in {res.capabilities}"
    cwe94 = [f for f in res.findings if f.cwe == "CWE-94"]
    assert len(cwe94) >= 4


def test_cross_stack_regression_wav_sink_and_music_machine_patterns() -> None:
    # Regression 1: os.open(path, os.O_RDONLY | os.O_BINARY) is fs:read only;
    # os.open(path, os.O_WRONLY | os.O_CREAT) triggers fs:write.
    src_ro = """
import os
def read_only(p):
    fd = os.open(p, os.O_RDONLY | getattr(os, "O_BINARY", 0))
    return fd
"""
    res_ro = scan_source(src_ro, "ro.py")
    assert CAP_FS_READ in res_ro.capabilities
    assert CAP_FS_WRITE not in res_ro.capabilities

    src_rw_default = """
import os
def open_unknown_flags(p, flags):
    return os.open(p, flags)
def open_bare(p):
    return os.open(p)
def open_const(p):
    return os.open(p, 0)
"""
    res_unk = scan_source(src_rw_default, "unk.py")
    assert CAP_FS_READ in res_unk.capabilities
    assert CAP_FS_WRITE in res_unk.capabilities

    # Regression 2: str.replace() must NOT trigger fs:write, while Path.replace() does
    # and filenames with '..' inside a stem ('v1..v2.wav') must NOT trigger CWE-22.
    src_pure = """
from importlib import import_module

def clean_name(text: str) -> str:
    return text.replace("\\\\", "/")

def lazy_load():
    return import_module("audio_device_sink")
"""
    res_pure = scan_source(src_pure, "pure.py")
    assert len(res_pure.capabilities) == 0
    assert len(res_pure.findings) == 0

    res_dot = scan_source('open("v1..v2.wav", "rb")\n', "dot.py")
    assert not any(f.cwe == "CWE-22" for f in res_dot.findings)

    # Regression 3: Path("../../../etc/passwd") and os.remove("../x") trigger CWE-22
    res_trav = scan_source(
        'import os\nfrom pathlib import Path\nPath("../../../etc/passwd")\n'
        'os.remove("../evil")\nPath("a/" + input_x).read_text()\n',
        "trav.py",
    )
    cwe22 = [f for f in res_trav.findings if f.cwe == "CWE-22"]
    assert len(cwe22) >= 2


    # Regression 4: constant-string importlib.import_module tracks module
    # capabilities and alias bindings, plus union/AnnAssign/Div Path receivers
    src_imp = """
from importlib import import_module
from pathlib import Path

def evade(flag: bool, opt_path: Path | None):
    sp = import_module("subprocess")
    sp.Popen(["id"])
    os_mod = import_module("os")
    os_mod.system("id")
    ann_p: Path | str = Path("a") / "b"
    if flag and opt_path is not None:
        opt_path.exists()
    (Path("x") / "y").replace(ann_p)
"""
    res_imp = scan_source(src_imp, "imp.py")
    assert CAP_PROCESS_EXEC in res_imp.capabilities
    assert CAP_FS_READ in res_imp.capabilities
    assert CAP_FS_WRITE in res_imp.capabilities
    assert len([f for f in res_imp.findings if f.cwe == "CWE-78"]) >= 2


def test_cwe674_depth_guard_and_os_environ_store_and_cli_dir(tmp_path: Path) -> None:
    src = """
import os

def bounded(node, max_depth: int = 10):
    if max_depth <= 0 or node is None:
        return 0
    return 1 + bounded(node, max_depth - 1)

def assign_env():
    os.environ = {}
"""
    res = scan_source(src, "guarded.py")
    assert not any(f.cwe == "CWE-674" for f in res.findings)
    assert CAP_ENV_WRITE in res.capabilities

    pkg = tmp_path / "dir_flag_pkg"
    pkg.mkdir()
    (pkg / "mod.py").write_text("x = 1 + 2\n", encoding="utf-8")
    assert main(["--dir", str(pkg), "--strict"]) == 0


def test_cross_stack_real_music_packages_pass_strict() -> None:
    py_pkgs_dir = Path(__file__).resolve().parents[2]
    for name in ("dsp-filters", "wav-sink", "music-machine", "wave"):
        pkg_dir = py_pkgs_dir / name
        if pkg_dir.is_dir():
            rep = analyze_package(pkg_dir, strict=True)
            assert rep.passed, f"{name} failed: {rep.render_text()}"
