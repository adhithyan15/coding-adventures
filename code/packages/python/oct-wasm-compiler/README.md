# oct-wasm-compiler

End-to-end Oct to WebAssembly compiler facade.

Pipeline:

```text
Oct source
-> oct-parser / oct-type-checker
-> oct-ir-compiler (WASM_IO: out() → SYSCALL 1, in() → SYSCALL 2)
-> ir-to-wasm-validator
-> ir-to-wasm-assembly
-> wasm-assembler / wasm-validator
```

```python
from oct_wasm_compiler import compile_source

result = compile_source("fn main() { out(0, 7); }")
assert result.binary[:4] == b"\x00asm"  # WASM magic
```
