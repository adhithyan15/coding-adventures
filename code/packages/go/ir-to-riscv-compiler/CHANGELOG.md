# Changelog

## 0.3.0

- Preserve mapped caller virtual registers around `CALL`.
- Document `v0` and `v1` as volatile starter-ABI registers.

## 0.2.0

- Add call-frame lowering for nested `CALL`/`RET` programs.
- Emit a hidden call stack for compiler-generated RISC-V images that need one.

## 0.1.0

- Initial `ir-to-riscv-compiler` package.
- Lowers compiler IR v1 to RV32I machine-code bytes.
- Records label/data offsets and `IrToMachineCode` mappings.
- Includes simulator-backed tests for arithmetic, memory, and branches.
