---
category: Perl
---

# A Perl hash key or a single-element sort result is a string, never an Int, so it fails the IOK check of an integer-typed API

The Perl fuzz engine (VER06q) kept the edges one execution reported as the keys of a hash (a set) and
returned `[ sort { $a <=> $b } keys %set ]`.  Every campaign test passed except the one whose target
reported exactly **one** edge, which died with `edge must be an Integer`.

Hash keys are always strings.  `sort { $a <=> $b }` numifies the elements it compares, and that sets the
numeric flag on those scalars, so a list of two or more keys *became* integers as a side effect of being
sorted; a list of one is never compared, so its only element stayed a plain string.  The package's `Int`
contract (the IOK flag, so a float or a numeric string can never reach the PRNG) then refused it.  The
bug was invisible to every test with more than one edge, and it was only ever "fixed" by accident.

Fix: convert explicitly where the keys leave the set, `[ map { 0 + $_ } sort { $a <=> $b } keys %set ]`.

Do differently: never rely on a comparison to give a value its type.  When an API checks the *type* of a
Perl scalar by flags, convert at the boundary where the value is produced, and add the one-element and
zero-element cases of every collection to the tests (the empty and singleton cases are where Perl's
implicit conversions do not happen).
