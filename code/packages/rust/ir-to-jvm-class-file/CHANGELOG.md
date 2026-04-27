# Changelog

## Unreleased

- add the first Rust `ir-to-jvm-class-file` backend
- lower the current Brainfuck and Nib IR subset into verifier-friendly JVM bytecode
- emit helper methods for register access, byte/word memory, and syscalls
- validate class names and write generated classes into classpath layout safely
- add end-to-end tests for generic lowering plus Brainfuck and Nib source lanes
