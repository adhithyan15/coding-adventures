---
category: Compiler / VM / language pipeline
---

# A sign bit alone cannot print JavaScript's -0: the minus belongs only before a zero, and no float compare is portable

**What went wrong.** The LANG84 6e plan said `console.log` would send "each
argument through `f64_signbit` and `write_f64_js`". That cannot reproduce
node: `write_f64_js` is `String(x)`, which already prints the `-` of every
negative number and prints `-0` as `0`. So the extra `-` must appear for a
negative *zero* only, and that needs a zero test. The obvious test,
`cmp_eq x, 0.0`, is not portable: the JIT refuses `cmp_*` on a float register
(interpreter fallback), and BEAM's `cmp_eq` is `is_eq_exact`, under which
OTP 27's `-0.0` and `0.0` differ.

**Fix.** Read sign bits only, as integers: `s = f64_signbit x`,
`t = f64_signbit (f64_add x, 0.0)`, and `putchar '-'` behind
`jmp_if_false (cmp_gt s, t)`. IEEE round-to-nearest makes `-0 + +0` equal
`+0` and leaves every other non-NaN value unchanged, so `s > t` holds for
`-0` alone. Literals are never `-0` (JavaScript has no negative literals), so
they skip the test, which keeps `console.log(42)` free of `f64_add` and
runnable on BEAM.

**Next time.** When a plan maps a formatting rule onto registry ops, write
the truth table for every sign and magnitude class (`-0`, `+0`, negative,
positive, `±Infinity`, NaN) before calling the plan complete, and check each
op it relies on against the JIT's and BEAM's actual lowering.
