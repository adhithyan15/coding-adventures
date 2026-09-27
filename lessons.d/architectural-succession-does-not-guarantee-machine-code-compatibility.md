---
category: Compiler / VM / language pipeline
---

# Architectural succession does not guarantee machine-code compatibility

OCT00 said an Intel 8008 Oct binary could run unchanged on the successor 8080.
That claim followed the machines' shared 8-bit concepts without checking their
instruction bytes. The repository's 8008 encoder emits `0x7C` for `JMP`, while
the 8080 decoder reads `0x7C` as `MOV A,H`; 8080 `JMP` is `0xC3`. Some bytes,
such as `HLT` `0x76`, happen to match and make a single-opcode smoke test
misleading.

When extending a language to a related processor, state source portability
separately from binary compatibility. Compare at least one control-flow opcode
in both encoders or decoders, generate target-specific code, and execute a
branching program on each simulator before claiming the port works.
