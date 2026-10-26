# Cortex-M33 simulator (Rust)

This package is the checked functional reference for a bounded Arm Cortex-M33
core. It composes the repository's Cortex-M3 integer execution surface with
Armv8-M Mainline security-state, banked control, stack-limit, and SAU behavior.

The boundary is intentionally architectural. It does not claim Cortex-M33
pipeline timing, RTL, DSP, floating point, MPU/NVIC/SysTick, AHB5, debug, or a
vendor SoC. See
[`../../../specs/07ap-cortex-m33-simulator.md`](../../../specs/07ap-cortex-m33-simulator.md)
for the exact contract and deferred work.

```rust
use cortex_m33_simulator::{CortexM33Simulator, SecurityState};

let cpu = CortexM33Simulator::new(true);
assert_eq!(cpu.security_state(), SecurityState::Secure);
```

Validation:

```text
cargo test -p cortex-m33-simulator --release
cargo clippy -p cortex-m33-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m33-simulator --no-deps
cargo llvm-cov -p cortex-m33-simulator --all-features --summary-only
```

The completed 0.1.0 cell has 91.03% line coverage (680/747 covered
source lines) under that exact all-feature command. Its release suite includes
an exact complete-core-state differential witness for every inherited
Cortex-M3 decode family.
