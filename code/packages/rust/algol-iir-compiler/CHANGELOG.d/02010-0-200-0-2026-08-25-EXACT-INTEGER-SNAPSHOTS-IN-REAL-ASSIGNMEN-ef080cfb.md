## 0.200.0 — 2026-08-25 — exact integer snapshots in real assignments

Real scalar assignment targets now provide the numeric context needed to widen
tracked integer snapshots. Pure integer expressions retain checked integer
semantics before widening, which remains bounded to values exactly
representable in binary64; overflow and larger values invalidate static real
metadata instead of recording a rounded compile-time value.

