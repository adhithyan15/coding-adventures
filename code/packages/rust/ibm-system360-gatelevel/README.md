# IBM System/360 gate-level simulator

This crate is the digital-logic partner for `ibm-system360-simulator`. Its
8,389,403 persistent architectural bits—1 MiB of main storage, 16 general
registers, four floating-point registers, the instruction address, condition
code, and halt latch—are stored in repository D flip-flops.

The independent transition engine uses a one-hot opcode decoder, ripple-carry
address and arithmetic networks, Boolean logical and comparison networks,
fixed-width shifters, shift/add multiplication, restoring division, and
fixed-width hexadecimal-floating networks. Condition-mask selection,
zero/nonzero count and target tests, and signed index-branch polarity are
settled by gate reductions, muxes, and one-hot opcode lines rather than host
comparisons. A single 256-line decoder now settles before the functional
oracle runs. Gate OR reductions derive transport width and a fixed candidate
table maps exactly one asserted legal line to a typed operation; that same
authority is shared by execution, independent validation, oracle comparison,
and commit. Reserved lines still retain their architectural transport width,
so truncation keeps precedence over the eventual unknown-opcode fault.
Unselected candidates are never evaluated and therefore cannot read storage,
fault, or mutate state. The functional package is used only as a fail-closed
trace, fault, and complete-state oracle; no gate transition is clocked unless
its complete settled state agrees with that oracle.

The authority is regression-locked with an independent literal 256-entry
opcode/family table at each 2/4/6-byte loaded length, parsed-token digests of
the complete decode-to-commit chain, and compile-checked full-source hostile
mutations (including indirect raw-byte dispatch and post-authorization
overwrites). Known-width reserved holes also have short-image witnesses proving
that `TruncatedInstruction` precedes `UnknownOpcode`.

This is an educational ISA-visible System/360 base processor rather than a
cycle-accurate reconstruction of a particular model's SLT cards, channels,
timers, protection hardware, or storage technology. The fidelity boundary is
defined in
[`../../../specs/07ad2-ibm-system360-gatelevel.md`](../../../specs/07ad2-ibm-system360-gatelevel.md).

## Development

```bash
bash BUILD
```
