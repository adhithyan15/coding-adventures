"""Package directory analyzer and manifest enforcement engine (VER08/VER15).

Pure Python with zero external dependencies.
"""

from __future__ import annotations

import os
from pathlib import Path

from coding_adventures_ca_capability_analyzer.manifest import load_manifest
from coding_adventures_ca_capability_analyzer.scanner import scan_source
from coding_adventures_ca_capability_analyzer.types import (
    MAX_FILE_BYTES,
    MAX_FILES,
    MAX_TOTAL_BYTES,
    AnalysisReport,
    Finding,
)

SKIP_DIR_NAMES = frozenset(
    {
        ".git",
        ".venv",
        "venv",
        "__pycache__",
        ".pytest_cache",
        ".ruff_cache",
        ".mypy_cache",
        "build",
        "dist",
        ".tox",
        "tests",
        "benches",
        "examples",
    }
)


def validate_path(path_str: str, base_dir: Path | None = None) -> Path:
    """Validate path string against null bytes, symlinks, and normalize."""
    if "\x00" in path_str:
        raise ValueError("Path contains embedded null byte")
    p = Path(path_str)
    if p.is_symlink():
        raise ValueError(f"Symlinks are not permitted: {path_str}")
    if not p.exists():
        raise FileNotFoundError(f"Target path does not exist: {path_str}")
    resolved = p.resolve(strict=True)
    if base_dir is not None:
        base_resolved = base_dir.resolve(strict=True)
        if not resolved.is_relative_to(base_resolved):
            raise ValueError(f"Path '{path_str}' escapes base directory '{base_dir}'")
    return resolved


def analyze_file(file_path: Path | str) -> AnalysisReport:
    """Analyze a single Python file."""
    p = validate_path(str(file_path))
    size = p.stat().st_size
    if size > MAX_FILE_BYTES:
        raise ValueError(
            f"File '{p}' exceeds MAX_FILE_BYTES ({size} > {MAX_FILE_BYTES})"
        )

    with p.open("rb") as f:
        raw = f.read(MAX_FILE_BYTES + 1)
    if len(raw) > MAX_FILE_BYTES:
        raise ValueError(
            f"File '{p}' exceeds MAX_FILE_BYTES ({len(raw)} > {MAX_FILE_BYTES})"
        )
    content = raw.decode("utf-8", errors="replace")

    try:
        res = scan_source(content, filename=str(p))
    except (RecursionError, MemoryError) as err:
        msg = f"Resource exhaustion during file analysis: {type(err).__name__}: {err}"
        finding = Finding(
            file_path=str(p),
            line=1,
            col=0,
            cwe="CWE-400",
            message=msg,
            severity="error",
        )
        return AnalysisReport(
            package_path=str(p),
            files_scanned=1,
            capabilities_detected=frozenset(),
            capabilities_declared=frozenset(),
            undeclared_capabilities=frozenset(),
            findings=(finding,),
            passed=False,
        )

    return AnalysisReport(
        package_path=str(p),
        files_scanned=1,
        capabilities_detected=res.capabilities,
        capabilities_declared=frozenset(),
        undeclared_capabilities=res.capabilities,
        findings=res.findings,
        passed=len(res.capabilities) == 0
        and not any(f.severity == "error" for f in res.findings),
    )


def analyze_package(
    package_dir: Path | str,
    manifest_path: Path | str | None = None,
    strict: bool = False,
) -> AnalysisReport:
    """Analyze a Python package directory against required_capabilities.json.

    Enforces CWE-400 bounds on file count and total size, and fails closed
    on undeclared capabilities.
    """
    root = validate_path(str(package_dir))

    if root.is_file():
        return analyze_file(root)

    # Manifest resolution
    if manifest_path is not None:
        m_path = validate_path(str(manifest_path))
        manifest = load_manifest(m_path)
    else:
        candidate = root / "required_capabilities.json"
        if candidate.is_symlink():
            raise ValueError(f"Manifest cannot be a symlink: {candidate}")
        manifest = load_manifest(candidate, expected_root=root)

    all_capabilities: set[str] = set()
    all_findings: list[Finding] = []
    files_scanned = 0
    total_bytes = 0

    # If the package has a src/ directory, scan that; otherwise scan package root
    src_dir = root / "src"
    if src_dir.is_dir():
        if src_dir.is_symlink():
            raise ValueError(f"Source directory cannot be a symlink: {src_dir}")
        if not src_dir.resolve(strict=True).is_relative_to(root.resolve(strict=True)):
            raise ValueError(f"Source directory escapes package root: {src_dir}")
        scan_root = src_dir
    else:
        scan_root = root

    # Iterative directory traversal avoiding deep recursion
    for dirpath, dirnames, filenames in os.walk(str(scan_root)):
        # Filter out skipped directories and symlinks in-place
        dirnames[:] = [
            d
            for d in dirnames
            if d not in SKIP_DIR_NAMES
            and not d.endswith(".egg-info")
            and not (Path(dirpath) / d).is_symlink()
        ]

        for fname in sorted(filenames):
            if not fname.endswith(".py"):
                continue

            fpath = Path(dirpath) / fname
            if fpath.is_symlink():
                continue

            resolved_fpath = fpath.resolve(strict=True)
            if not resolved_fpath.is_relative_to(root.resolve(strict=True)):
                all_findings.append(
                    Finding(
                        file_path=str(fpath.relative_to(root)),
                        line=1,
                        col=0,
                        cwe="CWE-22",
                        message=f"File escapes package root: {fpath}",
                        severity="error",
                    )
                )
                continue

            files_scanned += 1
            if files_scanned > MAX_FILES:
                raise ValueError(f"Package exceeds MAX_FILES bound ({MAX_FILES})")

            size = resolved_fpath.stat().st_size
            if size > MAX_FILE_BYTES:
                raise ValueError(
                    f"File '{fpath}' exceeds MAX_FILE_BYTES ({size} > {MAX_FILE_BYTES})"
                )
            total_bytes += size
            if total_bytes > MAX_TOTAL_BYTES:
                raise ValueError(
                    f"Package exceeds MAX_TOTAL_BYTES bound ({MAX_TOTAL_BYTES})"
                )

            with resolved_fpath.open("rb") as f:
                raw = f.read(MAX_FILE_BYTES + 1)
            if len(raw) > MAX_FILE_BYTES:
                msg = (
                    f"File '{fpath}' exceeds MAX_FILE_BYTES "
                    f"({len(raw)} > {MAX_FILE_BYTES})"
                )
                raise ValueError(msg)
            content = raw.decode("utf-8", errors="replace")

            try:
                res = scan_source(content, filename=str(fpath.relative_to(root)))
                all_capabilities.update(res.capabilities)
                all_findings.extend(res.findings)
            except SyntaxError as syn_err:
                all_findings.append(
                    Finding(
                        file_path=str(fpath.relative_to(root)),
                        line=syn_err.lineno or 1,
                        col=syn_err.offset or 0,
                        cwe="SYNTAX_ERROR",
                        message=f"Python syntax error: {syn_err.msg}",
                        severity="error",
                    )
                )
            except (RecursionError, MemoryError) as err:
                msg = (
                    "Resource exhaustion during AST analysis: "
                    f"{type(err).__name__}: {err}"
                )
                all_findings.append(
                    Finding(
                        file_path=str(fpath.relative_to(root)),
                        line=1,
                        col=0,
                        cwe="CWE-400",
                        message=msg,
                        severity="error",
                    )
                )

    # Compare capabilities against manifest
    undeclared = set()
    for cap in sorted(all_capabilities):
        if not manifest.covers(cap):
            undeclared.add(cap)
            all_findings.append(
                Finding(
                    file_path=str(root / "required_capabilities.json"),
                    line=1,
                    col=0,
                    cwe="CAP001",
                    capability=cap,
                    message=f"Undeclared capability detected: '{cap}'",
                    severity="error",
                )
            )

    has_errors = any(f.severity == "error" for f in all_findings)
    has_warnings = any(f.severity == "warning" for f in all_findings)

    if strict:
        passed = (len(undeclared) == 0) and not has_errors and not has_warnings
    else:
        passed = (len(undeclared) == 0) and not has_errors

    return AnalysisReport(
        package_path=str(root),
        files_scanned=files_scanned,
        capabilities_detected=frozenset(all_capabilities),
        capabilities_declared=manifest.capabilities,
        undeclared_capabilities=frozenset(undeclared),
        findings=tuple(all_findings),
        passed=passed,
    )
