## 0.81.0 — 2026-06-14 — real Brainfuck programs run cross-backend (LANG-FULL B1)

`tests/lang_matrix.rs` gains two executed Brainfuck programs beyond the existing
1-loop "print A": a **nested-loop** multiply-by-repeated-addition program
(`++++++++>+++++++++<[>[->+>+<<]>>[-<<+>>]<<<-]>>.-------.`) that computes 8 × 9 = 72
then adjusts to 65 → stdout **"HA"**, and a two-sequential-loop program → **"OK"**.
Both run across native / LLVM / WASM / JVM / CLR / VM / JIT. This proves the backends
lower **nested loops + multi-cell pointer movement + multiple `putchar`s**, not just a
single loop printing one char — converting the biggest Brainfuck smoke-test gap (only
the trivial "A" ran cross-backend) into real coverage. Test-harness only; Brainfuck's
semantics were already complete, so no `brainfuck-iir-compiler` change.

