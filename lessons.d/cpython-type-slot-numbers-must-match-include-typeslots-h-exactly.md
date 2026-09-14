---
category: Native extensions & FFI
---

# CPython type-slot numbers must match `Include/typeslots.h` exactly

Wrong slots cause silent memory corruption / `UnicodeDecodeError` / access-violation crashes during module load. Numbers are NOT sequential per category; verify each. Examples: `Py_tp_hash=59`, `Py_tp_iter=62`, `Py_nb_and=8`, `Py_nb_or=31`.
