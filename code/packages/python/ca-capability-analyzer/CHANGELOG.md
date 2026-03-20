# Changelog

All notable changes to `ca-capability-analyzer` will be documented in this file.

## [0.1.0] — 2026-03-19

### Added

- **Core AST analyzer** (`analyzer.py`): Walks Python ASTs to detect OS capability usage.
  - Import detection: maps `import os`, `import socket`, `import subprocess`, etc. to capability categories.
  - `open()` call detection: determines read vs write from mode argument, extracts literal file paths.
  - Attribute call detection: maps `os.listdir()`, `subprocess.run()`, `shutil.copy()`, etc. to capabilities.
  - From-import direct call detection: resolves `from os import listdir; listdir(".")`.
  - `os.environ[]` subscript detection: extracts environment variable names.
  - `analyze_file()` and `analyze_directory()` convenience functions.

- **Banned construct detector** (`banned.py`): Flags dynamic execution constructs banned outright.
  - Banned builtins: `eval()`, `exec()`, `compile()`, `__import__()`.
  - Banned module calls: `importlib.import_module()`, `pickle.loads()`, `marshal.loads()`.
  - Banned imports: `ctypes`, `cffi`.
  - Flags `getattr()` with non-literal second argument.
  - Flags `globals()` and `locals()` calls.

- **Manifest loader and comparator** (`manifest.py`): Loads `required_capabilities.json` and compares against detected capabilities.
  - Asymmetric comparison: undeclared capabilities are errors, unused declarations are warnings.
  - Glob-style target matching (fnmatch) for path patterns.
  - Default deny: no manifest = zero capabilities allowed.

- **CLI** (`cli.py`): Three commands:
  - `detect <path>`: Scan and output detected capabilities as JSON.
  - `check <path>`: Compare against manifest, exit 0 (pass) or 1 (fail).
  - `banned <path>`: Scan for banned dynamic execution constructs.

- **126 tests** across 4 test files with 95% code coverage.
