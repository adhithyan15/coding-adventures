# `lang-aot` had NO BUILD file, so the build tool never ran any of its tests in
# CI. That is how three red suites reached main unnoticed — including two that
# only turned up by running all 42 targets by hand.
#
# This runs the green ones now rather than waiting for the whole matrix to go
# green, so McCarthy's nine-backend uniformity and the ~30 per-backend suites are
# protected from here on. Every suite that needs an external toolchain
# (clang / java / dotnet+ilasm / erl / a macOS linker) gates on its presence and
# SKIPS when absent, so a CI host without them reports skips, not failures; the
# in-process suites (VM / JIT / WASM) always run.
#
# TWO TARGETS ARE DELIBERATELY EXCLUDED because they are red today. Each is
# tracked, and each should be added here the moment it goes green — an excluded
# target is an unprotected one:
#
#   lang_matrix           12 confirmed cross-language cell failures (the
#                         matrix-repair campaign). Add when it reaches zero.
#   e6d7a_wasm_closures   2 failures — Twig closures on wasm (unbox width).
#
# EXCLUSION ONLY SKIPS THE RUN, NOT THE BUILD. The check step compiles every
# target in the crate with `-D warnings`, so an excluded suite that fails to
# COMPILE still fails CI — only its failing assertions are excluded. This bit
# the first revision of this file: `end_to_end_smoke` (excluded here at the
# time) broke the macOS build on an unused import. Exclusion buys time on a
# red assertion; it buys nothing on a red compile.
#
# Listing targets explicitly rather than running the whole crate is what makes
# the exclusions visible. A bare `cargo test -p lang-aot` would hide them.
cargo test -p lang-aot --lib \
  --test array_ref_tracing \
  --test beam_cons \
  --test beam_emit \
  --test beam_predicates \
  --test beam_symbols_lambda \
  --test cil_cons \
  --test cil_emit \
  --test cil_lambda \
  --test cil_predicates \
  --test cil_symbols \
  --test clr_real_cons \
  --test clr_real_lambda \
  --test clr_real_predicates \
  --test clr_real_scalar \
  --test clr_real_symbols \
  --test conformance \
  --test e6d2b_dynamic_arith \
  --test end_to_end_smoke \
  --test e6d6_llvm_records \
  --test e6d6b_union_match_tagged \
  --test historical_round_trip \
  --test jit_mccarthy \
  --test jvm_cons \
  --test jvm_emit \
  --test jvm_lambda \
  --test jvm_predicates \
  --test jvm_symbols \
  --test llvm_cond \
  --test llvm_cons \
  --test llvm_gc_completion \
  --test llvm_gc_write_barrier \
  --test llvm_lambda \
  --test llvm_predicates \
  --test llvm_scalar \
  --test llvm_symbols \
  --test macos_native_lisp \
  --test metacircular \
  --test vm_gc_reclamation \
  --test wasm_emit \
  --test wasm_memory_growth
