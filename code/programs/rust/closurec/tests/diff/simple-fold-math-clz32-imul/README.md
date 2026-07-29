# simple-fold-math-clz32-imul

Locks the byte output of SIMPLE-level static `Math.clz32(n)` / `Math.imul(a,b)`
folding (`closure-pass-constant-fold`). `clz32` counts leading zero bits of
`ToUint32(n)` (0..32); `imul` is the 32-bit signed product of `ToUint32(a)` and
`ToUint32(b)`. Both are pure modular integer operations (no libm), so the fold is
bit-exact and verified byte-identical to the reference Closure Compiler
(`closure-compiler-v20260712.jar`, SIMPLE, `--language_out NO_TRANSPILE`):

- `Math.clz32(1)` -> `31`, `Math.clz32(0)` -> `32`, `Math.clz32(-1)` -> `0`
- `Math.imul(3,4)` -> `12`, `Math.imul(-1,5)` -> `-5`, `Math.imul(65536,65536)` -> `0`

Declined (left intact), matching the reference: a non-literal argument
(`Math.clz32(x)`) and a non-global receiver (`m.imul(2,3)`). See `input/a.js` and
`expected.stdout`.
