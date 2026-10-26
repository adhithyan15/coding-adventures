---
category: Cross-platform & Windows BUILD_windows
---

# A successful vcvars64 banner does not prove Windows SDK include and library paths exist

On the NN47 host vcvars64 printed Environment initialized for x64 and found cl,
but INCLUDE contained only MSVC directories and WindowsSdkDir was absent. Even
stddef.h/stdio.h failed; this was SDK bootstrap, not a parser syntax defect.
Read the effective INCLUDE/LIB values and verify installed SDK directories.
For bounded local validation, add the actual installed UCRT/shared/UM includes
and UCRT/UM x64 libraries to the child process environment. Do not commit a
machine-specific SDK path into portable BUILD fronts or claim the failed build
passed. Actual repository CI must still supply its normal MSVC setup.
