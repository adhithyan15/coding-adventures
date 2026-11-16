# actionscript-compiler

Deterministic ActionScript 1.0, 2.0, and 3.0 compiler targeting InterpreterIR and vm-core for Rust.

Part of the **Coding Adventures** Flash Player suite (`SWF03`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Overview

`actionscript-compiler` implements source-level compilation for ActionScript 1.0, 2.0, and 3.0 scripts into `interpreter-ir` (`LANG01`) modules executable on `vm-core` (`LANG02`):

1. **Dialects (`AsDialect`)**:
   - **AS1 (`AsDialect::As1`)**: Dynamic prototype-based ECMAScript 3 subset with Flash timeline verbs (`stop`, `play`, `gotoAndPlay`, `gotoAndStop`, `tellTarget`, `call`, `eval`, `onClipEvent`), `_root`/`_parent`/`_global`/`this` paths.
   - **AS2 (`AsDialect::As2`)**: Compile-time `class`, `interface`, `extends`, `implements`, type annotations (`var x: Number`), visibility modifiers (`public`/`private`/`static`), lowered into prototype/constructor IR.
   - **AS3 (`AsDialect::As3`)**: Modern ECMAScript 4 dialect with `package`, `import`, `namespace`, `const`, typed vectors (`Vector.<T>`), exception handling (`try`/`catch`/`finally`, `throw`), runtime type operators (`is`/`as`), and sealed class metadata.
2. **Deterministic IR Lowering**:
   - Lowers AST nodes into structured `IIRModule` / `IIRFunction` / `IIRInstr` sequences.
   - Standard opcodes: arithmetic, bitwise, comparisons, conditional jumps, array operations (`alloc_array`, `array_get`, `array_set`, `array_len`).
   - Flash timeline verbs lower to `call_builtin` instructions hooked into Flash runtime environment.
3. **VM Runner & Flash Runtime Bridge (`AsVmRunner`)**:
   - Executes compiled modules on `vm-core::VMCore`.
   - Builtin registry handlers for `flash_stop`, `flash_play`, `flash_goto_and_play`, `flash_goto_and_stop`, `flash_tell_target`, `flash_trace`, `flash_is`, `flash_as`, etc.
   - Trace capture and timeline action log verification.
4. **Security & Bounds Guarantees**:
   - Recursion depth ceiling: `MAX_AST_DEPTH = 128` (CWE-674 defense).
   - Token limit: `MAX_TOKENS = 100_000` (CWE-400 defense).
   - Instruction emission cap: `MAX_INSTRUCTIONS = 200_000` (CWE-400 defense).
   - Zero `unsafe`: `#![deny(unsafe_code)]`.
   - Zero ambient authority: `"capabilities": []` in `required_capabilities.json`.

---

## Usage

```rust
use actionscript_compiler::{compile_actionscript, run_actionscript, AsDialect};

let as1_source = r#"
    var a = 10;
    var b = 25;
    trace(a + b);
"#;

let output = run_actionscript(as1_source, AsDialect::As1)?;
assert_eq!(output, "35\n");
```
