## 0.244.0 - 2026-08-25 (ALGOL real-target integer snapshots)

The ALGOL matrix now carries a tracked integer through a real assignment target
and a transitive mixed-numeric while dependency on all seven standard backends.
This exercises exact binary64 widening without introducing runtime real
formatting or dynamic procedure descriptors.

