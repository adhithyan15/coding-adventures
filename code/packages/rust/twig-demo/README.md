# `twig-demo`

End-to-end demonstration binary that compiles and executes the same Twig
program through **six distinct execution backends**, printing a result table.

## Backends

| # | Backend | Crate | Notes |
|---|---------|-------|-------|
| 1 | **Interpreter** | `twig-vm` | Tree-walking interpreter |
| 2 | **AOT (ARM64)** | `twig-aot` | Mach-O native executable via `ld` |
| 3 | **BEAM** | `twig-to-beam` | Erlang bytecode, runs via `erl` |
| 4 | **WebAssembly** | `twig-to-wasm` | WASM binary, runs in pure-Rust runtime |
| 5 | **JVM** | `twig-to-jvm` | `.class` file, runs via `java` |
| 6 | **CLR (.NET)** | `twig-to-cil` | CIL bytecode, multi-method simulator |

## Program

```scheme
(define (fib n)
  (if (< n 2)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))
(fib 10)
```

Expected result: **55**  (`fib(10)` = 55th Fibonacci number at zero-based index 10).

This program exercises: recursion, conditional branching, integer arithmetic
— the core of any VM.

## Usage

```bash
cargo run -p twig-demo
```

Expected output:

```
══════════════════════════════════════════════════════════════
   🐦 Twig Multi-Backend Demo
══════════════════════════════════════════════════════════════

Program:  (define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)
Expected: 55  (fib(10) = Fibonacci number at index 10)

──────────────────────────────────────────────────────────────
Backend                           Time(ms)        Result  Status
──────────────────────────────────────────────────────────────
Interpreter (twig-vm)                    6            55  ✅ PASS
AOT (ARM64 native)                     224            55  ✅ PASS
BEAM (Erlang VM)                      1153            55  ✅ PASS
WebAssembly (Rust runtime)               7            55  ✅ PASS
JVM (Java 21)                           86            55  ✅ PASS
CLR (.NET 9)                             2            55  ✅ PASS
──────────────────────────────────────────────────────────────

🎉 All 6 backends returned 55. Twig runs everywhere!
```

## Runtime requirements

| Backend | Requirement |
|---------|-------------|
| AOT (ARM64) | `ld` (Xcode Command Line Tools) |
| BEAM | `erl` (Erlang/OTP) |
| WASM | none — pure-Rust runtime |
| JVM | `java` (JDK 5+) |
| CLR | none — built-in multi-method simulator |

## Pipeline

For BEAM, WASM, JVM, and CLR the compilation chain is:

```text
Twig source
  → twig-ir-compiler      (IIRModule, all types "any")
  → pre_lower_builtins    (call_builtin "+" → add, etc.)
  → iir-type-checker      (concrete types: "i64", "bool", …)
  → fixup_control_flow    (ret/call/label get concrete types)
  → backend lowering      (BEAM / WASM / JVM / CIL bytes)
```

For AOT (ARM64):

```text
Twig source
  → twig-ir-compiler      (IIRModule, all types "any")
  → prepare_module_for_aot (pre-lower + normalize params to u64 + propagate)
  → aot_specialise         (IIR → typed CIR)
  → aarch64-backend        (CIR → ARM64 bytes)
  → two-pass linker        (patch cross-function BL relocations)
  → code-packager          (Mach-O MH_OBJECT)
  → ld                     (link to runnable executable)
```

## In the stack

`twig-demo` is the integration canary for the entire Twig VM stack.  If
all six backends produce the same result, the shared IIR and all lowering
pipelines are consistent.  It is not a library — nothing imports it.
