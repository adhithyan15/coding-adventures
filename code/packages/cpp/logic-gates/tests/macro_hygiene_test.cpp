// Macro-hygiene test: user macros with common names are defined BEFORE the
// header is included.  The header must still compile and behave, because its
// internals and parameters use lg_-prefixed names.  (Standard headers are
// included first: macros that redefine common identifiers before <string>
// would break the standard library itself, which is the user's problem.)
#include <cstdio>
#include <stdexcept>
#include <string>

#define check(x) (this_is_not_valid_code_if_expanded x)
#define reject(x) (this_is_not_valid_code_if_expanded x)
#define min(x, y) (this_is_not_valid_code_if_expanded x y)
#define max(x, y) (this_is_not_valid_code_if_expanded x y)
#define assert(x) (this_is_not_valid_code_if_expanded x)
#define bit(x) (this_is_not_valid_code_if_expanded x)
#define a (this_is_not_valid_code_if_expanded)
#define b (this_is_not_valid_code_if_expanded)
#define n (this_is_not_valid_code_if_expanded)
#define n1 (this_is_not_valid_code_if_expanded)
#define n2 (this_is_not_valid_code_if_expanded)
#define n3 (this_is_not_valid_code_if_expanded)
#define T (this_is_not_valid_code_if_expanded)
#define U (this_is_not_valid_code_if_expanded)
#define param (this_is_not_valid_code_if_expanded)
#define value (this_is_not_valid_code_if_expanded)
#define detail (this_is_not_valid_code_if_expanded)

#include "logic_gates/logic_gates.hpp"

static_assert(logic_gates::and_gate(1, 1) == 1, "AND under hostile macros");
static_assert(logic_gates::nand_xor(1, 0) == 1, "NAND XOR under hostile macros");

int main() {
  const int ok = logic_gates::or_gate(0, 1) + logic_gates::not_gate(0) + logic_gates::nand_not(1);
  bool threw = false;
  try {
    (void)logic_gates::xor_gate(2, 0);
  } catch (const std::invalid_argument& e) {
    threw = std::string(e.what()).find("'a'") != std::string::npos;
  }
  const bool pass = ok == 2 && threw;
  std::printf("macro hygiene: %s\n", pass ? "OK" : "FAIL");
  return pass ? 0 : 1;
}
