# A recursion depth cap sized for an 8 MB stack still overflows Windows' 1 MB stack

The C backend guards cyclic-structure display (`_sir_fmt`) and equality
(`_sir_value_eq_d`) with a depth cap — past the cap it prints `[...]` / assumes
equal, so `h[0] = h` terminates instead of recursing forever. The cap was `5000`,
commented "comfortably under the stack limit." It was: on Linux/macOS (8 MB
stack) 5000 frames (~875 KB on the display path) fit fine, so CI and the sibling
cyclic-**sequence** test both passed. But Windows' default stack is **1 MB**, and
5000 frames overran it **before** the cap could trip — so the very guard meant to
prevent the overflow was unreachable there. Symptom: `cyclic_map_does_not_stack_
overflow` failed ONLY on Windows, exit `0xC00000FD` (STACK_OVERFLOW / -1073741571).

Lessons:
1. **A depth/recursion cap must be sized for the SMALLEST stack the artifact can
   run on, not the dev box.** Windows 1 MB is the floor for emitted C/Rust; pick
   caps (here lowered to `500`, ~90 KB) with margin for an *unoptimised* build
   (debug frames are 2–3× larger than the `-O` frames you might estimate from).
2. **"Passes in CI" can mean "CI's stack is bigger," not "correct."** A
   platform-dependent resource limit (stack, fd count, path length, default int
   width) makes a bug invisible on the CI OS. When a guard is about a resource
   limit, test against the tightest limit — or assert the guard *fires* (reaches
   the `[...]` branch), not just that the program happens not to crash.
3. `0xC00000FD` / exit `-1073741571` on Windows == stack overflow; suspect
   unbounded (or insufficiently-bounded) recursion.
