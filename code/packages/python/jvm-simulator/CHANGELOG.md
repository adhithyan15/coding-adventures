# Changelog

All notable changes to the jvm-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-18

### Added
- `JVMOpcode` enum with real JVM opcode values (iconst_0-5, bipush, ldc, iload/istore variants, iadd/isub/imul/idiv, goto, if_icmpeq, if_icmpgt, ireturn, return)
- `JVMTrace` dataclass capturing PC, opcode mnemonic, stack before/after, locals snapshot, and description
- `JVMSimulator` class with load(), step(), and run() methods
- Variable-width bytecode decoding matching real JVM encoding
- Constant pool support via ldc instruction
- Control flow: goto (unconditional), if_icmpeq (branch if equal), if_icmpgt (branch if greater)
- Helper functions: assemble_jvm(), encode_iconst(), encode_istore(), encode_iload()
- Comprehensive test suite with >80% coverage
