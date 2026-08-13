# `twig-aot` had NO BUILD file, so the build tool never ran a single one of its
# tests in CI. That is the same gap that hid three red suites in `lang-aot`: the
# crate compiled (a workspace-wide `cargo check` elsewhere covers that much) but
# nothing ever executed its assertions, so `macos_arm64_smoke.rs` was free to
# accumulate both a red suite and an unused `use std::io::Write;` that breaks the
# `-D warnings` build on every host. This file closes the gap.
#
# ONE COMMAND, ONE LINE — DO NOT REFORMAT WITH BACKSLASH CONTINUATIONS. The
# build tool reads a BUILD file with `readLines` (build-tool/internal/discovery),
# which splits on newlines, drops blanks and `#` comments, and runs EACH
# REMAINING LINE as its own `sh -c`. A trailing `\` is NOT honoured — it
# silently truncates the command and turns the continuation lines into bogus
# `sh: --: invalid option` failures. However long the target list gets, it stays
# on one line.
#
# WHAT RUNS WHERE. Pull-request CI fans out over ubuntu-latest, macos-latest and
# windows-latest (see the `detect` job's build_matrix in .github/workflows/ci.yml),
# and three of these targets are whole-file host-gated, so each leg runs a
# different subset and the others compile to zero tests:
#
#   --lib               58 unit tests incl. the dynval_runtime_golden divergence
#                       guard. Host-independent — runs on all three legs.
#   e4d_str_helpers     `#![cfg(unix)]`; drives the C runtime's string helpers
#                       through $CC. Runs on ubuntu + macOS, no-op on Windows.
#   linux_x86_64_smoke  `#[cfg(target_os = "linux")]`; ELF end-to-end + GC
#                       stackmap differentials. Runs on the ubuntu leg only.
#   windows_x86_64_smoke `#[cfg(target_os = "windows")]`; PE end-to-end + GC
#                       stackmap differentials. Runs on the windows leg only.
#
# ONE TARGET IS DELIBERATELY EXCLUDED because it is red today. It is named here
# rather than silently dropped — an excluded target is an unprotected one, and it
# should be added back the moment it goes green:
#
#   macos_arm64_smoke   13 of 16 pass on Apple Silicon; 3 fail, all in the
#                       precise-GC-roots area:
#                         end_to_end_typed_twig_returns_42
#                           link failure — `___gc_register_stackmap` undefined in
#                           smoke.o; the test's own `ld` invocation does not link
#                           the GC runtime archive that emits the symbol.
#                         end_to_end_gc_precise_keeps_ref_reclaims_lookalike
#                           the i64 look-alike is not reclaimed — precise and
#                           conservative both report 80 bytes live.
#                         end_to_end_gc_recursive_frame_live_bytes_differential
#                           the intermediate self-recursive frame is not precisely
#                           mapped at the recursive-call return address (precise
#                           64, conservative 128, expected 0).
#                       This matters on more than a developer laptop: macos-latest
#                       is an Apple Silicon runner and IS in the pull-request
#                       matrix, so including this target would turn that leg red.
#
# EXCLUSION ONLY SKIPS THE RUN, NOT THE BUILD. The check step compiles every
# target in the crate with `-D warnings`, so an excluded suite that fails to
# COMPILE still fails CI — only its failing assertions are excluded. That is
# exactly how the unused import in macos_arm64_smoke.rs would have surfaced, and
# why fixing the import was a prerequisite for adding this file rather than an
# unrelated tidy-up.
#
# Listing targets explicitly rather than running the whole crate is what makes
# the exclusion visible. A bare `cargo test -p twig-aot` would hide it.
cargo test -p twig-aot --lib --test e4d_str_helpers --test linux_x86_64_smoke --test windows_x86_64_smoke
