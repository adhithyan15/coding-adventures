# CodingAdventures.CtCompare.CSharp

Constant-time byte equality, fixed-size equality alias, mask-based byte
selection, and branch-free 64-bit equality helpers.

## Null handling

Every operation has array/array, array/span, span/array and span/span forms.
Whenever an argument is a `byte[]`, a `null` array throws
`ArgumentNullException`; it is never treated as an empty buffer, which once let
a missing value compare equal to an empty one.
