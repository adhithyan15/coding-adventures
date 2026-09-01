# Layer 07x2 — ARMv7-A / Thumb-2 Gate-Level Simulator

## Scope

This specification is the gate-level partner to Spec 07x and its completed
Rust `armv7a-simulator` oracle. It implements the same educational Thumb-only,
mixed 16/32-bit instruction surface, exact wrapping memory, state, checked
lifecycle, traces, results, and typed fault boundary.

## Persistent topology

The machine contains exactly **524,833 D flip-flops**:

| Bank | Width/count | DFFs |
|---|---:|---:|
| Little-endian memory | 65,536 x 8 | 524,288 |
| R0-R15, with PC stored once in R15 | 16 x 32 | 512 |
| CPSR | 32 | 32 |
| Halt | 1 | 1 |
| **Total** | | **524,833** |

Installed origin and length are checked lifecycle metadata rather than
architectural hardware state. Stable instruction-boundary storage may pack one
Q bit per DFF, but reset, restore, program installation, register writes,
memory writes, CPSR updates, and halt transitions clock repository DFF
primitives.

## Combinational execution

The gate machine has an independent complete Spec 07x execution path.
Repository logic gates implement mixed-width classification and decode,
condition predicates, Boolean operations, zero/sign/carry/overflow logic,
barrel shifts and rotates, register-list selection, and control/address muxes.
Repository ripple-carry networks implement sequential PC, branch targets,
effective addresses, arithmetic, compare, ADC/SBC/RSB, and stack updates.
Multiplication uses a fixed 32-stage gate partial-product network. Host integers
may assemble traces and index memory, but do not implement architectural ALU
results.

## Lifecycle and verification

The public state/error/trace/result contract is shared with
`armv7a-simulator`. Restore and origin-aware loads validate before commit;
faulting steps preserve the complete entry state; bounded runs are
transactional. Verification pins the topology, every DFF-backed public write,
all lifecycle and manual Spec 07x edges, all 417 Python common-surface vectors,
complete functional state/trace differential coverage, strict formatting,
Clippy and rustdoc, and at least 80% package line coverage.

The completed Rust package passes four topology/datapath unit tests, six
lifecycle suites, six manual Spec 07x suites, and all 417 Python common-surface
transitions in complete trace/state lockstep with the functional oracle. Strict
formatting, Clippy, and rustdoc pass. Package line coverage is 97.41% (716/735).
