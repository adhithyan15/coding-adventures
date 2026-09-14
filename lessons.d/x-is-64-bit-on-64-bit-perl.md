---
category: Perl
---

# `~$x` is 64-bit on 64-bit Perl

Always mask: `(~$x) & 0xFFFFFFFF` for 32-bit arithmetic (MD5, bitsets).
