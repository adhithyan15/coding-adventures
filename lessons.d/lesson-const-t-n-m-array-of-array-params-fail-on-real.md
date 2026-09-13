# Lesson: `const T[N][M]` array-of-array params fail on real gcc but not Apple clang

**Context:** The c/des and c/aes ports built clean locally (macOS `gcc` = Apple
clang) but FAILED on ubuntu CI's real gcc with:
`error: invalid use of pointers to arrays with different qualifiers in ISO C
before C2X [-Wpedantic]`.

**Cause:** In ISO C before C23, a `T (*)[N]` does NOT implicitly convert to
`const T (*)[N]` — `const`-ness does not compose through the array-of-array
pointer. So passing a non-const `uint8_t subkeys[16][6]` / `uint8_t state[4][4]`
local as an argument to a parameter declared `const uint8_t [16][6]` /
`const uint8_t [4][4]` is a constraint violation. (This is unlike 1-D
pointer-to-scalar, where `int* -> const int*` is fine.) Apple clang accepts it;
real gcc with `-pedantic-errors` rejects it.

**Fix:** Drop `const` on 2-D (and higher) array parameters of internal helpers
that receive a non-const array — leave the read-only intent to a comment. Grep
before pushing: `grep -rnE "const [a-z0-9_]+ [a-z_]+\[[0-9]*\]\[" src/`.

**Blind spot:** Apple clang (the macOS `gcc`) is more permissive than ubuntu
gcc on several pedantic ISO-C rules. "Verified under gcc/clang locally" does not
cover the ubuntu gcc arm — the pure-ISO guarantee is only firm once CI's gcc has
run. Watch babysit CI to green, don't assume from a local build.
