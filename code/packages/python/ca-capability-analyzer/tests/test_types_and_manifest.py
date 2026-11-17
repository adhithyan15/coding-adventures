"""Tests for types and manifest loader."""

import json
from pathlib import Path

import pytest

from coding_adventures_ca_capability_analyzer.manifest import (
    MAX_MANIFEST_SIZE_BYTES,
    load_manifest,
    normalize_capability,
)
from coding_adventures_ca_capability_analyzer.types import (
    CAP_DYNAMIC_EVAL,
    CAP_FFI_UNSAFE,
    CAP_FS_READ,
    CAP_FS_WRITE,
    CAP_NET_CLIENT,
    CAP_NET_SERVER,
    CAP_PROCESS_EXEC,
    CAP_REGEX_RE,
    AnalysisReport,
    Finding,
    Manifest,
)


def test_finding_and_manifest_structures() -> None:
    f = Finding(
        file_path="pkg/mod.py",
        line=10,
        col=4,
        cwe="CWE-78",
        message="Command sink",
        capability=CAP_PROCESS_EXEC,
        severity="error",
    )
    d = f.to_dict()
    assert d["cwe"] == "CWE-78"
    assert d["capability"] == CAP_PROCESS_EXEC

    f_no_cap = Finding(
        file_path="pkg/mod.py",
        line=1,
        col=0,
        cwe="CWE-674",
        message="Recursion",
    )
    assert "capability" not in f_no_cap.to_dict()

    m = Manifest(
        package="test/pkg",
        version=1,
        capabilities=frozenset({CAP_FS_READ}),
        justification="Reads files",
        is_explicit=True,
    )
    assert m.covers(CAP_FS_READ)
    assert not m.covers(CAP_FS_WRITE)


def test_analysis_report_rendering() -> None:
    rep = AnalysisReport(
        package_path="pkg",
        files_scanned=2,
        capabilities_detected=frozenset({CAP_FS_READ}),
        capabilities_declared=frozenset({CAP_FS_READ}),
        undeclared_capabilities=frozenset(),
        findings=(),
        passed=True,
    )
    d = rep.to_dict()
    assert d["passed"] is True
    assert rep.render_text().startswith("=== Capability Analysis Report")

    # With findings and undeclared capabilities
    rep_fail = AnalysisReport(
        package_path="pkg",
        files_scanned=2,
        capabilities_detected=frozenset({CAP_PROCESS_EXEC}),
        capabilities_declared=frozenset(),
        undeclared_capabilities=frozenset({CAP_PROCESS_EXEC}),
        findings=(
            Finding("a.py", 1, 0, "CWE-78", "Sink", capability=CAP_PROCESS_EXEC),
        ),
        passed=False,
    )
    text = rep_fail.render_text()
    assert "UNDECLARED CAPABILITIES" in text
    assert "Verdict: FAIL" in text


def test_normalize_capability_strings() -> None:
    assert normalize_capability("fs:read") == CAP_FS_READ
    assert normalize_capability("fs:read:*") == CAP_FS_READ
    assert normalize_capability("proc:exec") == CAP_PROCESS_EXEC
    assert normalize_capability("process:exec:*") == CAP_PROCESS_EXEC
    assert normalize_capability(12345) is None


def test_normalize_capability_dicts() -> None:
    assert normalize_capability({"category": "fs", "action": "read"}) == CAP_FS_READ
    assert normalize_capability({"category": "fs", "action": "list"}) == CAP_FS_READ
    assert normalize_capability({"category": "fs", "action": "write"}) == CAP_FS_WRITE
    assert normalize_capability({"category": "fs", "action": "delete"}) == CAP_FS_WRITE
    assert (
        normalize_capability({"category": "net", "action": "connect"}) == CAP_NET_CLIENT
    )
    assert normalize_capability({"category": "net", "action": "dns"}) == CAP_NET_CLIENT
    assert (
        normalize_capability({"category": "net", "action": "listen"}) == CAP_NET_SERVER
    )
    assert (
        normalize_capability({"category": "proc", "action": "exec"}) == CAP_PROCESS_EXEC
    )
    assert normalize_capability({"category": "ffi", "action": "call"}) == CAP_FFI_UNSAFE
    assert (
        normalize_capability({"category": "dynamic", "action": "eval"})
        == CAP_DYNAMIC_EVAL
    )
    assert normalize_capability({"category": "regex", "action": "re"}) == CAP_REGEX_RE
    assert normalize_capability({"category": "custom", "action": "act"}) == "custom:act"


def test_load_manifest_non_existent(tmp_path: Path) -> None:
    m = load_manifest(tmp_path / "does_not_exist.json")
    assert not m.is_explicit
    assert len(m.capabilities) == 0


def test_load_manifest_valid(tmp_path: Path) -> None:
    data = {
        "version": 1,
        "package": "test/pkg",
        "capabilities": [
            {"category": "fs", "action": "read"},
            "net:client:*",
        ],
        "justification": "Valid manifest",
    }
    p = tmp_path / "required_capabilities.json"
    p.write_text(json.dumps(data), encoding="utf-8")

    m = load_manifest(p)
    assert m.is_explicit
    assert m.covers(CAP_FS_READ)
    assert m.covers(CAP_NET_CLIENT)


def test_load_manifest_errors(tmp_path: Path) -> None:
    p = tmp_path / "bad.json"

    # Non-dict
    p.write_text("[]", encoding="utf-8")
    with pytest.raises(ValueError, match="JSON object"):
        load_manifest(p)

    # Bad capabilities field
    p.write_text(json.dumps({"capabilities": "not-a-list"}), encoding="utf-8")
    with pytest.raises(ValueError, match="must be a list"):
        load_manifest(p)

    # Size limit
    p_huge = tmp_path / "huge.json"
    p_huge.write_bytes(b"x" * (MAX_MANIFEST_SIZE_BYTES + 10))
    with pytest.raises(ValueError, match="exceeds limit"):
        load_manifest(p_huge)


def test_load_manifest_version_and_symlink_checks(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    p = tmp_path / "test_manifest.json"

    # Version as boolean
    p.write_text(json.dumps({"version": True}), encoding="utf-8")
    with pytest.raises(ValueError, match="must be an integer"):
        load_manifest(p)

    # Version < 1
    p.write_text(json.dumps({"version": 0}), encoding="utf-8")
    with pytest.raises(ValueError, match="must be >= 1"):
        load_manifest(p)

    # Valid manifest
    p.write_text(json.dumps({"version": 1, "capabilities": []}), encoding="utf-8")

    # Symlink manifest rejection
    monkeypatch.setattr(Path, "is_symlink", lambda self: self == p)
    with pytest.raises(ValueError, match="cannot be a symlink"):
        load_manifest(p)

    # Undo monkeypatch before testing root escape
    monkeypatch.undo()

    # Escape expected root
    outside_root = tmp_path / "other_dir"
    outside_root.mkdir()
    with pytest.raises(ValueError, match="escapes expected root"):
        load_manifest(p, expected_root=outside_root)
