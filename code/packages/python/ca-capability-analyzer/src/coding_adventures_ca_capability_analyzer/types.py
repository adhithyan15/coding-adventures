"""Data structures and capability constants for ca-capability-analyzer (VER08/VER15).

Pure Python with zero external dependencies.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any

# Canonical Capabilities
CAP_FS_READ = "fs:read"
CAP_FS_WRITE = "fs:write"
CAP_NET_CLIENT = "net:client"
CAP_NET_SERVER = "net:server"
CAP_PROCESS_EXEC = "process:exec"
CAP_FFI_UNSAFE = "ffi:unsafe"
CAP_DYNAMIC_EVAL = "dynamic:eval"
CAP_REGEX_RE = "regex:re"
CAP_ENV_READ = "env:read"
CAP_ENV_WRITE = "env:write"
CAP_TIME_READ = "time:read"
CAP_TIME_SLEEP = "time:sleep"
CAP_RAND_READ = "rand:read"
CAP_THREAD_SPAWN = "thread:spawn"

# CWE-400 Bounds
MAX_FILES = 10_000
MAX_FILE_BYTES = 10 * 1024 * 1024  # 10 MiB per file
MAX_TOTAL_BYTES = 100 * 1024 * 1024  # 100 MiB total
MAX_AST_NODES = 500_000

ALL_CAPABILITIES: frozenset[str] = frozenset(
    {
        CAP_FS_READ,
        CAP_FS_WRITE,
        CAP_NET_CLIENT,
        CAP_NET_SERVER,
        CAP_PROCESS_EXEC,
        CAP_FFI_UNSAFE,
        CAP_DYNAMIC_EVAL,
        CAP_REGEX_RE,
        CAP_ENV_READ,
        CAP_ENV_WRITE,
        CAP_TIME_READ,
        CAP_TIME_SLEEP,
        CAP_RAND_READ,
        CAP_THREAD_SPAWN,
    }
)


@dataclass(frozen=True)
class Finding:
    """Security or capability audit finding."""

    file_path: str
    line: int
    col: int
    cwe: str  # e.g. "CWE-1333", "CWE-674", "CWE-78", "CWE-94", "CWE-22", "CAP001"
    message: str
    capability: str | None = None
    severity: str = "error"  # "error" | "warning"

    def to_dict(self) -> dict[str, Any]:
        """Convert finding to dictionary."""
        d = asdict(self)
        if self.capability is None:
            del d["capability"]
        return d


@dataclass(frozen=True)
class Manifest:
    """Validated required_capabilities.json manifest."""

    package: str
    version: int
    capabilities: frozenset[str]
    justification: str
    is_explicit: bool

    def covers(self, cap: str) -> bool:
        """Check if capability is covered by declared set."""
        return cap in self.capabilities


@dataclass(frozen=True)
class AnalysisReport:
    """Aggregated analysis report for a package or file."""

    package_path: str
    files_scanned: int
    capabilities_detected: frozenset[str]
    capabilities_declared: frozenset[str]
    undeclared_capabilities: frozenset[str]
    findings: tuple[Finding, ...]
    passed: bool

    def to_dict(self) -> dict[str, Any]:
        """Convert report to JSON-serializable dictionary."""
        return {
            "package_path": self.package_path,
            "files_scanned": self.files_scanned,
            "capabilities_detected": sorted(self.capabilities_detected),
            "capabilities_declared": sorted(self.capabilities_declared),
            "undeclared_capabilities": sorted(self.undeclared_capabilities),
            "findings": [f.to_dict() for f in self.findings],
            "passed": self.passed,
        }

    def render_text(self) -> str:
        """Render a human-readable text summary."""
        det_str = ", ".join(sorted(self.capabilities_detected)) or "none (pure)"
        dec_str = ", ".join(sorted(self.capabilities_declared)) or "none (pure)"
        lines = [
            f"=== Capability Analysis Report: {self.package_path} ===",
            f"Files Scanned: {self.files_scanned}",
            f"Capabilities Detected: {det_str}",
            f"Capabilities Declared: {dec_str}",
        ]
        if self.undeclared_capabilities:
            und_str = ", ".join(sorted(self.undeclared_capabilities))
            lines.append(f"UNDECLARED CAPABILITIES: {und_str}")
        if self.findings:
            lines.append(f"Findings ({len(self.findings)}):")
            for f in self.findings:
                cap_info = f" [{f.capability}]" if f.capability else ""
                loc = f"{f.file_path}:{f.line}:{f.col}"
                sev = f.severity.upper()
                lines.append(f"  {loc} [{sev}] {f.cwe}{cap_info}: {f.message}")
        lines.append(f"Verdict: {'PASS' if self.passed else 'FAIL'}")
        return "\n".join(lines)
