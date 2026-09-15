## 0.44.0 — 2026-06-10 — McCarthy **cross-backend conformance suite** — **THE PLATFORM MATRIX IS COMPLETE** (LANG77 / W16)

New `tests/conformance.rs`: one shared table of **19** McCarthy programs (F1–F7)
run through **all eight backends** — VM (`mccarthy_lisp_vm`), JIT
(`run_mccarthy_on_jit`), WASM (`wasm-runtime`), CLR (`clr-simulator`), JVM (real
`java`), BEAM (real `erl`), LLVM (`clang` + `lispy_runtime.c`), native AOT (system
`ld`) — each asserting the **identical** integer result. The four pure-in-process
backends (VM/JIT/WASM/CLR) always run (the conformance floor); the external-tool
backends skip gracefully when their tool is absent, so CI proves uniformity across
whatever is installed. On a fully-equipped host all eight agree on all 19 programs.
New dev-deps `mccarthy-lisp-vm` + `iir-to-jvm-class-file`. **This is the capstone:
one McCarthy source, eight independent code generators, three value models, one
answer — the proof the platform is complete and uniform.**

