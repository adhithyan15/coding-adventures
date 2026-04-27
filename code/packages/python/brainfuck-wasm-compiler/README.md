# brainfuck-wasm-compiler

`brainfuck-wasm-compiler` is the end-to-end pipeline package for compiling
Brainfuck source into WebAssembly bytes.

It keeps the repo's Unix-pipes structure intact by orchestrating the smaller
packages in order:

1. `brainfuck` parses source into an AST
2. `brainfuck-ir-compiler` lowers Brainfuck into generic IR
3. `ir-optimizer` optionally optimizes that IR
4. `ir-to-wasm-validator` checks the IR can lower to WASM
5. `ir-to-wasm-assembly` emits readable WASM assembly
6. `wasm-assembler` assembles the readable form into `.wasm` bytes
7. `wasm-validator` validates the final module

## Quick Start

```python
from brainfuck_wasm_compiler import compile_source
from wasm_runtime import WasiConfig, WasiHost, WasmRuntime

result = compile_source(",[.,]")
output: list[str] = []
host = WasiHost(config=WasiConfig(stdin=lambda n: b"Hi", stdout=output.append))
runtime = WasmRuntime(host=host)
runtime.load_and_run(result.binary, "_start", [])

assert "".join(output) == "Hi"
```
