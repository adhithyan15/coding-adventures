---
category: Compiler / VM / language pipeline
---

# Integer narrowing checks should make the fixed transport range explicit

The Go STP01 reader parsed signed 64-bit integers and compared them with an int
parameter (one for schema version or 255 for bytes) before conversion. Native
boundary tests passed, but CodeQL check117222868684 on PR17677 could not prove
that parameter-based upper bound across the conversion and reported a high
integer-conversion finding at stp01_reader_test.go:223.

These fields share a fixed byte-sized envelope. Parse with signed16-bit range
so 255 remains representable, reject values below zero or above literal255,
then apply the narrower field-specific maximum before converting to int.
This keeps the accepted domain unchanged, including JSON integer -0 where zero
is legal, and gives both readers and static analysis an explicit width proof.
Retain oversized and huge-integer rejection tests; native tests alone do not
replace the exact-head static-analysis check.
