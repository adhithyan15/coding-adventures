## 0.288.0 - 2026-09-01 (Dartmouth BASIC portable RND parity)

The unified matrix now executes Dartmouth BASIC's deterministic `RND` contract
on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. One program proves negative
reseeding, positive advancement, zero-argument repeatability, and the next
advance through stable integer buckets `22`, `85032`, `85032`, and `601352`.

The frontend implements a Park–Miller generator through the already-shared
typed module-global substrate, so no backend receives a BASIC-specific random
runtime or host entropy dependency.

