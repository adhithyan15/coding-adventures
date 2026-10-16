---
category: Cross-platform & Windows BUILD_windows
---

# Bind the Python interpreter used by validation helpers

The D2 read-only source-transfer helper imported stdlib tomllib, but unqualified
python selected an older interpreter and failed before any proof or source/lock/
target mutation. Preserve that startup failure separately. Invoke the same
saved helper with the available py -3.13 interpreter, which supplies tomllib;
do not install an unnecessary package or alter reviewed dependencies. Record the
actual invocation and require a complete successful receipt before citing it.
