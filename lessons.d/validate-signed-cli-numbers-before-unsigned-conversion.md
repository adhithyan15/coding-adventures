---
category: Cross-platform & Windows BUILD_windows
---

# Validate signed CLI numbers before unsigned conversion

`strtoul` accepts a leading minus and converts the negative magnitude modulo
the unsigned type. On a 32-bit `unsigned long`, `-4294967295` becomes `1`, so
a CLI that checks only the converted range can accept a negative count. First
require the complete decimal token to contain only digits, then call
`strtoul` with `errno` reset and reject `ERANGE`, trailing text, and values
outside the application's range. Include a negative value that wraps to a
small valid value on the narrowest supported unsigned type in CLI tests.

Compiler compatibility macros also overlap: `clang-cl` defines both
`__clang__` and `_MSC_VER`. Detect Clang before the MSVC fallback when runtime
metadata should report the compiler actually producing the binary.
