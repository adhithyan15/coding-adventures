## 0.225.11 - 2026-08-13 (ALGOL static conditional while effects)

The seven-backend ALGOL matrix now proves that an unreachable dependency write
behind a variable-free false body condition does not invalidate a capped
`while` exit.

