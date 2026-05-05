# Changelog — `twig-aot`

## 0.1.1 — 2026-05-05

Real Twig source programs now compile and run on Apple Silicon — not
just hand-built IIR.  This release does NOT touch `twig-aot` itself
but pulls in upstream improvements that turn typed Twig source into
fully-resolved CIR + native code:

- `aot-core::specialise` now lowers `call_builtin "+ / - / * / / / = /
  != / < / <= / > / >= / _move"` to typed CIR ops (`add_<ty>`,
  `cmp_eq_<ty>`, `mov_<ty>`) when operand types are known, eliminating
  runtime calls for primitive arithmetic.
- `aarch64-backend` adds `mov_<ty>` lowering and fixes a stack-frame
  bug where virtual register slot 0 collided with the saved `fp/lr`
  (binaries previously SIGSEGV'd at function return).

End-to-end demonstration:

```
$ cat hello.twig
(+ 30 12)
$ twig-aot hello.twig -o hello && ./hello; echo $?
42
```

The integration test suite now runs 8 typed Twig programs through the
full pipeline and asserts their exit codes (see
`tests/macos_arm64_smoke.rs::end_to_end_typed_twig_arithmetic_and_branches`).

## 0.1.0 — 2026-05-05

Initial release.  End-to-end ahead-of-time compiler for Twig: source
file in, runnable native ARM64 Mach-O executable out.

### Pipeline

```
Twig source
   ↓ twig-ir-compiler
IIRModule
   ↓ aot-core (infer + specialise) → CIR
   ↓ aarch64-backend (compile_function) → ARM64 bytes
Vec<(fn, bytes)>
   ↓ aot-core::link → (text, offsets)
   ↓ code-packager::macho_object → MH_OBJECT
.o object file
   ↓ ld -arch arm64 -platform_version macos 15.0 15.0 -e _main -lSystem
runnable Mach-O executable
```

### Why we shell out to `ld`

On macOS 15+ (Sequoia / Tahoe) the kernel attaches a "provenance" tag
to every executable file, recording which process wrote it.  Files
written by Apple's system linker (`/usr/bin/ld`) inherit a trusted
provenance and run normally; files written by random user code are
SIGKILL'd by `AppleSystemPolicy` regardless of how well-formed the
Mach-O is.  Delegating the final link to `ld` solves that — and as a
bonus `ld` handles dyld setup, ad-hoc code signing, and SDK
versioning for us.

### CLI

Argument parsing is driven by `cli-builder` with a JSON spec
(`twig_aot.cli.json`) embedded at compile time.  `--help` and
`--version` are auto-generated.

```
twig-aot <FILE.twig> [-o <OUT>]
twig-aot --help
twig-aot --version
```

### Test coverage

- `module_with_no_entry_point_errors` — error path unit test
- `untyped_twig_returns_backend_refused` — surfaces unsupported opcodes
- `empty_main_compiles_to_object_bytes` — object-file structure
- **`end_to_end_object_through_ld_returns_42`** — real `ld` invocation,
  binary writes to disk, kernel `exec()`s it, asserts exit code 42
- **`end_to_end_typed_twig_returns_42`** — typed-IIR-via-API flow

The two E2E tests are gated to `aarch64-darwin`.

### Known limitation

The V1 ARM64 backend (PR #2156) doesn't yet lower `global_set` /
closure / property opcodes, so any Twig source that uses top-level
value defines (`(define x 5)`) or closures fails with
`AotError::BackendRefused`.  Hand-built typed IIR (function defines)
works end-to-end today.
