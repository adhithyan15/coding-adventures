# HOS-D05-GO2 — Insert Go load-use bubbles at EX

## Defect and bounded correction

The Go hazard detector's `StallCycles` is a duration. The generic pipeline's
`HazardResponse.StallStages` is used as the index of the stage that receives a
bubble. The Core adapter currently copies the duration (`1`) into the index,
which inserts the bubble in ID. IF and earlier stages freeze, but the dependent
token moves out of ID into EX on the very cycle the load moves into MEM. It can
therefore execute using stale register-file state before the loaded value can
be forwarded.

For a detected one-cycle load-use stall, the Core adapter must insert the
bubble at the first configured execute-stage index. The ID and earlier stages
remain held for that cycle; older instructions continue through MEM and WB.
On the next cycle, a matching load in MEM forwards its completed `WriteData`
to the still-held consumer. Derive the insertion point from the configured
pipeline stage roles, including the existing Classic5Stage default when no
explicit stages are supplied. This does not change the generic pipeline's
public `StallStages` behavior or the hazard detector's duration result.

## Acceptance evidence

Add an actual RV32I load-use program that first gives the destination register a
nonzero stale value, loads a different value (including zero), then consumes it
in both an arithmetic instruction or branch. Assert the dependent token remains
in ID during the stall and enters EX with the MEM-tagged loaded value on the
next cycle. Compare the branch decision, retired PC path, final registers, and
flat RAM with the independent sequential RISC-V simulator. Retain the
structural-hazard detector's direct behavior and test adjacent ALU progress at
the Core adapter boundary.

This correction is scoped to translating the Core's hazard result into the
pipeline's insertion point. It makes no claims about multi-cycle memory stalls,
precise faults, stores at retirement, MMIO, or the shared D05 fixture.
