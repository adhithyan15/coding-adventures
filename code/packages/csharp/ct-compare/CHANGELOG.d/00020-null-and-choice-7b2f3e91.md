## 0.2.0

- **Security fix.** A `null` `byte[]` converted implicitly to an *empty*
  `ReadOnlySpan<byte>`, so a missing value compared equal to an empty one. New
  `byte[]` overloads of `CtEq`, `CtEqFixed` and `CtSelectBytes` are chosen for
  array arguments and throw `ArgumentNullException` on `null`.
- **Security fix.** `CtSelectBytes` built its mask with `choice ? 1 : 0`, a
  branch on the secret bit that CT01 forbids. The bool's stored byte is now read
  with `Unsafe.As<bool, byte>` and normalised to 0x00/0xFF arithmetically.
- Regression tests for null rejection, overload agreement, non-0/1 stored
  bools, and fresh-copy output.
