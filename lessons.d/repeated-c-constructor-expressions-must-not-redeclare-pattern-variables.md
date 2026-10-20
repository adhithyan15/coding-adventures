---
category: C#
---

# Repeated C# constructor expressions must not redeclare pattern variables

Projection reconciliation reused the same width expression in constructor and
match calls. Its `is { } widths` pattern declared a local each time, and two
uses in one branch failed with CS0136. The compiled generated-code probe caught
this before native build. Use non-declaring null checks for repeatable argument
expressions, or hoist a single local. Keep compilation tests for generated
width-bearing nested projections; source string assertions miss scope errors.
