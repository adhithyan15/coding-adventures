# Rust D05 operand-indexed forwarding seam

## Purpose

The reusable Rust `cpu-pipeline` currently represents one forwarding decision
with a scalar `HazardResponse`. That is insufficient when one instruction has
two pending sources from different stages. This change adds an opt-in path for
operand-indexed bypass values and keeps the existing token, hazard-response,
constructor, and snapshot shapes intact.

## Contract

- A forwarding record names the consumer source (`Rs1` or `Rs2`), source
  register, forwarded value, and producer stage (`Execute` or `Memory`).
- The opt-in resolver evaluates the current pipeline slots before advancement.
  For each source independently, the youngest matching producer wins: EX before
  MEM. A producer must be valid and write a register. A matching EX load is
  not ready; the existing hazard path must stall it rather than bypass an older
  value from MEM.
- The pipeline associates records with the decoded token as that token advances
  toward EX and passes them to an opt-in execute callback. Records are delivered
  only when the consumer advances; a stall or flush never delivers records for
  the held or discarded consumer. The opted-in callback path does not place a
  forwarding value in `PipelineToken.alu_result`.
- Forward records are kept as pipeline sidecar state. Existing public
  `PipelineToken`, `HazardResponse`, and `PipelineSnapshot` construction and
  legacy scalar forwarding behavior remain unchanged. An accessor exposes the
  records actually applied in the most recent step without changing snapshots.
- `core::Core::new_with_operand_forwarding` opts into per-source bypass
  resolution. `Core::new` and existing `ISADecoder::execute` implementations
  retain their current behavior. The decoder trait gains a default forwarded
  execution method that delegates to the old method; implementations may opt in
  to consuming the named values.
- A Core configured with a hardwired zero register never forwards a pending
  write to register zero. Other register-file configurations keep register zero
  as an ordinary register.

## Scope and checks

This is forwarding plumbing only. It does not implement RISC-V decoding,
initialize or expose Core RAM, alter memory commit timing, or define precise
faults. Focused tests cover independent EX/MEM sources, EX-over-MEM priority for
the same register, one-cycle load-use stall followed by MEM forwarding, x0,
flush/stall non-delivery, and unchanged legacy scalar forwarding. The
fixture-level runner and final RAM comparison are a later adapter task.
