# Changelog

## Unreleased

- add the first Rust `brainfuck-jvm-compiler` orchestrator
- expose `compile_source`, `pack_source`, and `write_class_file`
- carry Brainfuck AST, raw IR, optimized IR, parsed class, and class bytes in the result
- add stage-labeled parse, IR compile, JVM lower, validation, and write errors
