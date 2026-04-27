# ir-to-jvm-class-file

`ir-to-jvm-class-file` is the generic Rust JVM backend for the repo's lower
level `compiler-ir`.

It takes an `IrProgram`, lowers it into plain JVM bytecode, and packages the
result into real `.class` bytes. The output stays intentionally boring:

- static register array
- static byte memory image
- plain static helper methods
- plain `invokestatic`, branches, loads, and stores

That keeps the backend easy to reason about and friendly to both ordinary JVMs
and GraalVM Native Image.

## Output shape

The generated class looks roughly like this:

```text
final class Program {
  private static int[]  __ca_regs;
  private static byte[] __ca_memory;

  static { ... }
  public static int _start() { ... }
  public static void main(String[] args) { _start(); }
}
```

## Example

```rust
use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use ir_to_jvm_class_file::{lower_ir_to_jvm_class_file, JvmBackendConfig};

let mut program = IrProgram::new("_start");
program.add_instruction(IrInstruction::new(
    IrOp::Label,
    vec![IrOperand::Label("_start".to_string())],
    -1,
));
program.add_instruction(IrInstruction::new(
    IrOp::LoadImm,
    vec![IrOperand::Register(1), IrOperand::Immediate(0)],
    0,
));
program.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));

let artifact = lower_ir_to_jvm_class_file(
    &program,
    JvmBackendConfig::new("demo.Program"),
)?;
assert!(artifact.class_bytes.starts_with(&[0xCA, 0xFE, 0xBA, 0xBE]));
# Ok::<(), Box<dyn std::error::Error>>(())
```
