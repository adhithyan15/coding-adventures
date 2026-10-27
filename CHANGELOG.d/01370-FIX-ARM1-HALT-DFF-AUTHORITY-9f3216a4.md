### Fixed ARM1 HALT storage authority

- Made the gate-level ARM1 HALT DFF Q output the sole authority for snapshots,
  lifecycle guards, instruction execution, interrupts, and typed rollback.
- Added a concrete 536,871,777-DFF inventory, deliberate host-mirror divergence
  tests, and hostile structural fixtures that lock the complete HALT transition
  chain to the clocked DFF path.
