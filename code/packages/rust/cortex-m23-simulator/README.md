# Cortex-M23 functional simulator

This package is a checked, bounded teaching model of one Arm Cortex-M23
(Armv8-M Baseline) core. It reuses the repository's checked Cortex-M0
instruction surface and adds the Cortex-M23 Security Extension boundary:
Secure/Non-secure execution state, banked stack/mask/control state, an
eight-region SAU, checked attribution on fetch and data accesses, and explicit
security-transition hooks.

The public contract is specified in
[`07ao-cortex-m23-simulator.md`](../../../specs/07ao-cortex-m23-simulator.md).
It deliberately does not claim Armv8-M-exclusive opcodes, encoded SG/BXNS/
BLXNS, exception entry/return, NVIC, MPU/IDAU, debug, timing, RTL, or a vendor
SoC. Those require separate bounded cells and evidence.

Run the package checks from `code/packages/rust`:

```sh
cargo test -p cortex-m23-simulator --release
cargo clippy -p cortex-m23-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cortex-m23-simulator --no-deps
```
