# Machine-layer probe fixtures

`boards.json` contains the `ascii-console-demo-v1` instruction bytes and runs
that identical image and synthetic CPU on two portable board maps. The cases
expect identical console bytes with different memory capacity and mapped
console addresses. They also record the bounded instruction budgets, final CPU
registers, instruction sequence, and data-only console write effects.
`verify_boards.mjs` reads this fixture outside the package root, runs the
library-only board simulator, and compares both traces for build validation.

`riscv.json` contains an RV32I instruction image executed by the repository's
existing RISC-V simulator through the same board bus. Its guest reads the
console address from configuration ROM and writes `OK\n` with byte stores on
both maps. `verify_riscv.test.ts` is an external build-time oracle for the exact
instruction, register, halt, console, and device-effect results. The adapter
does not claim transaction-wide atomicity for multi-byte word stores; the
fixture uses only individual byte stores.
