---
category: Cross-platform & Windows BUILD_windows
---

# Two spellings of one string literal are not one object; [lit, lit + n) as an iterator range crashes MSVC at /Od

`cpp/cfb-writer`'s test built a vector as
`vector(reinterpret_cast<const uint8_t*>("hello mini-stream"), reinterpret_cast<const uint8_t*>("hello mini-stream") + 17)`.
That passes pointers into two different spellings of the same literal as `[first, last)`.
GCC and Clang pool identical literals into one object, so it worked on Linux and under MinGW.
MSVC does not pool at `/Od` (the iso-harness default; `/GF` turns pooling on), and ISO C++ never promised it,
so `last - first` was a garbage huge number, the constructor threw `std::length_error` ("vector too long"),
nothing caught it, and `abort()` exited with `0xC0000409`. That is the same code as a stack-buffer
fast-fail, so it looks like memory corruption, and ASan reports nothing because there is none.

How it was found: wrap `main` with `std::set_terminate` that rethrows `current_exception()` and prints
`what()` plus a `CaptureStackBackTrace`/DbgHelp stack. The `0xC0000409` turned into "vector too long at
test line 110" in one run. `/GF-` versus `/GF` on the same source confirmed it.

Rule: an iterator or pointer range must come from ONE object (a named array, a `std::string`, a
`std::vector`), never from two evaluations of an expression that merely compares equal. Prefer
`static const uint8_t k[] = {...}; v(k, k + sizeof k)`. When a Windows-only test exits `0xC0000409`, check
for an uncaught C++ exception before suspecting the sanitizers: UCRT `abort()` and `std::terminate` both
end in `__fastfail`. (`0xC00000FD` is the other common one: a real stack overflow, here from /Od frames
against MSVC's 1 MiB default.)
