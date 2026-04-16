# JVM Class File

Reusable JVM class-file decoding primitives for the versioned JVM stack.

This package is intentionally separate from the simulator so a future real JVM,
class loader, verifier, or JIT can reuse the same class-file parsing layer.
