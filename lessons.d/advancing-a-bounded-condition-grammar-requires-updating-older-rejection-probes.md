---
category: Compiler / VM / language pipeline
---

# Advancing a bounded condition grammar requires updating older rejection probes

The PREP01 C dialect gained `!(operand)`, but two older tests still listed
`!(1)` among rejected expressions. The focused new tests passed while the
larger dialect suite failed on those stale expectations.

When a partial grammar deliberately accepts a formerly rejected shape, search
existing negative fixtures for that exact shape before running the full suite.
Remove only the obsolete rejection, keep neighboring unsupported forms, and
update the staged spec so its earlier scope and current scope remain clear.
