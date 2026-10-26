---
category: Cross-platform & Windows BUILD_windows
---

# ISO C fixture readers need narrow Windows CRT compatibility without weakening warnings

NN47's strict native Windows Clang compile rejected the test fixture's ISO
`fopen` because the Microsoft CRT deprecates it in favor of non-ISO `fopen_s`.
The library itself did not use file I/O. Keep warnings-as-errors and pure C17:
use the CRT's documented compatibility macro only in the fixture test translation
unit, before stdio headers. Do not disable compiler warnings globally or move
fixture I/O into the library. Replace avoidable string copying with an explicit
length guard and memcpy instead of suppressing an unnecessary strcpy warning.
