# logic-gates (C)

The seven basic logic gates (NOT, AND, OR, XOR, NAND, NOR, XNOR) and four
NAND-only constructions (NOT, AND, OR, XOR), in pure ISO C17.

## Where it fits

```
transistors  ->  logic gates (this package)  ->  arithmetic (adders, ALU) -> CPU
```

Gates are the first software layer above transistors. See
`code/specs/10-logic-gates.md` ("C lane API") for the contract.

## API

```c
#include "logic_gates.h"

int r = lg_and(1, 1);        /* 1 */
int x = lg_nand_xor(1, 0);   /* 1, built from four lg_nand calls */
int e = lg_or(2, 0);         /* LG_INVALID (-1): 2 is not a bit */
```

**Never use a result directly as a boolean (CWE-252).** `LG_INVALID` is -1,
which is truthy in C, so `if (lg_and(a, b))` treats invalid input as true.
Write `if (lg_and(a, b) == 1)`, or check `r != LG_INVALID` first. Every
function is declared `LG_NODISCARD` (C23 `[[nodiscard]]`, else
`__attribute__((warn_unused_result))` on GCC/Clang, else `_Check_return_` on
MSVC), so discarding a result warns, and fails under `-Werror`.

Every function takes `int`s by value and returns 0 or 1, or `LG_INVALID` (-1)
if any argument is not exactly 0 or 1. No pointers, allocation, I/O or globals,
so the memory-safety bug classes cannot occur.

## Tests

```
sh tools/run.sh      # ISO harness (gcc/clang), strict warnings, nodiscard negative control,
                     # ASan+UBSan (skipped locally if unavailable; FAILS when CI is set)
# CI counts as true iff $CI is set to a value other than "", "false" or "0".
# Under CI, every compiler found must run the nodiscard control and the sanitizers.
```

On Windows, `tools\run.ps1` builds with MSVC under warnings-as-errors (no
sanitizers).
