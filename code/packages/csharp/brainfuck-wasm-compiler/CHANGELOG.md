# Changelog

## 0.1.0

- Add a pure C# Brainfuck parser and WebAssembly emitter.
- Emit optional WASI input/output imports, linear memory, and `_start` exports.
- Validate source size, loop balance, and loop nesting depth.
- Add compile, pack, file-write, module-shape, runtime, and error-path tests.
