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
- `constexpr` and `[[nodiscard]]` throughout; the valid-input path allocates
  nothing and does no I/O or global access. The **rejection** path allocates (a
  `std::string` message, copied into the exception's heap buffer; it may throw
  `std::bad_alloc` instead). `std::invalid_argument(const char*)` also
  allocates, so a fixed message would not change this.
- For DIRECT calls only an exact `int` argument is accepted. These are
  rejected at compile time by deleted overloads: `short`, `signed char`,
  `unsigned char`, `char`, `bool`, `unsigned`, `long`, `unsigned long`,
  `long long`, `float`, `double`, enums, `nullptr`, string literals, and
  braced-init-lists of any kind (`and_gate({true}, 1)`, `{c}`, `{1}`, `{}`).
- The deleted overloads guard **direct calls only**. Reaching a gate through an
  explicit function-pointer type (`int (*f)(int, int) = &and_gate;`), a cast
  (`static_cast<int (*)(int, int)>(and_gate)`) or a lambda converts the
  caller's value to `int` in the *caller* (enum, `wchar_t`, `char16_t`,
  `unsigned short`, a class with `operator int`, a narrowed 2^32+1 which becomes
  1, 2^64-1 which becomes -1); the gate can only validate the `int` it receives.
  `std::function<int(int,int)> g = and_gate;` does not compile.
- Gates are not `noexcept`: an invalid input reaching a gate from a `noexcept`
  caller calls `std::terminate` (tested).
- All symbols live in the inline namespace `logic_gates::v0_2`, so two header
  copies of different versions cannot silently violate the ODR;
  `logic_gates::and_gate` and `using namespace logic_gates` keep working.
- Macro-hygienic: internals use `lg_`-prefixed names, take no pointer arguments, and are called through
  parentheses, so user macros such as `check`, `min`, `a` defined before the
  include do not break it.

## Testing

`sh tools/run.sh` (or `tools\run.ps1` on Windows) builds the tests with each
available compiler using `-std=c++17 -Wall -Wextra -Wpedantic -Wconversion
-Wsign-conversion -Werror` at `-O0` and `-O2`, runs a macro-hygiene TU,
compile-must-fail tests (deleted overloads, braced lists, invalid constant
expressions, discarded results, `std::function`), and ASan+UBSan. The sanitizer
run is **mandatory**: a toolchain that cannot run it fails the script unless
`LG_ALLOW_NO_SANITIZER=1` is set (which prints a loud warning). MSVC has no
ASan+UBSan combination, so `tools\run.ps1` builds with `/O2` and warnings as
errors instead. Clang header coverage is enforced at 100% (regions, functions, lines).
The header also defines `LOGIC_GATES_VERSION` (200 for 0.2.0) and `#error`s if a different version was already included.
