# Lesson 93 — `unpack('C*', ...)` in Perl amplifies memory before any size check

**Date:** 2026-04-26

**What happened:** The Perl ZStd `decompress` function called `my @data = unpack('C*', $input)` on the raw compressed bytes as its very first step, converting each byte into a full Perl scalar. A Perl scalar occupies ~56 bytes on 64-bit builds (SV header + IV/PV storage). A 64 MB compressed input therefore expands to ~3.5 GB of Perl scalars on the heap before any frame-header validation or size guard could fire — a classic unpack memory amplification attack.

**Rule:** In Perl, never `unpack('C*', ...)` a caller-supplied buffer without first checking its length:

```perl
die "input too large" if length($data) > 64 * 1024 * 1024;
my @bytes = unpack('C*', $data);
```

64 MB is a safe upper bound for all realistic ZStd frames (the compressor's MAX_BLOCK_SIZE is 128 KB). The same pattern applies to any language where unpacking bytes into an array of objects/scalars multiplies memory by a large constant factor. Always validate the *raw byte count* before the amplifying operation, not just the logical content-size field inside the frame.

---
