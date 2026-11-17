# Changelog

## 0.1.0 - 2026-11-15

### Added
- Pure Python AST-based static capability analyzer (`coding_adventures_ca_capability_analyzer`).
- Canonical capability mapping for `fs:read`, `fs:write`, `net:client`, `net:server`, `process:exec`, `ffi:unsafe`, `dynamic:eval`, and `regex:re`.
- `required_capabilities.json` loader with fail-closed default deny enforcement.
- Static CWE detectors for CWE-1333 (ReDoS / `re`), CWE-674 (uncontrolled recursion), CWE-78 (command injection sinks), CWE-94 (dynamic evaluation sinks), and CWE-22 (path traversal).
- CWE-400 bounds on directory walking (`MAX_FILES`, `MAX_FILE_BYTES`) and AST traversal (`MAX_AST_NODES`).
- CLI tool `ca-capability-analyzer` supporting human-readable output, `--json`, `--strict`, and exit codes.
- Complete self-analysis verification (`fs:read` capability matching self manifest).
