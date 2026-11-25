# logic-gates (C++)

Header-only C++17 library of the seven fundamental logic gates (NOT, AND, OR,
XOR, NAND, NOR, XNOR) plus NAND-only constructions of NOT, AND, OR and XOR, in
namespace `logic_gates`.

## Where it fits

```
transistors  ->  logic gates (this package)  ->  arithmetic (adders, ALU)  ->  CPU
```

Semantics follow `code/specs/10-logic-gates.md` (see its "C++ lane API"
section) and the Rust reference implementation.

## Usage

```cpp
#include "logic_gates/logic_gates.hpp"
using namespace logic_gates;

static_assert(and_gate(1, 1) == 1);          // checked at compile time
static_assert(nand_xor(1, 0) == 1);

int y = nor_gate(0, 0);                      // 1
try { and_gate(2, 1); }                      // not a bit
catch (const std::invalid_argument& e) { /* "parameter 'a' must be 0 or 1, got 2" */ }
// and_gate(true, 1);  // does not compile: only exact int arguments are allowed
```

## Contract

- Inputs and outputs are `int` bits: exactly 0 or 1.
- Any other value throws `std::invalid_argument`; every function validates every
  argument itself (the NAND-built ones included).
- `constexpr` throughout; no pointers, allocation, I/O or globals.
- Deleted overloads reject `bool`, `char`, floating point, `long`, `unsigned`
  and other non-`int` argument types at compile time.

## Testing

`sh tools/run.sh` (or `tools\run.ps1` on Windows) builds the tests with each
available compiler using `-std=c++17 -Wall -Wextra -Wpedantic -Wconversion
-Wsign-conversion -Werror`, repeats under ASan+UBSan, and prints clang coverage.
