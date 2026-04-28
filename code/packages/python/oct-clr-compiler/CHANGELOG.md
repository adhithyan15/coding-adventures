# Changelog

## 0.1.0

- Add end-to-end Oct to CLR compiler facade.
- Pipeline: oct-lexer → oct-parser → oct-type-checker → oct-ir-compiler (CLR_IO) → ir-to-cil-bytecode → cli-assembly-writer → clr-vm-simulator.
- `OctClrCompiler` class with `compile_source`, `write_assembly_file`, and `run_source` methods.
- Module-level convenience functions: `compile_source`, `pack_source`, `write_assembly_file`, `run_source`.
- Uses `CLR_IO` config so `out()` → SYSCALL 1 (Console.Write) and `in()` → SYSCALL 2 (Console.Read).
- `PackageError` with `stage` attribute for precise failure diagnosis.
- `ExecutionResult` dataclass wrapping `PackageResult` and `CLRVMResult`.
