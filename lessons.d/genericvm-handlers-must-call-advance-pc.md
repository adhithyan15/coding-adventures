---
category: Elixir
---

# GenericVM handlers must call `advance_pc`

at the end, or the VM loops forever. Exceptions: `HALT`, unconditional `JUMP` (uses `jump_to`), conditional jumps (advance OR jump, never both).
