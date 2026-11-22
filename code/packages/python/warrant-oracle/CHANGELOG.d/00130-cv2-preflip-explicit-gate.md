# pre-flip: parity dumps and closed-gate tests use an explicit gate

No version bump, no committed byte moves.

- `parity_dumps.py`: `GATE_CLOSED = ("ir.core",)`, `GATE_ON` and `GATE_OPEN` are literals, no longer
  derived from `faults.IMPLEMENTED`. `shipped_derivation()` passes `GATE_CLOSED` and the fault view's
  `gate_shipped` is `list(GATE_CLOSED)`, so opening the shipped gate (the atomic flip, [D29]) cannot
  move the tuple-forms-v1 or fixed-eval-v1 dumps.
- Tests that assert `deferred` (closed-gate behaviour) pass `implemented=("ir.core",)` explicitly.
