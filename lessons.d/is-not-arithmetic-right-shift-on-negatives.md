---
category: Perl
---

# `>>` is not arithmetic right shift on negatives

`(-1 >> 7)` is a huge positive. Use `floor($x / 128.0)` from `POSIX` for signed shifts (LEB128, etc.).
