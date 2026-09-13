---
category: Perl
---

# `unpack('C*', $buf)` amplifies memory ~56× per byte

(Perl scalar header). Always validate `length($buf)` against a hard cap (64 MB is a safe default) before unpacking caller-supplied data.
