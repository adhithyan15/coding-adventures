## Unreleased — Macsyma on real CoreCLR (VM-049)

Add `tests/clr_real_macsyma.rs`: the McCarthy `clr_real_scalar.rs` pattern
retargeted at Macsyma. `compile_source_to_cil_text(Language::Macsyma, …)`
emits textual CIL that real `ilasm` assembles and real `dotnet` runs, asserted
against the same 21-program corpus `macsyma_conformance.rs`'s in-repo-simulator
CLR column already agrees on. Gated on `dotnet`+`ilasm` (skips cleanly when
absent, confirmed locally per VM-047c); a toolchain-independent
`macsyma_emits_valid_cil_text_for_full_corpus` test in the same file compiles
the whole corpus to `.il` text and asserts success unconditionally, so this
lane still exercises the real lowering path on a host with no CLR toolchain at
all. `iir-to-cil-bytecode::emit_il` needed no change: Macsyma's `call_builtin
"+"/"-"/"*"/"/ "` arithmetic is expanded to `unbox`/`add`/`box` by the shared
`iir-builtin-lowering::dynamic_arith` pass before any backend sees it, and
`emit_il` already lowers those ops for McCarthy's cons/predicate paths.
`clr_support/mod.rs::run_on_real_clr` is now a thin McCarthy-only wrapper
around the new language-generic `run_lang_on_real_clr`. The in-repo-simulator
CLR column in `macsyma_conformance.rs` is unchanged and remains the required,
always-on conformance floor.

Also fixes VM-D028: `BUILD` gained `# needs-toolchain: dotnet`. Without it, no
PR touching only `lang-aot` (bucket language "rust") ever set CI's
`needs_dotnet` flag, so the hosted `ilasm` NuGet restore never ran and the
CLR-real column — McCarthy's pre-existing one included — never actually
executed on its own PR merge-gate CI, only on a forced main-branch full build.
Confirmed via a recent merged PR's hosted job log (`needs_dotnet=false`, the
`.NET: $(dotnet --version)` verification line generated but its `if` guard
`false`). `BUILD` also adds `--test clr_real_macsyma` to the protected target
list.

