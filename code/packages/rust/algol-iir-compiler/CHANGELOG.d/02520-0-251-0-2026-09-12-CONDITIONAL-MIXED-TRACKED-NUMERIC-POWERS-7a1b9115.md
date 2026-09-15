## 0.251.0 — 2026-09-12 — conditional mixed tracked numeric powers

Pure conditional arithmetic mixing initialized tracked local integer and real
snapshots may now provide a bounded power exponent when every runtime branch
proves the same exact integral value. The selector remains emitted; differing,
uninitialized, effectful, nested-power, and standard-free forms fail closed.

