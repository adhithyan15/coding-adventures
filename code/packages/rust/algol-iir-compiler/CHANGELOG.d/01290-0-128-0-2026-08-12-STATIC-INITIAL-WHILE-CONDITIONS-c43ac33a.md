## 0.128.0 — 2026-08-12 — static initial while conditions

Definite string initialization now flows out of a `while` for-list element
when a bounded static numeric comparison proves its initial condition true
after abstractly assigning the controlled variable's first value. The abstract
maps are restored before normal lowering; false, dynamic, call-bearing, array,
by-name, and captured-global cases remain fail-closed.

