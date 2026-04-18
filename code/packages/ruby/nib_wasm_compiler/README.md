# `coding_adventures_nib_wasm_compiler`

`coding_adventures_nib_wasm_compiler` is the Ruby end-to-end Nib-to-WASM
orchestrator. It packages the local Ruby Nib parser, type checker, IR compiler,
and the existing generic IR-to-WASM stack into a single source-to-binary step.

## Public API

- `compile_source`
- `pack_source`
- `write_wasm_file`
- `PackageResult`
- `PackageError`

## Example

```ruby
require "coding_adventures_nib_wasm_compiler"

result = CodingAdventures::NibWasmCompiler.compile_source(<<~NIB)
  fn main() -> u4 { return 7; }
NIB

File.binwrite("program.wasm", result.binary)
```
