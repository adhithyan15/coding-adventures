## [0.1.0] - 2026-11-24

### Added

- Header-only C++17 `logic_gates` library (`include/logic_gates/logic_gates.hpp`):
  `not_gate`, `and_gate`, `or_gate`, `xor_gate`, `nand_gate`, `nor_gate`,
  `xnor_gate`, and the NAND-only `nand_not`, `nand_and`, `nand_or`, `nand_xor`
  (4-NAND construction).
- All functions are `constexpr`, return exactly 0 or 1, and throw
  `std::invalid_argument` (naming the parameter and value) for any argument
  that is not exactly 0 or 1; every function validates every argument itself.
- Deleted template overloads reject `bool`, `char`, floating point, `long`,
  `unsigned`, etc. at compile time (CWE-843/704 type confusion).
- Tests: `static_assert` truth tables, runtime truth tables, NAND-built equals
  direct, exhaustive rejection over [-1024, 1024] plus INT_MIN/INT_MAX edges in
  every argument position, and `void_t` traits proving non-int calls do not
  compile. Runner covers clang++/g++ with strict warnings-as-errors, ASan+UBSan,
  and clang source-based coverage (100% of header lines).
- Spec: "C++ lane API" section in `code/specs/10-logic-gates.md`.
