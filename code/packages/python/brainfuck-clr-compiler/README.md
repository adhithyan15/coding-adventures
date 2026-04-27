# brainfuck-clr-compiler

End-to-end Brainfuck to CLR compiler facade.

Pipeline:

```text
Brainfuck source
-> brainfuck parser
-> brainfuck-ir-compiler
-> ir-optimizer
-> ir-to-cil-bytecode
-> cli-assembly-writer
-> clr-vm-simulator
```

```python
from brainfuck_clr_compiler import compile_source, run_source

compiled = compile_source("+" * 65 + ".")
result = run_source("+" * 65 + ".")

assert compiled.assembly_bytes
assert result.vm_result.output == "A"
```
