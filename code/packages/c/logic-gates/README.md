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

Every function takes `int`s by value and returns 0 or 1, or `LG_INVALID` (-1)
if any argument is not exactly 0 or 1. No pointers, allocation, I/O or globals,
so the memory-safety bug classes cannot occur.

## Tests

```
sh tools/run.sh      # ISO harness (gcc/clang), strict warnings, ASan+UBSan if available
```

On Windows, `tools\run.ps1` builds with MSVC under warnings-as-errors (no
sanitizers).
