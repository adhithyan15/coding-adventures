## 0.231.0 - 2026-08-25 (ALGOL real unit selector writes)

The seven-backend ALGOL matrix now proves a computed `choose := choose * 1.0`
write preserves a transitive bounded-while selector dependency. The compiler
also pins the symmetric multiplication and division forms while keeping real
additive zero conservative.

