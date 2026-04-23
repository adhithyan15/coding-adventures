# oct-clr-compiler

End-to-end Oct to CLR compiler facade.

Pipeline:

```text
Oct source
-> oct-parser / oct-type-checker
-> oct-ir-compiler (CLR_IO: out() → SYSCALL 1, in() → SYSCALL 2)
-> ir-to-cil-bytecode
-> cli-assembly-writer
-> clr-vm-simulator (optional)
```

```python
from oct_clr_compiler import compile_source, run_source

compiled = compile_source("fn main() { out(0, 7); }")
assert compiled.assembly_bytes

result = run_source("fn main() { out(0, 7); }")
assert result.vm_result.output == "7"
```
