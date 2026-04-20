# Changelog

## 0.2.0 — 2026-04-20

### Added

- `JVMInputStream` sentinel dataclass alongside the existing `JVMPrintStream`;
  `get_static` now returns it for `java.lang.System.in` references.
- `JVMStdlibHost.invoke_virtual` handles two new dispatch paths:
  - `PrintStream.write(I)V` — emits a single raw byte as a Unicode character,
    routing it through the optional `_stdout` callback and the shared `output`
    list.  Used by the BASIC-to-JVM compiled print path.
  - `PrintStream.flush()V` — no-op (in-memory output never needs flushing).
  - `InputStream.read()I` — always returns `-1` (EOF); BASIC V1 has no INPUT.
- `JVMStdlibHost.invoke_static` — new host method for `invokestatic` dispatch.
  Handles two sub-cases:
  1. `java.util.Arrays.fill([BIIB)V` — Python-side fill of a `bytearray` slice.
  2. Same-class methods (e.g. `__ca_regGet`, `__ca_regSet`, `__ca_syscall`)
     forwarded to `JVMRuntime._run_method_with_shared_state`.
- `JVMRuntime.run_method` — new public API that runs any named static method
  (not just `main`).  Accepts either raw bytes or a pre-parsed `JVMClassFile`.
  Runs `<clinit>` automatically (using an inner `JVMSimulator`) when present,
  then runs the requested method with the same `shared` static-fields dict.
- `JVMRuntime._run_method_with_shared_state` — internal helper that spins up a
  fresh `JVMSimulator` sharing the caller's `static_fields` dict, injects
  positional arguments into local-variable slots, and returns the return value.
  This is the mechanism that lets `invokestatic` helper calls (register access,
  syscall dispatch) remain coherent with the outer execution frame.
- `JVMRuntime._class_file` attribute — stored before any nested call so that
  `JVMStdlibHost.invoke_static` can look up methods in the right class file.

### Changed

- `JVMRuntime.__init__` now sets `self.host._runtime = self` immediately after
  constructing the host, breaking the circular dependency without any lazy
  initialisation.

## 0.1.0

- add top-level JVM runtime orchestration with `System.out.println` host support
