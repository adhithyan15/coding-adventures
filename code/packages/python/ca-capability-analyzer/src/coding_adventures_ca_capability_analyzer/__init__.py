"""coding_adventures_ca_capability_analyzer: AST capability scanner and CWE detector."""

from __future__ import annotations

from coding_adventures_ca_capability_analyzer.analyzer import (
    MAX_FILE_BYTES,
    MAX_FILES,
    MAX_TOTAL_BYTES,
    analyze_file,
    analyze_package,
    validate_path,
)
from coding_adventures_ca_capability_analyzer.manifest import (
    MAX_MANIFEST_SIZE_BYTES,
    load_manifest,
    normalize_capability,
)
from coding_adventures_ca_capability_analyzer.scanner import (
    MAX_AST_NODES,
    FileScanResult,
    scan_source,
)
from coding_adventures_ca_capability_analyzer.types import (
    ALL_CAPABILITIES,
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

__all__ = [
    "ALL_CAPABILITIES",
    "CAP_DYNAMIC_EVAL",
    "CAP_FFI_UNSAFE",
    "CAP_FS_READ",
    "CAP_FS_WRITE",
    "CAP_NET_CLIENT",
    "CAP_NET_SERVER",
    "CAP_PROCESS_EXEC",
    "CAP_REGEX_RE",
    "MAX_AST_NODES",
    "MAX_FILES",
    "MAX_FILE_BYTES",
    "MAX_MANIFEST_SIZE_BYTES",
    "MAX_TOTAL_BYTES",
    "AnalysisReport",
    "FileScanResult",
    "Finding",
    "Manifest",
    "analyze_file",
    "analyze_package",
    "load_manifest",
    "normalize_capability",
    "scan_source",
    "validate_path",
]
