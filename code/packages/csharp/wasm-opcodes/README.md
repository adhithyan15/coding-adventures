# Wasm Opcodes

Pure C# WebAssembly 1.0 opcode metadata table.

## Verification

`BUILD` and `BUILD_windows` build the test project graph with Coverlet's
MSBuild instrumentation, then run the seven native tests from the isolated
artifacts. Coverage is restricted to `CodingAdventures.WasmOpcodes`, and the
fronts fail if production line coverage falls below 95%.
