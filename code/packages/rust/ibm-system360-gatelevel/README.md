# IBM System/360 gate-level simulator

This crate is the digital-logic partner for `ibm-system360-simulator`. Its
8,389,403 persistent architectural bits—1 MiB of main storage, 16 general
registers, four floating-point registers, the instruction address, condition
code, and halt latch—are stored in repository D flip-flops.

The independent transition engine uses a one-hot opcode decoder, ripple-carry
address and arithmetic networks, Boolean logical and comparison networks,
fixed-width shifters, shift/add multiplication, restoring division, and
fixed-width hexadecimal-floating networks. The functional package is used as
a fail-closed trace and fault oracle; no gate transition is clocked unless its
complete settled state agrees with that oracle.

This is an educational ISA-visible System/360 base processor rather than a
cycle-accurate reconstruction of a particular model's SLT cards, channels,
timers, protection hardware, or storage technology. The fidelity boundary is
defined in
[`../../../specs/07ad2-ibm-system360-gatelevel.md`](../../../specs/07ad2-ibm-system360-gatelevel.md).

## Development

```bash
bash BUILD
```
