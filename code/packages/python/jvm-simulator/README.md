# JVM Simulator

Execution engine for disassembled JVM bytecode.

This package intentionally does not parse `.class` files or JARs directly.
Those responsibilities live in sibling packages so the JVM stack stays loosely
coupled and reusable:

- `jvm-class-file` decodes versioned class files
- `jvm-jar-decoder` decodes JAR containers
- `jvm-bytecode-disassembler` owns opcode tables, version metadata, and the
  disassembled instruction representation
- `jvm-simulator` executes the disassembled method body

That split is meant to let later work, like verification, class loading, or a
JIT-backed real JVM, reuse the same lower layers instead of depending on one
monolithic simulator package.

## Usage

```python
from jvm_bytecode_disassembler import JVMOpcode, assemble_jvm, disassemble_method_body
from jvm_simulator import JVMSimulator

method = disassemble_method_body(
    assemble_jvm(
        (JVMOpcode.ICONST_1,),
        (JVMOpcode.ICONST_2,),
        (JVMOpcode.IADD,),
        (JVMOpcode.IRETURN,),
    ),
    max_stack=2,
    max_locals=0,
)

sim = JVMSimulator()
sim.load_method(method)
traces = sim.run()
print(sim.return_value)  # 3
```
