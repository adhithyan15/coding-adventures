## 0.1.1

- **Security fix.** `ctSelectBytes` built its mask with `if choice then ...`, a
  branch on the secret bit that CT01 forbids. The bool's stored byte is now read
  with `Unsafe.As<bool, byte>` and normalised to 0x00/0xFF arithmetically.
- Tests for null rejection, length mismatch, non-0/1 stored bools and
  fresh-copy output (coverage 100% of lines).
