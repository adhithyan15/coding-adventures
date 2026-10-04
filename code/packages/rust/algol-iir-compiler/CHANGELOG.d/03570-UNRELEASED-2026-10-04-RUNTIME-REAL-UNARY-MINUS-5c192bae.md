## 0.357.0 - Runtime real unary minus

Unary minus now preserves runtime-real formatter provenance through direct
output, scalar assignment, and later signed reads. Signed expressions no longer
take the bare-variable output fast path and therefore retain their sign.
