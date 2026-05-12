# twig-to-jvm

End-to-end Twig → JVM class file compilation pipeline.

## What it does

`twig-to-jvm` wires four upstream crates into a single function call that
compiles a Twig source string all the way to a `JvmClassFile` structure
ready for inspection, simulation, or serialisation.

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
IIRModule   (call_builtin "+" → add, etc.)
    │
    ▼  iir-to-jvm-class-file  ::  lower_iir_to_jvm
JvmClassFile  (structured, multi-method JVM class)
```

## Usage

```rust
use twig_to_jvm::compile_twig_to_jvm;

// Compile a simple expression:
let class_file = compile_twig_to_jvm("(+ 1 2)", "Demo").unwrap();
assert!(!class_file.methods.is_empty());

// Compile a recursive function:
let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)";
let class_file = compile_twig_to_jvm(src, "Factorial").unwrap();
assert!(class_file.methods.iter().any(|m| m.name == "fact"));
```

For custom JVM class configuration (class name, super class, etc.):

```rust
use twig_to_jvm::pipeline::run_pipeline;
use iir_to_jvm_class_file::IIRJvmConfig;

let config = IIRJvmConfig::new("com/example/MyModule");
let class_file = run_pipeline("(+ 1 2)", "my_module", config).unwrap();
assert_eq!(class_file.this_class_name, "com/example/MyModule");
```

## Return type — `JvmClassFile`

The return type is `JvmClassFile` (not `Vec<u8>`) because the
`jvm-class-file` crate does not expose a `to_bytes`/`serialize` method on
`JvmClassFile`.  The structured form allows:

- Inspection in tests (method names, Code attributes, constant pool).
- Feeding to a JVM simulator that works with the structured form.
- Choosing the right serialisation strategy for the target use case.

## Error handling

Every pipeline failure is reported through `TwigToJvmError`:

| Variant | Cause |
|---------|-------|
| `Compile` | Twig lexer / parser / IIR compiler rejected the source |
| `TypeCheck` | Type-checker found fatal errors in the IIR module |
| `JvmValidation` | Pre-flight JVM validation failed (unsupported ops, `"any"` hints) |
| `JvmBackend` | JVM lowering pass returned an `IIRJvmError` |

## Pipeline crates

| Crate | Role |
|-------|------|
| `twig-ir-compiler` | Twig → IIR frontend |
| `iir-type-checker` | Type inference + validation |
| `iir-builtin-lowering` | `call_builtin "+"`→`add` rewriting |
| `iir-to-jvm-class-file` | IIR → JVM class file backend |
