# Changelog

All notable changes to the CLR IL Simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-18

### Added

- Initial CLR IL simulator implementation with step-by-step execution traces
- Constant loading: `ldc.i4.0` through `ldc.i4.8` (short forms), `ldc.i4.s` (signed int8), `ldc.i4` (int32)
- Local variable access: `ldloc.0`-`ldloc.3`, `stloc.0`-`stloc.3` (short forms), `ldloc.s`, `stloc.s` (generic)
- Type-inferred arithmetic: `add`, `sub`, `mul`, `div` (single opcode for all numeric types)
- Two-byte comparison opcodes: `ceq`, `cgt`, `clt` (with 0xFE prefix)
- Short branch instructions: `br.s`, `brfalse.s`, `brtrue.s` (signed int8 offsets)
- `nop`, `ldnull`, `ret` instructions
- Helper functions: `assemble_clr()`, `encode_ldc_i4()`, `encode_stloc()`, `encode_ldloc()`
- `CLRTrace` dataclass for detailed execution tracing
- `CLROpcode` enum with real ECMA-335 opcode values
- Comprehensive test suite with >80% code coverage
