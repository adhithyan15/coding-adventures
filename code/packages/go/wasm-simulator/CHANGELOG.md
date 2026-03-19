# Changelog

## [0.1.0] - Unreleased

### Added
- Created `WasmSimulator` decoupled from abstract register-machines, natively modeling operand-stacks.
- Built explicit `WasmStepTrace` allowing exact historical verification across cycle executions mapping exact Stack traces.
- `WasmDecoder` abstracts decoding variable byte-lengths (between 1 and 5 bytes depending on opcode).
- `WasmExecutor` mutates slice arrays referencing Top-Of-Stack computational parameters.
- Literate-programming implementations teaching the differences in encoding byte sizing versus static 32-bit CPU architectures.
