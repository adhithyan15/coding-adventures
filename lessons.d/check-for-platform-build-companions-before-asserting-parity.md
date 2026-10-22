---
category: Repo policy / workflow reminders
---

# Check for platform BUILD companions before asserting parity

A validation command assumed every Rust package had both `BUILD` and
`BUILD_windows`, but `der-asn1` only has `BUILD`; the nonexistent comparison
failed before the real package build ran. Inspect the package's checked-in
build surfaces first, then run parity checks only for companions that exist.
