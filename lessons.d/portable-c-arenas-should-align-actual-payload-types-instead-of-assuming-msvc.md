---
category: Cross-platform & Windows BUILD_windows
---

# Portable C arenas should align actual payload types instead of assuming MSVC max_align_t

NN47 compiled under GCC and Clang but MSVC's C17 stddef.h did not expose
max_align_t, so the arena header type failed before tests could run. Its actual
payloads were only nodes containing pointers, size_t, int64_t and double, plus
byte strings. Align the arena header union explicitly for those exact types;
malloc still supplies the base alignment, and sizeof(union) preserves alignment
at the following payload. Do not assume a compiler's C17 switch implements
every optional/header facility, or weaken warnings to hide a missing type.
