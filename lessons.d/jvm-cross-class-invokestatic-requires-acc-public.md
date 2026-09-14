---
category: Python
---

# JVM cross-class invokestatic requires `ACC_PUBLIC`

— a method tagged `ACC_PRIVATE` on class A cannot be called by class B even via `invokestatic`. The JVM raises `NoSuchMethodError` at runtime (not a compile-time error). In multi-module mode set all callable methods to `ACC_PUBLIC | ACC_STATIC`.
