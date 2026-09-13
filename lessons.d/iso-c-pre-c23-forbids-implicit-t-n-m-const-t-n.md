# ISO C (pre-C23) forbids implicit `T[N][M]` → `const T[N][M]`; clang allows it, gcc `-pedantic-errors` rejects it

**Symptom:** A pure-ISO C package built clean on macOS (Apple clang) but failed
on ubuntu CI (real gcc) with `-pedantic-errors`:
`invalid use of pointers to arrays with different qualifiers in ISO C before C2X
[-Wpedantic]` at every call site passing a **non-const** 2-D array to a function
whose parameter is `const double m[3][3]`.

**Cause:** For a plain pointer, `T*` → `const T*` is a permitted implicit
conversion. For a pointer to an *array*, `T(*)[N]` → `const T(*)[N]` is **not**
permitted in ISO C before C23 (C11/C17 6.3.2.3 only qualifies the immediately
pointed-to type, and a 2-D array parameter decays to `double(*)[3]`, so the const
lands one level too deep). C23/C2X finally allows it. **Clang does not diagnose
this even under `-pedantic-errors`; real gcc does.** So a Mac-only local build
(where `gcc` is an Apple-clang shim) cannot catch it — only ubuntu CI will.

**Fix (caller side, minimal):** make every *input-only* array `const` at its
definition so the call passes `const → const` (identical qualifiers, always
clean); for a genuinely non-const array that is also read as input (e.g. an
out-param from an earlier call reused as input), add an explicit
`(const double (*)[3])` cast at the call. Do **not** drop the `const` from the
library parameter — that just flips the error onto the const fixtures (`ID`,
`SWAP`) instead.

**Blind spot / how to catch it:** any C API that takes a multi-dimensional array
by `const` parameter is a portability trap you cannot verify on a clang-only
machine. When writing such tests, declare read-only matrix fixtures `const` from
the start. Treat ubuntu-gcc CI as the source of truth for pure-ISO C conformance,
not the local build. Cross-ref [[feedback_no_third_party_ffi]] and the C/C++
lane's strict-ISO intent.
