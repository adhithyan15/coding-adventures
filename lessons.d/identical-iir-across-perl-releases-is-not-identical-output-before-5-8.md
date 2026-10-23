---
category: Perl
---

# Identical IIR across Perl releases is not identical output: before 5.8 Perl computes in doubles and prints a negative zero by platform

Phase 4a showed that all 118 checked Perl release pairs lower the integer
`print` subset to identical IIR. It is tempting to read that as "identical
behaviour", so that the host `perl` v5.38.2 oracle covers Perl 1.000 as well.
It does not. Before PERL_PRESERVE_IVUV (5.8), Perl computes `+`, `-`, `*`
and unary minus in C doubles, so `-0` and `0 * -3` are the IEEE `-0.0`. How
that prints depends on the release and the platform. Perl 1–4 use `%.20g`,
which glibc prints as `-0`, and treat zero specially only `#ifdef apollo`.
5.000 does not fold constants to integers. 5.001–5.004 fold an integral
double to an IV. 5.005 stops doing that for `OP_NEGATE`. `FIXNEGATIVEZERO`
exists only in hints files. Host perl 5.38 prints `0` in every case.

The fix (4c) is a `ReleaseProfile` switch, `Arithmetic::Double` versus
`Integer`. Under `Double` the compiler tracks each value's IEEE zero sign and
refuses to print a negative zero. A corpus case that prints one carries
`.reject@perl_nv`. The IIR stays identical. Only acceptance changes.

What to do differently: before extending a modern-interpreter oracle to old
releases through IIR identity, list each construct's runtime semantics
(number representation, formatting, folding) and read the old release's
own source for them (`str.c`, `sv.c`, `pp.c`, `op.c` at the tag). A grammar
that parses the program, and a compiler that emits the same IIR, say
nothing about what the old interpreter printed.
