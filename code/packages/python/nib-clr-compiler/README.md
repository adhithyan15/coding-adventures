# nib-clr-compiler

End-to-end Nib to CLR compiler facade.

Pipeline:

```text
Nib source
-> nib parser
-> nib type checker
-> nib-ir-compiler
-> ir-optimizer
-> ir-to-cil-bytecode
-> cli-assembly-writer
-> clr-vm-simulator
```

```python
from nib_clr_compiler import compile_source, run_source

source = "fn main() -> u4 { return 7; }"
compiled = compile_source(source)
result = run_source(source)

assert compiled.assembly_bytes
assert result.vm_result.return_value.value == 7
```
