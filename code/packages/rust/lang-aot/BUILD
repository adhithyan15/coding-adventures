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
# ONE COMMAND, ONE LINE — DO NOT REFORMAT WITH BACKSLASH CONTINUATIONS. The
# build tool reads a BUILD file with `readLines` (build-tool/internal/discovery),
# which splits on newlines and runs EACH LINE as its own `sh -c`. A trailing `\`
# is not honoured: the first revision of this file wrapped the target list over
# 40 lines, so CI silently ran only `cargo test -p lang-aot --lib` (15 tests) and
# then failed ~40 bogus commands with `sh: --: invalid option`. It looked like
# the suites were being watched when none of them were.
#
# Every CI-green test target is explicitly protected. Keep this list complete: an
# omitted target still compiles in the check step, but its assertions do not run
# in CI. A bare `cargo test -p lang-aot` would make an accidental omission harder
# to review.
#
# ONE FULL TARGET IS DELIBERATELY EXCLUDED because it is red today:
#
#   lang_matrix          ALGOL-owned native-AOT four-dimensional-array cases
#                        remain red on Linux (VM-D005). Its focused non-ALGOL
#                        BASIC/libm differential is protected separately below.
cargo test -p lang-aot --lib --test array_ref_tracing --test beam_cons --test beam_emit --test beam_predicates --test beam_symbols_lambda --test cil_cons --test cil_emit --test cil_lambda --test cil_predicates --test cil_symbols --test clr_real_cons --test clr_real_lambda --test clr_real_predicates --test clr_real_scalar --test clr_real_symbols --test conformance --test e6d2b_dynamic_arith --test end_to_end_smoke --test e6d6_llvm_records --test e6d6b_union_match_tagged --test e6d7a_wasm_closures --test historical_round_trip --test jit_mccarthy --test jvm_cons --test jvm_emit --test jvm_lambda --test jvm_predicates --test jvm_symbols --test llvm_cond --test llvm_cons --test llvm_gc_completion --test llvm_gc_write_barrier --test llvm_lambda --test llvm_predicates --test llvm_scalar --test llvm_symbols --test macos_native_lisp --test macsyma_conformance --test metacircular --test vm_gc_reclamation --test wasm_emit --test wasm_memory_growth
cargo test -p lang-aot --test lang_matrix t7_differential_random_basic_
