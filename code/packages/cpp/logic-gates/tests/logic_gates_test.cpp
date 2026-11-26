// Tests for logic_gates.hpp.  Dependency-free: a tiny CHECK counter, exit code
// 0 on success.  Compile-time truth tables are static_assert'ed first.
#include <climits>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include "logic_gates/logic_gates.hpp"

using namespace logic_gates;

// --- compile-time truth tables ---------------------------------------------
static_assert(not_gate(0) == 1 && not_gate(1) == 0, "NOT");
static_assert(and_gate(0, 0) == 0 && and_gate(0, 1) == 0 && and_gate(1, 0) == 0 && and_gate(1, 1) == 1, "AND");
static_assert(or_gate(0, 0) == 0 && or_gate(0, 1) == 1 && or_gate(1, 0) == 1 && or_gate(1, 1) == 1, "OR");
static_assert(xor_gate(0, 0) == 0 && xor_gate(0, 1) == 1 && xor_gate(1, 0) == 1 && xor_gate(1, 1) == 0, "XOR");
static_assert(nand_gate(0, 0) == 1 && nand_gate(0, 1) == 1 && nand_gate(1, 0) == 1 && nand_gate(1, 1) == 0, "NAND");
static_assert(nor_gate(0, 0) == 1 && nor_gate(0, 1) == 0 && nor_gate(1, 0) == 0 && nor_gate(1, 1) == 0, "NOR");
static_assert(xnor_gate(0, 0) == 1 && xnor_gate(0, 1) == 0 && xnor_gate(1, 0) == 0 && xnor_gate(1, 1) == 1, "XNOR");
static_assert(nand_not(0) == 1 && nand_not(1) == 0, "NAND NOT");
static_assert(nand_and(0, 0) == 0 && nand_and(0, 1) == 0 && nand_and(1, 0) == 0 && nand_and(1, 1) == 1, "NAND AND");
static_assert(nand_or(0, 0) == 0 && nand_or(0, 1) == 1 && nand_or(1, 0) == 1 && nand_or(1, 1) == 1, "NAND OR");
static_assert(nand_xor(0, 0) == 0 && nand_xor(0, 1) == 1 && nand_xor(1, 0) == 1 && nand_xor(1, 1) == 0, "NAND XOR");

// --- compile-time rejection of non-int argument types ----------------------
namespace {

// is_callable_N<Gate-tag, Args...> is true iff `gate(declval<Args>()...)` is
// a well-formed call.  Deleted functions make the expression ill-formed in the
// immediate context, so SFINAE turns it into `false` rather than an error.
#define LG_DEFINE_TRAIT1(NAME, FN)                                                   \
  template <class T, class = void> struct NAME : std::false_type {};                 \
  template <class T>                                                                 \
  struct NAME<T, std::void_t<decltype(FN(std::declval<T>()))>> : std::true_type {}

#define LG_DEFINE_TRAIT2(NAME, FN)                                                   \
  template <class T, class U, class = void> struct NAME : std::false_type {};        \
  template <class T, class U>                                                        \
  struct NAME<T, U, std::void_t<decltype(FN(std::declval<T>(), std::declval<U>()))>> \
      : std::true_type {}

LG_DEFINE_TRAIT1(can_not, not_gate);
LG_DEFINE_TRAIT1(can_nand_not, nand_not);
LG_DEFINE_TRAIT2(can_and, and_gate);
LG_DEFINE_TRAIT2(can_or, or_gate);
LG_DEFINE_TRAIT2(can_xor, xor_gate);
LG_DEFINE_TRAIT2(can_nand, nand_gate);
LG_DEFINE_TRAIT2(can_nor, nor_gate);
LG_DEFINE_TRAIT2(can_xnor, xnor_gate);
LG_DEFINE_TRAIT2(can_nand_and, nand_and);
LG_DEFINE_TRAIT2(can_nand_or, nand_or);
LG_DEFINE_TRAIT2(can_nand_xor, nand_xor);

// Sanity: int IS accepted (so the traits are not vacuously false).
static_assert(can_not<int>::value && can_nand_not<int>::value, "int one-arg");
static_assert(can_and<int, int>::value && can_or<int, int>::value && can_xor<int, int>::value &&
                  can_nand<int, int>::value && can_nor<int, int>::value && can_xnor<int, int>::value &&
                  can_nand_and<int, int>::value && can_nand_or<int, int>::value &&
                  can_nand_xor<int, int>::value,
              "int two-arg");

// One-argument gates reject every non-int type.
#define LG_REJECT1(T)                                                                \
  static_assert(!can_not<T>::value && !can_nand_not<T>::value, "one-arg rejects " #T)
LG_REJECT1(bool);
LG_REJECT1(char);
LG_REJECT1(signed char);
LG_REJECT1(unsigned char);
LG_REJECT1(short);
LG_REJECT1(unsigned);
LG_REJECT1(long);
LG_REJECT1(unsigned long);
LG_REJECT1(long long);
LG_REJECT1(float);
LG_REJECT1(double);

// Two-argument gates reject a non-int type in either position.
#define LG_REJECT2(T)                                                                \
  static_assert(!can_and<T, int>::value && !can_and<int, T>::value &&                \
                    !can_or<T, int>::value && !can_or<int, T>::value &&              \
                    !can_xor<T, int>::value && !can_xor<int, T>::value &&            \
                    !can_nand<T, int>::value && !can_nand<int, T>::value &&          \
                    !can_nor<T, int>::value && !can_nor<int, T>::value &&            \
                    !can_xnor<T, int>::value && !can_xnor<int, T>::value &&          \
                    !can_nand_and<T, int>::value && !can_nand_and<int, T>::value &&  \
                    !can_nand_or<T, int>::value && !can_nand_or<int, T>::value &&    \
                    !can_nand_xor<T, int>::value && !can_nand_xor<int, T>::value &&  \
                    !can_and<T, T>::value,                                           \
                "two-arg rejects " #T)
LG_REJECT2(bool);
LG_REJECT2(char);
LG_REJECT2(signed char);
LG_REJECT2(unsigned char);
LG_REJECT2(short);
LG_REJECT2(unsigned);
LG_REJECT2(long);
LG_REJECT2(unsigned long);
LG_REJECT2(long long);
LG_REJECT2(float);
LG_REJECT2(double);

}  // namespace

// --- runtime harness --------------------------------------------------------
static int g_checks = 0;
static int g_failures = 0;

#define CHECK(cond)                                                                  \
  do {                                                                               \
    ++g_checks;                                                                      \
    if (!(cond)) {                                                                   \
      ++g_failures;                                                                  \
      std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);                    \
    }                                                                                \
  } while (0)

// True iff calling f() throws std::invalid_argument; the message must mention
// the expected parameter name and the decimal value.
template <class F>
static bool throws_invalid(F f, const char* param, int value) {
  try {
    (void)f();
  } catch (const std::invalid_argument& e) {
    const std::string msg = e.what();
    return msg.find(std::string("'") + param + "'") != std::string::npos &&
           msg.find(std::to_string(value)) != std::string::npos;
  } catch (...) {
    return false;
  }
  return false;
}

// --- indirect calls: conversion happens in the CALLER ------------------------
// A function pointer `int (*)(int, int)` bypasses the deleted overloads (they
// guard DIRECT calls only).  Whatever the caller's type is, it is converted to
// int BEFORE the gate runs, so the gate can only judge the int it receives:
// valid iff that int is 0 or 1.  The helpers below assert exactly that.
namespace {
enum Plain { kPlainZero = 0, kPlainOne = 1, kPlainTwo = 2, kPlainNeg = -1 };
struct Convertible {
  int v;
  operator int() const { return v; }
};

using Fn1 = int (*)(int);
using Fn2 = int (*)(int, int);

bool valid_bit(int v) { return v == 0 || v == 1; }

// Call every gate through a pointer with the int in either position; the
// result must be a bit for valid ints and an invalid_argument otherwise.
void expect_indirect(int v, int line) {
  const Fn1 one[] = {&not_gate, &nand_not};
  const Fn2 two[] = {&and_gate, &or_gate,   &xor_gate,  &nand_gate, &nor_gate,
                     &xnor_gate, &nand_and, &nand_or,   &nand_xor};
  for (Fn1 f : one) {
    bool threw = false;
    int r = -1;
    try { r = f(v); } catch (const std::invalid_argument&) { threw = true; }
    ++g_checks;
    if (threw == valid_bit(v) || (!threw && !valid_bit(r))) {
      ++g_failures;
      std::printf("FAIL line %d: indirect 1-arg v=%d\n", line, v);
    }
  }
  for (Fn2 f : two) {
    for (int pos = 0; pos < 2; ++pos) {
      bool threw = false;
      int r = -1;
      try { r = pos == 0 ? f(v, 1) : f(1, v); } catch (const std::invalid_argument&) { threw = true; }
      ++g_checks;
      if (threw == valid_bit(v) || (!threw && !valid_bit(r))) {
        ++g_failures;
        std::printf("FAIL line %d: indirect 2-arg v=%d pos=%d\n", line, v, pos);
      }
    }
  }
}
}  // namespace

static void indirect_matrix() {
  // Types the deleted overloads cannot see through a pointer (LG02 section 9).
  expect_indirect(kPlainZero, __LINE__);
  expect_indirect(kPlainOne, __LINE__);
  expect_indirect(kPlainTwo, __LINE__);
  expect_indirect(kPlainNeg, __LINE__);
  expect_indirect(static_cast<int>(L'\0'), __LINE__);  // wchar_t
  expect_indirect(static_cast<int>(L'\1'), __LINE__);
  expect_indirect(static_cast<int>(L'\2'), __LINE__);
  expect_indirect(static_cast<int>(char16_t{2}), __LINE__);
  expect_indirect(static_cast<int>(char32_t{2}), __LINE__);
  expect_indirect(static_cast<int>(char32_t{0x10FFFF}), __LINE__);
  expect_indirect(static_cast<int>(static_cast<unsigned short>(65535)), __LINE__);
  expect_indirect(static_cast<int>(static_cast<unsigned short>(2)), __LINE__);
  // Class with operator int: the user-defined conversion runs in the caller.
  {
    const Fn2 f = &and_gate;
    const Convertible two{2}, one{1};
    bool threw = false;
    try { (void)f(two, 1); } catch (const std::invalid_argument&) { threw = true; }
    CHECK(threw);
    CHECK(f(one, 1) == 1);
  }
  // Wide integers: narrowing to int is implementation-defined and performed
  // by the CALLER.  2^32+1 narrows to 1 on every supported target (a VALID
  // bit: the gate cannot know), 2^64-1 narrows to -1 (rejected).  So the
  // assertion is "the gate agrees with the int it was given".
  const unsigned long long wide[] = {4294967297ULL, 18446744073709551615ULL, 4294967296ULL,
                                     4294967298ULL, 2147483648ULL};
  for (unsigned long long w : wide) expect_indirect(static_cast<int>(w), __LINE__);
  CHECK(static_cast<int>(4294967297ULL) == 1);
  CHECK(static_cast<int>(18446744073709551615ULL) == -1);
}

// Messages are byte-identical to the original contract, for every gate and
// both parameters.
static std::string msg_of(int (*f)(int, int), int x, int y) {
  try { (void)f(x, y); } catch (const std::invalid_argument& e) { return e.what(); }
  return "<no throw>";
}
static std::string msg1(int (*f)(int), int x) {
  try { (void)f(x); } catch (const std::invalid_argument& e) { return e.what(); }
  return "<no throw>";
}
static void message_matrix() {
  const std::string pa = "logic_gates: parameter 'a' must be 0 or 1, got ";
  const std::string pb = "logic_gates: parameter 'b' must be 0 or 1, got ";
  const int vals[] = {2, -1, 1024, INT_MIN, INT_MAX};
  for (int v : vals) {
    CHECK(msg1(&not_gate, v) == pa + std::to_string(v));
    CHECK(msg1(&nand_not, v) == pa + std::to_string(v));
    const Fn2 two[] = {&and_gate, &or_gate, &xor_gate, &nand_gate, &nor_gate,
                       &xnor_gate, &nand_and, &nand_or, &nand_xor};
    for (Fn2 f : two) {
      CHECK(msg_of(f, v, 1) == pa + std::to_string(v));
      CHECK(msg_of(f, 1, v) == pb + std::to_string(v));
      CHECK(msg_of(f, v, v) == pa + std::to_string(v));
    }
  }
}

static std::vector<int> bad_values() {
  std::vector<int> v;
  for (int i = -1024; i <= 1024; ++i) {
    if (i != 0 && i != 1) v.push_back(i);
  }
  v.push_back(INT_MIN);
  v.push_back(INT_MIN + 1);
  v.push_back(INT_MAX - 1);
  v.push_back(INT_MAX);
  return v;
}

int main() {
  // Literal truth tables, rows ordered (a,b) = 00, 01, 10, 11.
  struct Row2 { int a, b, and_, or_, xor_, nand_, nor_, xnor_; };
  const Row2 rows[] = {
      {0, 0, 0, 0, 0, 1, 1, 1},
      {0, 1, 0, 1, 1, 1, 0, 0},
      {1, 0, 0, 1, 1, 1, 0, 0},
      {1, 1, 1, 1, 0, 0, 0, 1},
  };
  for (const Row2& r : rows) {
    CHECK(and_gate(r.a, r.b) == r.and_);
    CHECK(or_gate(r.a, r.b) == r.or_);
    CHECK(xor_gate(r.a, r.b) == r.xor_);
    CHECK(nand_gate(r.a, r.b) == r.nand_);
    CHECK(nor_gate(r.a, r.b) == r.nor_);
    CHECK(xnor_gate(r.a, r.b) == r.xnor_);
    // NAND-built equal direct.
    CHECK(nand_and(r.a, r.b) == and_gate(r.a, r.b));
    CHECK(nand_or(r.a, r.b) == or_gate(r.a, r.b));
    CHECK(nand_xor(r.a, r.b) == xor_gate(r.a, r.b));
    CHECK(nand_and(r.a, r.b) == r.and_);
    CHECK(nand_or(r.a, r.b) == r.or_);
    CHECK(nand_xor(r.a, r.b) == r.xor_);
  }
  CHECK(not_gate(0) == 1);
  CHECK(not_gate(1) == 0);
  CHECK(nand_not(0) == 1);
  CHECK(nand_not(1) == 0);
  CHECK(nand_not(0) == not_gate(0));
  CHECK(nand_not(1) == not_gate(1));

  // No valid input throws (also exercises the non-constexpr runtime path).
  {
    bool threw = false;
    try {
      for (int a = 0; a <= 1; ++a) {
        (void)not_gate(a);
        (void)nand_not(a);
        for (int b = 0; b <= 1; ++b) {
          (void)and_gate(a, b); (void)or_gate(a, b); (void)xor_gate(a, b);
          (void)nand_gate(a, b); (void)nor_gate(a, b); (void)xnor_gate(a, b);
          (void)nand_and(a, b); (void)nand_or(a, b); (void)nand_xor(a, b);
        }
      }
    } catch (...) {
      threw = true;
    }
    CHECK(!threw);
  }

  // Exhaustive rejection in every argument position, other argument valid
  // (0 and 1) and invalid.
  const std::vector<int> bad = bad_values();
  for (int x : bad) {
    CHECK(throws_invalid([&] { return not_gate(x); }, "a", x));
    CHECK(throws_invalid([&] { return nand_not(x); }, "a", x));

#define REJECT_BINARY(FN)                                                            \
  do {                                                                               \
    for (int ok = 0; ok <= 1; ++ok) {                                                \
      CHECK(throws_invalid([&] { return FN(x, ok); }, "a", x));                      \
      CHECK(throws_invalid([&] { return FN(ok, x); }, "b", x));                      \
    }                                                                                \
    CHECK(throws_invalid([&] { return FN(x, x); }, "a", x));                         \
  } while (0)
    REJECT_BINARY(and_gate);
    REJECT_BINARY(or_gate);
    REJECT_BINARY(xor_gate);
    REJECT_BINARY(nand_gate);
    REJECT_BINARY(nor_gate);
    REJECT_BINARY(xnor_gate);
    REJECT_BINARY(nand_and);
    REJECT_BINARY(nand_or);
    REJECT_BINARY(nand_xor);
  }
  // Both invalid with different values: first argument reported first.
  CHECK(throws_invalid([] { return and_gate(5, 7); }, "a", 5));
  CHECK(throws_invalid([] { return nand_xor(2, -3); }, "a", 2));

  indirect_matrix();
  message_matrix();

  std::printf("logic_gates tests: %d checks, %d failures\n", g_checks, g_failures);
  return g_failures == 0 ? 0 : 1;
}
