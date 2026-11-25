---
category: Cross-platform & Windows BUILD_windows
---

# Unsigned wraparound should avoid unary negation under MSVC warnings-as-errors

The C++ sampler computed a rejection threshold with unary minus on a
`std::uint64_t`. Unsigned wraparound is defined, and GCC accepted it, but MSVC
reported C4146 and the package treats that warning as an error.

Spell the same modulo arithmetic as `std::uint64_t{0} - value`. Keep the
warning-as-error flags enabled and retain a fixed-seed schedule assertion so
the portability edit cannot silently change pair order.
