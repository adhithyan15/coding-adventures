# IBM System/360 Simulator (Rust)

Checked functional model of the IBM System/360 CPU architecture. This is the
behavioral reference for the repository's later gate-level System/360 cell.

The machine model exposes sixteen 32-bit general registers, four raw 64-bit
hexadecimal-floating registers, a 24-bit instruction address, the condition
code, one MiB of installed big-endian storage, halt state, and the exact loaded
program range. Its five historical instruction formats cover fixed-point,
logical, character, storage-to-storage, branch, shift, multiply/divide, and
short/long hexadecimal floating-point families.

All public lifecycle operations are checked. A fault is typed and atomic:
misaligned data, a truncated instruction, bad register pair, divide by zero,
unknown opcode, or exhausted step bound leaves the complete prior state intact.
Because System/360 has no unprivileged halt instruction, repository programs
use `SVC 0` (`0A00`) as their deterministic halt sentinel.

```rust
use ibm_system360_simulator::System360Simulator;

// LR R1,R2; SVC 0
let program = [0x18, 0x12, 0x0a, 0x00];
let mut cpu = System360Simulator::new();
cpu.load(&program, 0x100)?;
cpu.write_gpr(2, 42)?;
let result = cpu.run(4)?;
assert_eq!(result.final_state.gprs[1], 42);
# Ok::<(), ibm_system360_simulator::System360Error>(())
```

Hexadecimal floating arithmetic operates on integer fractions and radix-16
exponents rather than host binary floating point, preserving all 24 or 56
architectural fraction bits. The optional decimal feature, extended precision,
privileged control, interrupts, protection, channel programs, and devices are
explicitly outside this CPU cell.

The normative source is IBM's *System/360 Principles of Operation*, form
GA22-6821-8 (November 1970), especially its instruction tables, effective
address rules, condition-code tables, and hexadecimal floating representation.
See `code/specs/07ad-ibm-system360-simulator.md` for the repository contract.

## Verification

From `code/packages/rust`:

```sh
cargo test -p ibm-system360-simulator --release
cargo clippy -p ibm-system360-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ibm-system360-simulator --no-deps
cargo llvm-cov -p ibm-system360-simulator --summary-only
```

The package has nineteen passing tests, 100% function coverage, and 93.46%
line coverage (971/1,039 lines).
