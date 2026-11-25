// logic_gates.hpp -- the seven fundamental logic gates plus NAND-built versions.
//
// Where this sits in the stack
// ----------------------------
//   transistors  ->  LOGIC GATES (this file)  ->  arithmetic (adders, ALU) -> CPU
//
// A logic gate maps one or two bits to one bit.  A bit is the integer 0 or 1.
//
// Truth tables (a, b -> output)
// -----------------------------
//     a b | AND OR XOR NAND NOR XNOR        a | NOT
//     0 0 |  0   0   0    1    1    1       0 |  1
//     0 1 |  0   1   1    1    0    0       1 |  0
//     1 0 |  0   1   1    1    0    0
//     1 1 |  1   1   0    0    0    1
//
// NAND is "universal": every other gate can be built from NAND alone.
//
//     NOT a    = NAND(a, a)
//     AND a b  = NAND(n, n) where n = NAND(a, b)
//     OR  a b  = NAND(NAND(a,a), NAND(b,b))          (De Morgan)
//     XOR a b  = the classic four-NAND circuit:
//
//                  a ----+-------------[NAND]--- n2 ---+
//                        |               |             |
//                        +--[NAND]-- n1 -+             +--[NAND]--- out
//                        |               |             |
//                  b ----+-------------[NAND]--- n3 ---+
//
//                n1 = NAND(a, b); n2 = NAND(a, n1); n3 = NAND(b, n1);
//                out = NAND(n2, n3)
//
// Design decisions (see code/specs/10-logic-gates.md, "C++ lane API")
// -------------------------------------------------------------------
// * Error model: EXCEPTIONS.  An argument that is not exactly 0 or 1 throws
//   std::invalid_argument naming the parameter and the value.  Returning a
//   sentinel (as the C lane does with LG_INVALID) would let a careless caller
//   treat "invalid" as a bit; an exception cannot be ignored by accident.
//
// * constexpr: every gate is a constant expression.  Tests static_assert the
//   whole truth tables at compile time.  A throw inside a constant expression
//   is a hard compile error, so an invalid input can never be silently
//   constant-folded either.
//
// * Deleted overloads: the real functions take `int`.  Without further care,
//   and_gate(true, 1.5) would compile, silently converting 1.5 -> 1 (an
//   ACCEPTED, silently wrong bit), and and_gate(0u, 1u) would hide an
//   unsigned -> signed conversion.  That is a type-confusion bug class
//   (CWE-843 access of resource using incompatible type, CWE-704 incorrect
//   type conversion).  So for each gate we also declare a deleted function
//   TEMPLATE.  Overload resolution then works like this:
//     - an exact `int` call matches the non-template function exactly, and a
//       non-template beats a template on an otherwise equal match -> OK;
//     - any other argument type (bool, char, double, long, unsigned, ...)
//       cannot match the non-template without a conversion, but the template
//       deduces the exact types -> the better match -> it is deleted ->
//       a compile error.
//
// * Every function validates EVERY argument itself, including the NAND-built
//   ones, before delegating; no function relies on a callee for its contract.
//
// * Direct calls only: the deleted overloads guard DIRECT calls.  When a gate
//   is reached through a function pointer `int (*)(int, int)`, a
//   std::function<int(int, int)>, or any other indirection, the conversion of
//   the caller's value (enum, wchar_t, char16_t, unsigned short, a class with
//   `operator int`, a 2^32+1 or 2^64-1 integer narrowed to int, ...) happens
//   in the CALLER, before the gate sees an int; the gate can only validate the
//   int value it receives (anything other than 0 or 1 is rejected).  Only an
//   explicit function-pointer type (`int (*f)(int,int) = &and_gate;`), a cast
//   (`static_cast<int (*)(int, int)>(and_gate)`) or a lambda reaches the int
//   overload; `std::function<int(int,int)> g = and_gate;` does not compile
//   (the overload set cannot be deduced).  Braced-init-list arguments
//   (`and_gate({true}, 1)`) are rejected by extra deleted initializer_list
//   overloads.  The gates are not noexcept: a throwing gate called from a
//   noexcept caller calls std::terminate (documented, tested in run.sh).
//
// * [[nodiscard]]: every gate result must be used; dropping it is a warning
//   (an error under -Werror).  A gate called only for its throw is a bug.
//
// * Allocation: the valid-input path performs no allocation, I/O, or global
//   access and has no undefined behaviour.  The REJECTION path allocates: it
//   builds a std::string, and std::invalid_argument copies it into a heap
//   buffer (constructing it may itself throw std::bad_alloc instead).  Even
//   std::invalid_argument(const char*) allocates internally, so a fixed
//   message would not remove this.
//
// * Versioned inline namespace: everything lives in `logic_gates::v0_2`, an
//   inline namespace, so existing `logic_gates::x` and `using namespace
//   logic_gates` callers keep compiling, while two copies of the header of
//   different versions in one program get distinct symbols instead of
//   silently violating the ODR.
//
// * Macro hygiene: internals and parameters use `lg_`-prefixed names, so a
//   user macro named check, reject, a, b, n, min, max, T, U, ... defined
//   before the include does not break the header.
// Version macro (major*10000 + minor*100 + patch).  Checked BEFORE the include
// guard: the guard name is versioned too, so a second, different version of
// this header would otherwise be silently included alongside this one.
#define LOGIC_GATES_VERSION_THIS 200
#if defined(LOGIC_GATES_VERSION) && LOGIC_GATES_VERSION != LOGIC_GATES_VERSION_THIS
#error "logic_gates.hpp: a different version of this header was already included"
#endif
#ifndef LOGIC_GATES_LOGIC_GATES_V0_2_HPP
#define LOGIC_GATES_LOGIC_GATES_V0_2_HPP
#define LOGIC_GATES_VERSION LOGIC_GATES_VERSION_THIS

#include <initializer_list>
#include <stdexcept>
#include <string>

namespace logic_gates {
inline namespace v0_2 {

namespace lg_detail {

// Not constexpr on purpose: building a message needs std::string.  A constexpr
// function may call it only on a path that is never constant-evaluated, which
// is exactly the "invalid input" path -- so evaluating it in a constant
// expression is a compile error, as intended.
//
// No pointer appears in any signature: `lg_which` is 0 for parameter "a" and 1
// for "b"; the name literal is chosen inside the body.  (lg_detail is internal
// -- not part of the API -- but even a caller who reaches it cannot hand it a
// raw pointer to read.)
[[noreturn]] inline void lg_reject(int lg_which, int lg_value) {
  throw std::invalid_argument(std::string("logic_gates: parameter '") + (lg_which == 0 ? "a" : "b") +
                              "' must be 0 or 1, got " + std::to_string(lg_value));
}

// Validate one bit: throws on anything but exactly 0 or 1.
constexpr void lg_check(int lg_which, int lg_value) {
  if (lg_value != 0 && lg_value != 1) {
    lg_reject(lg_which, lg_value);
  }
}

}  // namespace lg_detail

// ---------------------------------------------------------------------------
// The seven fundamental gates
// ---------------------------------------------------------------------------

// NOT: inverts the bit.  0 -> 1, 1 -> 0.
[[nodiscard]] constexpr int not_gate(int lg_a) {
  (lg_detail::lg_check)(0, lg_a);
  return lg_a == 0 ? 1 : 0;
}

// AND: 1 only when both inputs are 1.
[[nodiscard]] constexpr int and_gate(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return (lg_a == 1 && lg_b == 1) ? 1 : 0;
}

// OR: 1 when at least one input is 1.
[[nodiscard]] constexpr int or_gate(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return (lg_a == 1 || lg_b == 1) ? 1 : 0;
}

// XOR: 1 when the inputs differ.
[[nodiscard]] constexpr int xor_gate(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return lg_a != lg_b ? 1 : 0;
}

// NAND: NOT(AND).  0 only when both inputs are 1.  The universal gate.
[[nodiscard]] constexpr int nand_gate(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return (lg_a == 1 && lg_b == 1) ? 0 : 1;
}

// NOR: NOT(OR).  1 only when both inputs are 0.
[[nodiscard]] constexpr int nor_gate(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return (lg_a == 0 && lg_b == 0) ? 1 : 0;
}

// XNOR: NOT(XOR).  1 when the inputs are equal.
[[nodiscard]] constexpr int xnor_gate(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return lg_a == lg_b ? 1 : 0;
}

// ---------------------------------------------------------------------------
// Gates built from NAND only
// ---------------------------------------------------------------------------
// Each validates its own arguments first, then uses nothing but nand_gate.

// NOT a = NAND(a, a).
[[nodiscard]] constexpr int nand_not(int lg_a) {
  (lg_detail::lg_check)(0, lg_a);
  return nand_gate(lg_a, lg_a);
}

// AND a b = NAND(n, n) with n = NAND(a, b).
[[nodiscard]] constexpr int nand_and(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  const int lg_n = nand_gate(lg_a, lg_b);
  return nand_gate(lg_n, lg_n);
}

// OR a b = NAND(NAND(a, a), NAND(b, b)).
[[nodiscard]] constexpr int nand_or(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  return nand_gate(nand_gate(lg_a, lg_a), nand_gate(lg_b, lg_b));
}

// XOR a b via the four-NAND circuit drawn at the top of this file.
[[nodiscard]] constexpr int nand_xor(int lg_a, int lg_b) {
  (lg_detail::lg_check)(0, lg_a);
  (lg_detail::lg_check)(1, lg_b);
  const int lg_n1 = nand_gate(lg_a, lg_b);
  const int lg_n2 = nand_gate(lg_a, lg_n1);
  const int lg_n3 = nand_gate(lg_b, lg_n1);
  return nand_gate(lg_n2, lg_n3);
}

// ---------------------------------------------------------------------------
// Deleted catch-all overloads: only an exact `int` argument may be passed.
// (A non-template function wins ties against a template, so int arguments
// still reach the functions above.)  The return type is irrelevant -- these
// can never be called.
// ---------------------------------------------------------------------------
template <class LgT> [[nodiscard]] int not_gate(LgT) = delete;
template <class LgT> [[nodiscard]] int nand_not(LgT) = delete;

template <class LgT, class LgU> [[nodiscard]] int and_gate(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int or_gate(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int xor_gate(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int nand_gate(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int nor_gate(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int xnor_gate(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int nand_and(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int nand_or(LgT, LgU) = delete;
template <class LgT, class LgU> [[nodiscard]] int nand_xor(LgT, LgU) = delete;

// ---------------------------------------------------------------------------
// Deleted braced-list overloads.  A braced-init-list argument (`{true}`,
// `{c}`, `{'a'}`) gives a deduced template parameter nothing to deduce from,
// so the catch-all templates above are not viable and, without these, the
// `int` overload would accept it (bool/char -> int is not narrowing).  An
// std::initializer_list<T> parameter DOES deduce T from the braces and is an
// identity conversion, so it beats the int overload for every element type
// (including int: a braced list ranks as an initializer_list conversion, so
// even `{1}` is deleted -- pass a plain int).
// ---------------------------------------------------------------------------
template <class LgT = int> [[nodiscard]] int not_gate(std::initializer_list<LgT>) = delete;
template <class LgT = int> [[nodiscard]] int nand_not(std::initializer_list<LgT>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int and_gate(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int and_gate(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int and_gate(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int or_gate(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int or_gate(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int or_gate(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int xor_gate(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int xor_gate(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int xor_gate(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_gate(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_gate(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_gate(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nor_gate(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nor_gate(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nor_gate(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int xnor_gate(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int xnor_gate(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int xnor_gate(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_and(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_and(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_and(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_or(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_or(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_or(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_xor(std::initializer_list<LgT>, LgU) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_xor(LgT, std::initializer_list<LgU>) = delete;
template <class LgT = int, class LgU = int> [[nodiscard]] int nand_xor(std::initializer_list<LgT>, std::initializer_list<LgU>) = delete;

// An EMPTY braced list `{}` deduces nothing and would value-initialise an int
// to 0 (a valid bit, silently).  Non-template deleted overloads taking
// initializer_list<int> win that case (a list-to-initializer_list conversion
// outranks list-to-int, and a non-template wins ties with the templates).
[[nodiscard]] int not_gate(std::initializer_list<int>) = delete;
[[nodiscard]] int nand_not(std::initializer_list<int>) = delete;
[[nodiscard]] int and_gate(std::initializer_list<int>, int) = delete;
[[nodiscard]] int and_gate(int, std::initializer_list<int>) = delete;
[[nodiscard]] int and_gate(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int or_gate(std::initializer_list<int>, int) = delete;
[[nodiscard]] int or_gate(int, std::initializer_list<int>) = delete;
[[nodiscard]] int or_gate(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int xor_gate(std::initializer_list<int>, int) = delete;
[[nodiscard]] int xor_gate(int, std::initializer_list<int>) = delete;
[[nodiscard]] int xor_gate(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_gate(std::initializer_list<int>, int) = delete;
[[nodiscard]] int nand_gate(int, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_gate(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int nor_gate(std::initializer_list<int>, int) = delete;
[[nodiscard]] int nor_gate(int, std::initializer_list<int>) = delete;
[[nodiscard]] int nor_gate(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int xnor_gate(std::initializer_list<int>, int) = delete;
[[nodiscard]] int xnor_gate(int, std::initializer_list<int>) = delete;
[[nodiscard]] int xnor_gate(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_and(std::initializer_list<int>, int) = delete;
[[nodiscard]] int nand_and(int, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_and(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_or(std::initializer_list<int>, int) = delete;
[[nodiscard]] int nand_or(int, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_or(std::initializer_list<int>, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_xor(std::initializer_list<int>, int) = delete;
[[nodiscard]] int nand_xor(int, std::initializer_list<int>) = delete;
[[nodiscard]] int nand_xor(std::initializer_list<int>, std::initializer_list<int>) = delete;

}  // namespace v0_2
}  // namespace logic_gates

#endif  // LOGIC_GATES_LOGIC_GATES_V0_2_HPP
