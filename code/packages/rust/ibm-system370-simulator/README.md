# IBM System/370 functional simulator

This crate models the bounded problem-state IBM System/370 core defined by
[Spec 07as](../../../specs/07as-ibm-system370-simulator.md). It retains the
repository's checked System/360 integer, branch, character, and hexadecimal
floating-point surface and adds the characteristic long-field and masked-byte
instructions `MVCL`, `CLCL`, `CLM`, `STCM`, and `ICM`.

The public API is wholly System/370-owned. The System/360 simulator is a private
compatibility oracle and executor for inherited instructions; none of its state,
trace, result, or error types appear in public signatures.

```rust
use ibm_system370_simulator::System370Simulator;

let mut cpu = System370Simulator::new();
cpu.load(&[0xbf, 0x28, 0x00, 0x80, 0x0a, 0x00], 0)?;
cpu.write_memory(0x80, &[0x80])?;
let result = cpu.run(2)?;
assert!(result.halted);
assert_eq!(result.final_state.gprs[2], 0x8000_0000);
assert_eq!(result.final_state.condition_code, 1);
# Ok::<(), ibm_system370_simulator::System370Error>(())
```

The simulator uses 24-bit architectural addresses over one installed MiB of
storage. Every checked failure is atomic, and `run` is transactional across the
whole bounded execution.

## Verification

From `code/packages/rust`:

```sh
cargo test -p ibm-system370-simulator --release
cargo clippy -p ibm-system370-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ibm-system370-simulator --no-deps
cargo llvm-cov -p ibm-system370-simulator --all-targets --summary-only
```

The package has fourteen focused tests. They include an exhaustive first-byte
classification witness for every inherited opcode and produce 100% function
coverage and 98.78% line coverage (568/575 lines).
