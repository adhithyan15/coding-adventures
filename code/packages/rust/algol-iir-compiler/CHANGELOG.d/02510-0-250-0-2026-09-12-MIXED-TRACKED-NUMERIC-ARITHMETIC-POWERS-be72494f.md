## 0.250.0 — 2026-09-12 — mixed tracked numeric arithmetic powers

Finite arithmetic that mixes initialized tracked local integer and real
snapshots may now provide a bounded integral exponent for a power whose base
contains a path-independent pure built-in result. Integer snapshots must widen
exactly to binary64; uninitialized, inexact, fractional, conditional,
call-bearing, nested-power, and standard-free forms fail closed.

