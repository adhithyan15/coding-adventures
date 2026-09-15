## 0.111.0 — 2026-08-11 — straight-line static real scalar output

Local real scalars assigned a finite compile-time expression may now print its
canonical decimal value through the existing portable string path. Tracking is
restricted to straight-line local state and is cleared by labels, branches,
loops, gotos, procedure calls, dynamic reassignment, and captured globals, so
values that can vary at run time still fail closed pending a typed formatter.

