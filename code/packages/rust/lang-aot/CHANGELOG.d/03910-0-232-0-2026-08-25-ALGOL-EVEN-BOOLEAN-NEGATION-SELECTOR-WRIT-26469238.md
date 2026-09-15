## 0.232.0 - 2026-08-25 (ALGOL even boolean negation selector writes)

The seven-backend ALGOL matrix now proves `flag := not not flag` preserves a
transitive bounded-while selector dependency. Compiler regressions also pin a
four-negation identity while retaining the existing odd-negation fail-closed
case.

