# Changelog

## Unreleased - Cross-stack Music/Audio/DSP capability audit (VER-W9-D)

- Expanded Spec 13 capability detection and normalization in `types.py`, `manifest.py`, and `scanner.py`: added `env:read`, `env:write`, `time:read`, `time:sleep`, `rand:read`, and `thread:spawn`, plus `cffi`, `marshal`, `yaml.unsafe_load`/`yaml.full_load`, `__import__`, dynamic `importlib.import_module`, and `os.*`/`shutil.*`/`Path.*` operations.
- Hardened `os.open` flag inference (`os.O_RDONLY | getattr(os, ...)`), `Path` vs `str.replace` receiver disambiguation, segment-based `CWE-22` `..` traversal checks, depth-guarded `CWE-674` recursion exemption, duplicate JSON key rejection in `load_manifest`, and `--dir` CLI parity in `cli.py`.
- Added `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (SplitMix64 PBT + cross-stack regressions).

## 0.1.0 - 2026-11-15

### Added
- Pure Python AST-based static capability analyzer (`coding_adventures_ca_capability_analyzer`).
- Canonical capability mapping for `fs:read`, `fs:write`, `net:client`, `net:server`, `process:exec`, `ffi:unsafe`, `dynamic:eval`, and `regex:re`.
- `required_capabilities.json` loader with fail-closed default deny enforcement.
- Static CWE detectors for CWE-1333 (ReDoS / `re`), CWE-674 (uncontrolled recursion), CWE-78 (command injection sinks), CWE-94 (dynamic evaluation sinks), and CWE-22 (path traversal).
- CWE-400 bounds on directory walking (`MAX_FILES`, `MAX_FILE_BYTES`) and AST traversal (`MAX_AST_NODES`).
- CLI tool `ca-capability-analyzer` supporting human-readable output, `--json`, `--strict`, and exit codes.
- Complete self-analysis verification (`fs:read` capability matching self manifest).
