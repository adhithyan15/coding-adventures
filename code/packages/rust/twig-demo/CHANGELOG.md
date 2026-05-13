# Changelog — `twig-demo`

## [0.1.0] — 2026-05-13

### Added

Initial release.  End-to-end multi-backend demonstration binary.

- Compiles and runs `(define (fib n) …) (fib 10)` through six backends:
  Interpreter (twig-vm), AOT (ARM64 native), BEAM (Erlang), WebAssembly
  (pure-Rust runtime), JVM (Java class file + `java`), CLR (CIL bytecode +
  built-in simulator).

- All six backends return **55** (`fib(10)`), confirming the shared IIR
  pipeline and all backend lowerers are consistent.

- **JVM backend** (`run_jvm`):
  - Uses the full pre-lower → `infer_and_check` → `fixup_control_flow_types`
    pipeline to resolve `"any"` types before JVM lowering.
  - Hand-generates a `TwigLauncher.class` (Java 21) with the required
    `public static void main(String[])` entry point; uses
    `invokestatic TwigFib.main()J + l2i + System.exit` so the process
    exit code equals `fib(10) % 256 = 55`.
  - `fixup_control_flow_types` has three passes:
    - Pass 0: normalise param types `"any"` → `"i64"`
    - Pass 1: build SSA env from concretely-typed dests
    - Pass 2: fix `"any"` hints on control-flow and arithmetic ops
    - Pass 3: infer `func.return_type` from `ret` instruction src

- **CLR backend** (`run_clr`):
  - Same pre-lowering pipeline as JVM.
  - Multi-method CIL simulator (`run_cil_artifact` / `exec_method`)
    handles the full CIL opcode subset emitted for integer programs
    including two-byte comparison opcodes (`0xFE 0x01` ceq etc.).
  - Fixed `entry_method` lookup to use `entry_label` (not hardcoded
    `methods[0]`) so the correct `main` function is called when the
    module contains multiple methods.

- **AOT backend** (`run_aot`):
  - Calls `twig_aot::compile_file_macos_arm64`; reads the process exit
    code as the result (`fib(10) % 256 = 55`).

- **BEAM / WASM backends**: delegate to `twig_to_beam::compile_twig_to_beam`
  and `twig_to_wasm::compile_twig_to_wasm` respectively; no fixup needed
  as those pipelines handle type normalisation internally.
