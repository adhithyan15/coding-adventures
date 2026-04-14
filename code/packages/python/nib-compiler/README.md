# coding-adventures-nib-compiler

Compiles Nib source code all the way to an Intel HEX ROM image that can be
decoded and loaded into the Intel 4004 simulator.

## Pipeline

```text
Nib source
    -> nib-parser
    -> nib-type-checker
    -> nib-ir-compiler
    -> ir-optimizer
    -> intel-4004-ir-validator
    -> ir-to-intel-4004-compiler
    -> intel-4004-assembler
    -> intel-4004-packager
Intel HEX
```

## Usage

```python
from nib_compiler import compile_source, write_hex_file

source = """
fn main() {
    let x: u4 = 5;
}
"""

result = compile_source(source)
print(result.hex_text)

write_hex_file(source, "out/program.hex")
```

## Public API

- `NibCompiler`
- `PackageResult`
- `PackageError`
- `compile_source(source, ...)`
- `pack_source(source, ...)`
- `write_hex_file(source, path, ...)`
