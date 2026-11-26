---
category: Perl
---

# Perl numeric != numifies strings: validate type before value

Perl's `!=` and `==` numify their operands. A validator written as
`die if $v != 0 && $v != 1` therefore accepted `"1abc"` (numifies to 1) and
treated `"a"`, `""` and `undef` as 0, with at most an "isn't numeric" warning.
In the Perl logic-gates and transistors packages this let garbage flow through
every gate as a plausible bit (CWE-20 / CWE-843). Dualvars and overloaded
objects can also answer a numeric comparison however they like.

Fix: check the type first, on the string form, with no numeric operator:
`defined $v && !ref $v && $v =~ /\A[01]\z/` (`\z`, not `$`, so `"1\n"` fails).
Do not let method-call detection depend on argument COUNT either; strip an
invocant only when it equals the package name, otherwise enforce arity.

Do instead: for any Perl input validation, write the type/shape predicate
first, test it with a matrix of "1abc", "", undef, " 1", "1\n", "0 but true",
dualvars, refs and blessed/overloaded objects, and assert no warnings fire.
