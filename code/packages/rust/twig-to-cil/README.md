# twig-to-cil

End-to-end Twig → CLR CIL compilation pipeline.

## What it does

`twig-to-cil` wires four upstream crates into a single function call that
compiles a Twig source string all the way to a `CILProgramArtifact` with
raw CIL bytecode per method.

```text
Twig source (str)
    │
    ▼  twig-ir-compiler  ::  compile_source
IIRModule   (all type hints = "any")
    │
    ▼  iir-type-checker  ::  infer_and_check
IIRModule   (constants and arithmetic get concrete hints: "i64", "bool", …)
    │
    ▼  iir-builtin-lowering  ::  lower_builtins
IIRModule   (call_builtin "+" → add; call_builtin "<" → lt; etc.)
    │
    ▼  iir-to-cil-bytecode  ::  lower_iir_to_cil
CILProgramArtifact  (structured, multi-method CLR artifact)
```

## Usage

```rust
use twig_to_cil::compile_twig_to_cil;

// Compile a simple expression:
let artifact = compile_twig_to_cil("(+ 1 2)", "Demo").unwrap();
assert!(!artifact.methods.is_empty());
assert!(!artifact.methods[0].body.is_empty());

// Compile a recursive function:
let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)";
let artifact = compile_twig_to_cil(src, "Factorial").unwrap();
assert!(artifact.methods.iter().any(|m| m.name == "fact"));

// Inspect specific CIL opcodes:
const RET: u8 = 0x2A;
assert!(artifact.methods[0].body.contains(&RET));
```

For a custom assembly name:

```rust
use twig_to_cil::pipeline::run_pipeline;
use iir_to_cil_bytecode::IIRClrConfig;

let config = IIRClrConfig::new("MyCompany.MyAssembly");
let artifact = run_pipeline("(+ 1 2)", "my_module", config).unwrap();
```

## Return type — `CILProgramArtifact`

The return type mirrors what the underlying `iir-to-cil-bytecode` crate
returns.  Each `CILMethodArtifact` in `artifact.methods` carries:

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Function name (e.g. `"main"`, `"fact"`) |
| `body` | `Vec<u8>` | Raw CIL bytecode bytes |
| `descriptor` | `String` | CLR method descriptor |

## Error handling

Every pipeline failure is reported through `TwigToCilError`:

| Variant | Cause |
|---------|-------|
| `Compile` | Twig lexer / parser / IIR compiler rejected the source |
| `TypeCheck` | Type-checker found fatal errors in the IIR module |
| `ClrValidation` | Pre-flight CLR validation failed |
| `ClrBackend` | CLR lowering pass returned an `IIRClrError` |

## Pipeline crates

| Crate | Role |
|-------|------|
| `twig-ir-compiler` | Twig → IIR frontend |
| `iir-type-checker` | Type inference + validation |
| `iir-builtin-lowering` | `call_builtin "+"`→`add` rewriting |
| `iir-to-cil-bytecode` | IIR → CLR CIL backend |
