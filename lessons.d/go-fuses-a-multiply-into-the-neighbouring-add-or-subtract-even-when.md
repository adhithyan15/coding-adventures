---
category: Testing & coverage
---

# Go fuses a multiply into the neighbouring add or subtract even when the multiply is a division by a power of two

Porting a bit-exact float algorithm to Go (TRIG-CONV TC-6), the arm64 assembly check
(`GOARCH=arm64 go build -gcflags=-S . 2>&1 | grep FMADD\|FMSUB\|FNMSUB`) still found one
`FNMSUBD` after every `x*y + z` had been wrapped in `float64(...)`. It was the Newton step
`next := (guess + q) / 2.0; improvement := next - guess`: the compiler rewrites `/ 2.0` as
`* 0.5` and then fuses that product into the subtraction. Numerically harmless (halving is
exact) but it fails a "zero FMA" gate, and a gate you cannot trust to be zero is worthless.
Wrap it too: `next := float64((guess + q) / 2.0)`.

What to do differently: do not trust a reading of the source for "no fusable sites". Count
the fused instructions in the cross-compiled assembly (`arm64`, `ppc64le`, `s390x`,
`riscv64`, and `GOAMD64=v3`), run a negative control (strip the `float64()` wrappers and
confirm the grep now finds dozens), and make the check fail when the build itself fails
(capture the output first; a bare `grep -c` after a failed build also reports 0).
