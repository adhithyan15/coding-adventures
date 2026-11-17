"""Tests for package analysis, directory bounds, and CLI entrypoint."""

import json
from pathlib import Path

import pytest

from coding_adventures_ca_capability_analyzer.analyzer import (
    MAX_FILE_BYTES,
    analyze_file,
    analyze_package,
    validate_path,
)
from coding_adventures_ca_capability_analyzer.cli import main, parse_args


def test_validate_path(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="null byte"):
        validate_path("path\x00with_null")
    with pytest.raises(FileNotFoundError):
        validate_path(str(tmp_path / "non_existent"))

    p = validate_path(str(tmp_path))
    assert p.is_dir()


def test_analyze_file_single(tmp_path: Path) -> None:
    py_file = tmp_path / "single.py"
    py_file.write_text("x = 42\n", encoding="utf-8")
    rep = analyze_file(py_file)
    assert rep.passed
    assert rep.files_scanned == 1
    assert len(rep.capabilities_detected) == 0


def test_analyze_file_too_large(tmp_path: Path) -> None:
    huge_file = tmp_path / "huge.py"
    huge_file.write_bytes(b"x = 1\n" * (MAX_FILE_BYTES // 5 + 10))
    with pytest.raises(ValueError, match="MAX_FILE_BYTES"):
        analyze_file(huge_file)


def test_analyze_package_pass(tmp_path: Path) -> None:
    pkg_dir = tmp_path / "clean_pkg"
    pkg_dir.mkdir()
    (pkg_dir / "mod.py").write_text(
        "def compute(a, b): return a + b\n", encoding="utf-8"
    )

    manifest_data = {
        "version": 1,
        "package": "test/clean_pkg",
        "capabilities": [],
        "justification": "Pure math",
    }
    (pkg_dir / "required_capabilities.json").write_text(
        json.dumps(manifest_data), encoding="utf-8"
    )

    rep = analyze_package(pkg_dir)
    assert rep.passed
    assert len(rep.capabilities_detected) == 0
    assert len(rep.undeclared_capabilities) == 0


def test_analyze_package_undeclared_capability(tmp_path: Path) -> None:
    pkg_dir = tmp_path / "impure_pkg"
    pkg_dir.mkdir()
    (pkg_dir / "mod.py").write_text(
        "import os\ndef kill(): os.system('echo bye')\n", encoding="utf-8"
    )

    manifest_data = {
        "version": 1,
        "package": "test/impure_pkg",
        "capabilities": [],  # Declares pure computation!
        "justification": "Purportedly pure",
    }
    (pkg_dir / "required_capabilities.json").write_text(
        json.dumps(manifest_data), encoding="utf-8"
    )

    rep = analyze_package(pkg_dir)
    assert not rep.passed
    assert "process:exec" in rep.undeclared_capabilities


def test_analyze_package_syntax_error(tmp_path: Path) -> None:
    pkg_dir = tmp_path / "syntax_err_pkg"
    pkg_dir.mkdir()
    (pkg_dir / "bad.py").write_text("def unclosed_paren(\n", encoding="utf-8")

    rep = analyze_package(pkg_dir)
    assert not rep.passed
    assert any(f.cwe == "SYNTAX_ERROR" for f in rep.findings)


def test_analyze_package_on_single_file(tmp_path: Path) -> None:
    f = tmp_path / "single_file.py"
    f.write_text("x = 100\n", encoding="utf-8")
    rep = analyze_package(f)
    assert rep.passed
    assert rep.files_scanned == 1


def test_analyze_package_explicit_manifest(tmp_path: Path) -> None:
    pkg = tmp_path / "custom_manifest_pkg"
    pkg.mkdir()
    (pkg / "test.py").write_text("x = 1\n", encoding="utf-8")
    m_path = tmp_path / "custom_manifest.json"
    m_path.write_text(
        json.dumps(
            {
                "version": 1,
                "package": "test/pkg",
                "capabilities": [],
                "justification": "pure",
            }
        ),
        encoding="utf-8",
    )
    rep = analyze_package(pkg, manifest_path=m_path)
    assert rep.passed


def test_analyze_package_strict_mode(tmp_path: Path) -> None:
    pkg = tmp_path / "strict_pkg"
    pkg.mkdir()
    # Contains a warning (CWE-1333 regex) but no undeclared OS cap
    (pkg / "code.py").write_text("import re\n", encoding="utf-8")
    m_path = pkg / "required_capabilities.json"
    m_path.write_text(
        json.dumps(
            {
                "version": 1,
                "package": "test/strict",
                "capabilities": ["regex:re"],
                "justification": "uses regex",
            }
        ),
        encoding="utf-8",
    )
    rep_non_strict = analyze_package(pkg, strict=False)
    assert rep_non_strict.passed

    rep_strict = analyze_package(pkg, strict=True)
    assert not rep_strict.passed


def test_analyze_package_bounds_mock(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pkg = tmp_path / "mock_bounds"
    pkg.mkdir()
    (pkg / "a.py").write_text("a = 1\n", encoding="utf-8")
    (pkg / "b.py").write_text("b = 2\n", encoding="utf-8")

    import coding_adventures_ca_capability_analyzer.analyzer as az

    monkeypatch.setattr(az, "MAX_FILES", 1)
    with pytest.raises(ValueError, match="MAX_FILES"):
        analyze_package(pkg)

    monkeypatch.setattr(az, "MAX_FILES", 100)
    monkeypatch.setattr(az, "MAX_FILE_BYTES", 2)
    with pytest.raises(ValueError, match="MAX_FILE_BYTES"):
        analyze_package(pkg)

    monkeypatch.setattr(az, "MAX_FILE_BYTES", 1000)
    monkeypatch.setattr(az, "MAX_TOTAL_BYTES", 2)
    with pytest.raises(ValueError, match="MAX_TOTAL_BYTES"):
        analyze_package(pkg)


def test_main_module_invocation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    import runpy
    import sys

    pkg = tmp_path / "runpy_pkg"
    pkg.mkdir()
    (pkg / "m.py").write_text("val = 1\n", encoding="utf-8")

    monkeypatch.setattr(sys, "argv", ["ca-capability-analyzer", str(pkg)])
    with pytest.raises(SystemExit) as exc:
        runpy.run_module(
            "coding_adventures_ca_capability_analyzer", run_name="__main__"
        )
    assert exc.value.code == 0


def test_cli_parsing_and_execution(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pkg_dir = tmp_path / "cli_pkg"
    pkg_dir.mkdir()
    (pkg_dir / "test.py").write_text("x = 10\n", encoding="utf-8")

    # Parse args
    args = parse_args([str(pkg_dir), "--strict", "--json"])
    assert args.strict
    assert args.json
    assert args.target == str(pkg_dir)

    # CLI main text mode
    exit_code = main([str(pkg_dir)])
    assert exit_code == 0

    # CLI main json mode
    exit_code_json = main([str(pkg_dir), "--json"])
    assert exit_code_json == 0

    # CLI non-existent path
    exit_code_err = main([str(tmp_path / "does_not_exist")])
    assert exit_code_err == 2


def test_validate_path_security_checks(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from coding_adventures_ca_capability_analyzer.analyzer import validate_path

    # Embedded null byte
    with pytest.raises(ValueError, match="null byte"):
        validate_path("path\x00with_null")

    # Symlink rejection
    f = tmp_path / "regular_file.txt"
    f.write_text("hello", encoding="utf-8")
    monkeypatch.setattr(Path, "is_symlink", lambda self: self == f)
    with pytest.raises(ValueError, match="Symlinks are not permitted"):
        validate_path(str(f))

    # Base directory escape
    base = tmp_path / "base_dir"
    base.mkdir()
    outside = tmp_path / "outside_dir" / "file.txt"
    outside.parent.mkdir()
    outside.write_text("outside", encoding="utf-8")
    with pytest.raises(ValueError, match="escapes base directory"):
        validate_path(str(outside), base_dir=base)


def test_analyze_file_and_package_cwe400_and_symlinks(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    import coding_adventures_ca_capability_analyzer.analyzer as az

    # File > MAX_FILE_BYTES in analyze_file
    f = tmp_path / "big_file.py"
    f.write_bytes(b"x = 1234567890\n")
    monkeypatch.setattr(az, "MAX_FILE_BYTES", 5)
    with pytest.raises(ValueError, match="MAX_FILE_BYTES"):
        az.analyze_file(f)

    # File > MAX_FILE_BYTES via content read check
    monkeypatch.setattr(az, "MAX_FILE_BYTES", 10)
    with pytest.raises(ValueError, match="MAX_FILE_BYTES"):
        az.analyze_file(f)

    monkeypatch.setattr(az, "MAX_FILE_BYTES", 10 * 1024 * 1024)

    # analyze_file catching RecursionError
    def raise_rec(*args: object, **kwargs: object) -> None:
        raise RecursionError("analyzer recursion")

    monkeypatch.setattr(az, "scan_source", raise_rec)
    rep_rec = az.analyze_file(f)
    assert not rep_rec.passed
    assert rep_rec.findings[0].cwe == "CWE-400"
    assert "RecursionError" in rep_rec.findings[0].message

    # analyze_package catching RecursionError
    pkg = tmp_path / "rec_pkg"
    pkg.mkdir()
    (pkg / "m.py").write_text("x = 1\n", encoding="utf-8")
    rep_pkg_rec = az.analyze_package(pkg)
    assert not rep_pkg_rec.passed
    assert any(f.cwe == "CWE-400" for f in rep_pkg_rec.findings)

    # Symlink manifest in analyze_package
    cand = pkg / "required_capabilities.json"
    cand.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(Path, "is_symlink", lambda self: self == cand)
    with pytest.raises(ValueError, match="Manifest cannot be a symlink"):
        az.analyze_package(pkg)

    # Symlink src directory in analyze_package
    src_dir = pkg / "src"
    src_dir.mkdir()
    monkeypatch.setattr(Path, "is_symlink", lambda self: self == src_dir)
    with pytest.raises(ValueError, match="Source directory cannot be a symlink"):
        az.analyze_package(pkg)

    # Undo is_symlink monkeypatch
    monkeypatch.setattr(Path, "is_symlink", lambda self: False)

    # src directory escaping package root
    outside_src = tmp_path / "outside_src"
    outside_src.mkdir()
    orig_resolve = Path.resolve

    def mock_resolve(self: Path, strict: bool = True) -> Path:
        if self == src_dir:
            return outside_src
        return orig_resolve(self, strict=strict)

    monkeypatch.setattr(Path, "resolve", mock_resolve)
    with pytest.raises(ValueError, match="escapes package root"):
        az.analyze_package(pkg)

    # Undo resolve monkeypatch
    monkeypatch.setattr(Path, "resolve", orig_resolve)

    # Symlink .py file skipped in package traversal
    py_sym_pkg = tmp_path / "py_sym_pkg"
    py_sym_pkg.mkdir()
    sym_py = py_sym_pkg / "sym.py"
    sym_py.write_text("x = 1\n", encoding="utf-8")
    monkeypatch.setattr(Path, "is_symlink", lambda self: self == sym_py)
    rep_sym = az.analyze_package(py_sym_pkg)
    assert rep_sym.files_scanned == 0
    monkeypatch.setattr(Path, "is_symlink", lambda self: False)

    # .py file escaping root recorded as CWE-22 finding
    escape_pkg = tmp_path / "escape_pkg"
    escape_pkg.mkdir()
    esc_file = escape_pkg / "esc.py"
    esc_file.write_text("x = 1\n", encoding="utf-8")
    outside_file = tmp_path / "outside_py.py"
    outside_file.write_text("x = 1\n", encoding="utf-8")

    def mock_esc_resolve(self: Path, strict: bool = True) -> Path:
        if self == esc_file:
            return outside_file
        return orig_resolve(self, strict=strict)

    monkeypatch.setattr(Path, "resolve", mock_esc_resolve)
    rep_esc = az.analyze_package(escape_pkg)
    assert any(f.cwe == "CWE-22" for f in rep_esc.findings)
    monkeypatch.setattr(Path, "resolve", orig_resolve)

    # File > MAX_FILE_BYTES during content read in package
    big_pkg = tmp_path / "big_pkg"
    big_pkg.mkdir()
    (big_pkg / "big.py").write_text("x = 1234567890\n", encoding="utf-8")
    monkeypatch.setattr(az, "MAX_FILE_BYTES", 5)
    # stat size passes check but read exceeds
    orig_stat = Path.stat

    class MockStat:
        st_size = 4

    monkeypatch.setattr(
        Path,
        "stat",
        lambda self, **kw: MockStat() if self.name == "big.py" else orig_stat(self),
    )
    with pytest.raises(ValueError, match="MAX_FILE_BYTES"):
        az.analyze_package(big_pkg)


def test_cli_entry_point_direct(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    import runpy
    import sys

    pkg = tmp_path / "cli_run_pkg"
    pkg.mkdir()
    (pkg / "mod.py").write_text("x = 1\n", encoding="utf-8")
    monkeypatch.setattr(sys, "argv", ["cli.py", str(pkg)])
    with pytest.raises(SystemExit) as exc:
        runpy.run_module(
            "coding_adventures_ca_capability_analyzer.cli", run_name="__main__"
        )
    assert exc.value.code == 0
