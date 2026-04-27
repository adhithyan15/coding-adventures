# Changelog

## 0.1.0

- Add end-to-end Oct to JVM class-file compiler facade.
- Pipeline: oct-lexer → oct-parser → oct-type-checker → oct-ir-compiler (JVM_IO) → ir-to-jvm-class-file → jvm-class-file.
- `OctJvmCompiler` class with `compile_source` and `write_class_file` methods.
- Module-level convenience functions: `compile_source`, `pack_source`, `write_class_file`.
- Uses `JVM_IO` config so `out()` → SYSCALL 1 (System.out.write) and `in()` → SYSCALL 4 (System.in.read).
- `PackageError` with `stage` attribute for precise failure diagnosis.
