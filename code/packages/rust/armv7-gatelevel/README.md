# armv7-gatelevel

Exact gate-level partner for the educational ARMv7 machine in Spec 07b/07b2.

## Persistent topology

The machine owns exactly 524,805 clocked D flip-flops:

| Bank | Bits |
|---|---:|
| 64 KiB memory | 524,288 |
| R0–R15 (PC lives once in R15) | 512 |
| NZCV | 4 |
| Halt | 1 |

Installed origin and length are validated lifecycle metadata, not hardware
storage. Memory uses packed stable Q bits; registers, flags, halt, program
loads, and writes pass through `logic-gates` D-flip-flop primitives.

## Gate path

The implementation has its own complete Spec 07b execution path. Repository
gates implement condition predicates, decode equality, AND/OR, zero detection,
NZCV, a four-stage rotated-immediate mux network, and address/control Boolean
networks. ADD, SUB, CMP, sequential PC, effective addresses, and branch targets
use the repository ripple-carry adder. Host arithmetic does not implement the
architectural data path.

The public lifecycle mirrors `arm_simulator::functional::Armv7Simulator`:
validated complete restore, checked origin-aware loads and direct access,
atomic steps, complete before/after traces, and transactional bounded runs.

## Verification

- 2 gate/topology unit suites
- 4 lifecycle and atomicity suites
- exhaustive functional differentials across all 16 conditions, every data
  opcode/form, LDR/STR direction, B/BL, and HLT
- all 388 reproducible Python common-surface full-state vectors
- strict formatting, Clippy, and rustdoc
- 93.40% package line coverage (495/530 lines)
