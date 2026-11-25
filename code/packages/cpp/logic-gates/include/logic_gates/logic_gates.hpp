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
// * No pointers, arrays, allocation, I/O, globals, or undefined behaviour.
#ifndef LOGIC_GATES_LOGIC_GATES_HPP
#define LOGIC_GATES_LOGIC_GATES_HPP

#include <stdexcept>
#include <string>

namespace logic_gates {

namespace detail {

// Not constexpr on purpose: building a message needs std::string.  A constexpr
// function may call it only on a path that is never constant-evaluated, which
// is exactly the "invalid input" path -- so evaluating it in a constant
// expression is a compile error, as intended.
[[noreturn]] inline void reject(const char* param, int value) {
  throw std::invalid_argument(std::string("logic_gates: parameter '") + param +
                              "' must be 0 or 1, got " + std::to_string(value));
}

// Validate one bit: throws on anything but exactly 0 or 1.
constexpr void check(const char* param, int value) {
  if (value != 0 && value != 1) {
    reject(param, value);
  }
}

}  // namespace detail

// ---------------------------------------------------------------------------
// The seven fundamental gates
// ---------------------------------------------------------------------------

// NOT: inverts the bit.  0 -> 1, 1 -> 0.
constexpr int not_gate(int a) {
  detail::check("a", a);
  return a == 0 ? 1 : 0;
}

// AND: 1 only when both inputs are 1.
constexpr int and_gate(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return (a == 1 && b == 1) ? 1 : 0;
}

// OR: 1 when at least one input is 1.
constexpr int or_gate(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return (a == 1 || b == 1) ? 1 : 0;
}

// XOR: 1 when the inputs differ.
constexpr int xor_gate(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return a != b ? 1 : 0;
}

// NAND: NOT(AND).  0 only when both inputs are 1.  The universal gate.
constexpr int nand_gate(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return (a == 1 && b == 1) ? 0 : 1;
}

// NOR: NOT(OR).  1 only when both inputs are 0.
constexpr int nor_gate(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return (a == 0 && b == 0) ? 1 : 0;
}

// XNOR: NOT(XOR).  1 when the inputs are equal.
constexpr int xnor_gate(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return a == b ? 1 : 0;
}

// ---------------------------------------------------------------------------
// Gates built from NAND only
// ---------------------------------------------------------------------------
// Each validates its own arguments first, then uses nothing but nand_gate.

// NOT a = NAND(a, a).
constexpr int nand_not(int a) {
  detail::check("a", a);
  return nand_gate(a, a);
}

// AND a b = NAND(n, n) with n = NAND(a, b).
constexpr int nand_and(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  const int n = nand_gate(a, b);
  return nand_gate(n, n);
}

// OR a b = NAND(NAND(a, a), NAND(b, b)).
constexpr int nand_or(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  return nand_gate(nand_gate(a, a), nand_gate(b, b));
}

// XOR a b via the four-NAND circuit drawn at the top of this file.
constexpr int nand_xor(int a, int b) {
  detail::check("a", a);
  detail::check("b", b);
  const int n1 = nand_gate(a, b);
  const int n2 = nand_gate(a, n1);
  const int n3 = nand_gate(b, n1);
  return nand_gate(n2, n3);
}

// ---------------------------------------------------------------------------
// Deleted catch-all overloads: only an exact `int` argument may be passed.
// (A non-template function wins ties against a template, so int arguments
// still reach the functions above.)  The return type is irrelevant -- these
// can never be called.
// ---------------------------------------------------------------------------
template <class T> int not_gate(T) = delete;
template <class T> int nand_not(T) = delete;

template <class T, class U> int and_gate(T, U) = delete;
template <class T, class U> int or_gate(T, U) = delete;
template <class T, class U> int xor_gate(T, U) = delete;
template <class T, class U> int nand_gate(T, U) = delete;
template <class T, class U> int nor_gate(T, U) = delete;
template <class T, class U> int xnor_gate(T, U) = delete;
template <class T, class U> int nand_and(T, U) = delete;
template <class T, class U> int nand_or(T, U) = delete;
template <class T, class U> int nand_xor(T, U) = delete;

}  // namespace logic_gates

#endif  // LOGIC_GATES_LOGIC_GATES_HPP
