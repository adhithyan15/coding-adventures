### Enforced .NET WasmOpcodes coverage gates

The C# and F# WasmOpcodes build fronts now produce real production-assembly
coverage reports from clean isolated artifacts and fail below the repository's
95% library line-coverage target. Both test projects carry the matching
Coverlet MSBuild provider; opcode APIs and data remain unchanged.
