---
category: Compiler / VM / language pipeline
---

# Windows LLVM byte output needs binary CRT stdout before the first write

PR #18838 passed on Linux and macOS but the Windows LLVM conformance run wrote
`00 0D 0A 0D` where the raw-byte literal required `00 0A 0D`. The Windows CRT
opens stdout in text mode, and even `fwrite` translates LF to CRLF. The native
runtime's lazy binary-mode setup did not help modules that only linked the
LLVM print shim, or called libc `putchar` directly.

Emit a target-specific LLVM module constructor selecting `_setmode(1, 0x8000)`
before the first output. Cover every output primitive, MSVC and MinGW triples,
and a program with no string helper linked. Keep exact raw-byte comparisons;
normalizing captured newlines would conceal the broken language contract.
