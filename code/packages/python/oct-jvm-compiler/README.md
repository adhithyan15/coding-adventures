# oct-jvm-compiler

End-to-end Oct to JVM class-file compiler facade.

Pipeline:

```text
Oct source
-> oct-parser / oct-type-checker
-> oct-ir-compiler (JVM_IO: out() → SYSCALL 1, in() → SYSCALL 4)
-> ir-to-jvm-class-file
-> jvm-class-file (structural validation)
```

```python
from oct_jvm_compiler import compile_source

result = compile_source("fn main() { out(0, 7); }")
assert result.class_bytes[:4] == b"\xca\xfe\xba\xbe"  # JVM magic
```
