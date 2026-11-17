# coding-adventures-ca-capability-analyzer

Pure Python static capability analyzer and weakness detector based on the Python Abstract Syntax Tree (`ast`). Conforms to VER00, VER08, and VER15 verified shipping standards. Zero third-party runtime dependencies.

## Features

- **Capability Extraction**: Statically discovers OS capability requirements by mapping Python imports and function calls:
  - `fs:read` / `fs:write`: `open`, `io.open`, `pathlib.Path` read/write/open operations.
  - `net:client`: `socket.connect`, `urllib.request`, `http.client`.
  - `net:server`: `socket.bind`, `socket.listen`, `socketserver`, `http.server`.
  - `process:exec`: `subprocess` functions, `os.system`, `os.popen`, `os.spawn*`, `os.exec*`.
  - `ffi:unsafe`: `ctypes` foreign function calls and library loading.
  - `dynamic:eval`: `eval`, `exec`, `compile`, `pickle.loads`, `pickle.load`.
  - `regex:re`: `re` module usage and regex compilation.
- **Fail-Closed Manifest Enforcement**: Compares extracted capabilities against `required_capabilities.json`. Unmanifested capabilities trigger a failure (`CAP001`). Packages without a manifest default to pure computation (zero OS capabilities allowed).
- **Static CWE Detectors**:
  - **CWE-1333**: ReDoS prevention — flags `re` module usage / regex compilation.
  - **CWE-674**: Uncontrolled recursion — detects direct recursion in AST call graphs.
  - **CWE-78**: OS Command Injection — flags dangerous process execution sinks (`os.system`, `subprocess.*`).
  - **CWE-94**: Code Injection — flags dynamic code evaluation sinks (`eval`, `exec`, `compile`, `pickle.loads`).
  - **CWE-22**: Path Traversal — detects directory traversal sequences (`..`) and unvalidated file open calls.
- **CWE-400 Hardening**: Strict directory and AST bounds (`MAX_FILES = 10,000`, `MAX_FILE_BYTES = 10 MiB`, `MAX_AST_NODES = 500,000`), with null-byte and path traversal rejection.
- **CLI and Library APIs**: Programmatic API (`scan_source`, `analyze_package`) and CLI entry point (`ca-capability-analyzer`).
