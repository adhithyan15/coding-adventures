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
# VM-049/VM-D028: `lang-aot`'s own bucket language is "rust", so before this
# line existed no PR touching only this crate ever set `needs_dotnet`, and
# `.github/workflows/ci.yml` only installs the real .NET SDK and restores the
# `ilasm` NuGet package (the CLR-real assembler `clr_real_*.rs`/
# `clr_real_macsyma.rs` need — see `tests/clr_support/mod.rs`) when that flag
# is set. Every CLR-real test still SKIPPED cleanly without it (correct tool
# gating), but a PR that only touches `lang-aot` never actually proved real
# CoreCLR execution on its own merge-gate CI — only a forced main-branch full
# build did. This declaration is what VM-032/VM-036 already established for
# Windows native execution, applied to the CLR-real column.
# needs-toolchain: dotnet
# Macsyma and McCarthy conformance execute emitted BEAM modules with erl.
# CI installs Erlang through its Elixir/setup-beam toolchain gate.
# needs-toolchain: elixir
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
# THAT WARNING HAS ALREADY BEEN MISSED ONCE. The CLR09-CLR16 campaign
# (2026-09-19..20) added four test targets -- clr_long_branches,
# clr_source_typed, clr_strict_flow, clr_typed_scalars, 19 tests between them --
# and none were added here, because this file was last touched 2026-09-07. They
# compiled in the check step and silently asserted nothing in CI for the whole
# campaign. They are in the list below now.
#
# The failure mode is structural, not careless: adding a `tests/*.rs` file is a
# complete, working, locally-green change, and nothing in the toolchain reports
# that a new target went unprotected. If you add a test target to this crate,
# add it to line 45 in the SAME commit.
#
# `lang_matrix` stays a filtered target (see the VM-065 audit below), but since
# 2026-10-19 its filters include the full-corpus aggregate. The two ALGOL reds
# that kept the aggregate out are fixed: native AOT refused captured 4-D array
# formals (VM-D005: x86_64-backend had no stack-passed arguments), and the JVM
# raised `VerifyError: Stack size too large` on captured 3-D/4-D real arrays
# (#12032: iir-to-jvm-class-file declared a fixed max_stack).
cargo test -p lang-aot --lib --test cargo_archive_path --test array_ref_tracing --test beam_array_len --test beam_cons --test beam_emit --test beam_predicates --test beam_symbols_lambda --test cil_cons --test cil_emit --test cil_lambda --test cil_predicates --test cil_symbols --test clr_long_branches --test clr_real_cons --test clr_real_lambda --test clr_real_macsyma --test clr_real_predicates --test clr_real_scalar --test clr_real_symbols --test clr_source_typed --test clr_strict_flow --test clr_typed_scalars --test conformance --test e6d2b_dynamic_arith --test end_to_end_smoke --test e6d6_llvm_records --test e6d6b_union_match_tagged --test e6d7a_wasm_closures --test historical_round_trip --test jit_mccarthy --test jvm_cons --test jvm_emit --test jvm_lambda --test jvm_predicates --test jvm_symbols --test llvm_cond --test llvm_cons --test llvm_gc_completion --test llvm_gc_write_barrier --test llvm_lambda --test llvm_predicates --test llvm_scalar --test llvm_symbols --test macos_native_lisp --test macsyma_conformance --test metacircular --test vm_gc_reclamation --test wasm_emit --test wasm_memory_growth --test op_conformance
cargo test -p lang-aot --test lang_matrix t7_differential_random_basic_
cargo test -p lang-aot --test lang_matrix portable_text_stdout_
cargo test -p lang-aot --test lang_matrix matrix_every_proven_cell_agrees -- --exact --nocapture
cargo test -p lang-aot --test lang_matrix proven_columns_do_not_silently_skip -- --exact
# PREP01 slice 1's acceptance criterion. The line above already RUNS the nine
# MacroOct rows on every backend whose toolchain is present, but running them
# only proves the selected program behaves; these two prove it is the SAME
# program as its hand-expanded Oct equivalent, which is the claim the whole
# slice exists to make. A filter rather than a new test target, so the oracle
# cannot drift from the rows it checks — it reads `PROGRAMS` directly and fails
# if any MacroOct row lacks a hand-expanded twin.
cargo test -p lang-aot --test lang_matrix macrooct -- --nocapture
# The pinned corpus counts. Also filtered in rather than left to the unfiltered
# target, which is excluded above — so before this line the declaration in
# LANG-VM-FEATURE-COVERAGE.md and the corpus could diverge in CI without
# anything noticing, which is exactly the failure the test was written to stop.
cargo test -p lang-aot --test lang_matrix feature_coverage_doc_counts_match_programs_source -- --exact
#
# VM-065 AUDIT (2026-09-21): `lang_matrix` is run BY NAME FILTER here, because
# the unfiltered target is red (above). A filter is not a file: a test living
# in lang_matrix.rs that matches none of these lines does not run, and nothing
# reports it -- the same defect as an omitted `--test` target, but harder to
# see, because the file IS named. Auditing every #[test] in that file against
# these filters found six that ran nowhere:
#
#   feature_coverage_doc_counts_match_programs_source   ← the test PINNING the
#       coverage doc against this file. It had never run, which is why the
#       ALGOL row drifted 233 -> 292 rows unchallenged (VM-063).
#   t7_differential_random_u8_expressions_agree         ← the `t7_..._basic_`
#       filter has a trailing underscore and never matched the u8 sibling.
#   twig_beam_match_union / _string_ops /
#       _dynamic_arith_list_ops_and_closures            ← BEAM track tests.
#   every_backend_name_round_trips_through_the_single_cell_env_var
#
# All five are now listed below. The sixth,
#
#   proven_columns_do_not_silently_skip  was held back while ALGOL 60 on the
#       JVM raised `VerifyError: Stack size too large` (#12032). It guards
#       against silent backend skips and runs above since #12032 closed.
#
# When adding a test to lang_matrix.rs, add a filter for it here in the SAME
# commit, and prefer `-- --exact` so a later rename fails loudly instead of
# silently matching nothing.
cargo test -p lang-aot --test lang_matrix t7_differential_random_u8_expressions_agree -- --exact
cargo test -p lang-aot --test lang_matrix twig_beam_match_union -- --exact
cargo test -p lang-aot --test lang_matrix twig_beam_string_ops -- --exact
cargo test -p lang-aot --test lang_matrix twig_beam_dynamic_arith_list_ops_and_closures -- --exact
cargo test -p lang-aot --test lang_matrix every_backend_name_round_trips_through_the_single_cell_env_var -- --exact
cargo test -p lang-aot --test lang_matrix algol_out_of_range_entier_traps_on_every_available_standard_backend -- --exact
# LANG83 ordered output stream: one ALGOL program mixing `print_i64` and
# `print_str` must produce one ordered stream on every available column (WASM/VM/
# JIT used to drop the integers whenever any byte output existed).
cargo test -p lang-aot --test lang_matrix mixed_print_i64_and_print_str_output_is_one_ordered_stream_on_every_available_backend -- --exact
# LANG84 phase 4a: Perl on every backend. The eleven Perl rows also run in the
# full-corpus aggregate above; these name the Perl-specific claims so a Perl
# regression is reported as one. The oracle compares every row's expectation
# with a host `perl` (skips without one); the LANG83 check runs
# `check_module_for` on all nine backends, CLR and BEAM included.
cargo test -p lang-aot --test lang_matrix perl_rows_run_on_every_available_backend -- --exact
cargo test -p lang-aot --test lang_matrix perl_rows_agree_with_the_host_perl_oracle -- --exact
cargo test -p lang-aot --test lang_matrix perl_rows_pass_the_lang83_check_on_all_nine_backends -- --exact
# LANG84 phase 7a: Python's py-int-print on every backend. The eighteen Python
# rows also run in the full-corpus aggregate above; these name the
# Python-specific claims so a Python regression is reported as one. The oracle
# compares every expectation with a host python3 3.13 (skips without one); the
# corpus test ties the rows to code/grammars/python/corpus/py-int-print in both
# directions; the LANG83 check runs `check_module_for` on all nine backends,
# CLR, BEAM and AArch64 included. The StdoutThenTrap trap-kind reader is pinned
# by the last line (shared with LANG84 phase 5a, which lists it too).
cargo test -p lang-aot --test lang_matrix python_rows_run_on_every_available_backend -- --exact
cargo test -p lang-aot --test lang_matrix python_rows_are_py_int_print_corpus_cases -- --exact
cargo test -p lang-aot --test lang_matrix python_rows_agree_with_the_host_python_oracle -- --exact
cargo test -p lang-aot --test lang_matrix python_rows_pass_the_lang83_check_on_all_nine_backends -- --exact
cargo test -p lang-aot --test lang_matrix trap_kinds_are_read_from_every_columns_stderr -- --exact
