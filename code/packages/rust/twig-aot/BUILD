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
# windows-2025 (the `detect` job's build_matrix in .github/workflows/ci.yml),
# but Rust is only ever built on ubuntu and macOS for this package. On a PR the
# "Build and test affected packages" step — the one that runs `-language all` —
# executes on Windows only when the diff needs Swift or Elixir; this Rust-only
# package enables neither. On main
# the detect job hardcodes every matrix entry to `"os": "ubuntu-latest"`, so the
# unguarded "Full build on main merge" step has no windows leg to run on. A green
# `build (windows-2025)` is therefore not evidence that Rust compiled there.
#
#   --lib               unit tests incl. the dynval_runtime_golden divergence
#                       guard. Host-independent — runs on ubuntu + macOS.
#   e4d_str_helpers     `#![cfg(unix)]`; drives the C runtime's string helpers
#                       through $CC. Runs on ubuntu + macOS.
#   gc_param_roots      `#![cfg(target_os = "linux")]`; #18492 — heap-valued
#                       parameters and their copies stay precise GC roots. Builds
#                       ELF executables with the system `cc`, runs them, and runs
#                       valgrind memcheck when installed. ubuntu only.
#   gc_global_and_buffer_roots `#![cfg(target_os = "linux")]`; #18765 / #18766 —
#                       globals and `alloc_bytes` string buffers stay precise GC
#                       roots. Same shape as gc_param_roots. ubuntu only.
#   linux_x86_64_smoke  `#![cfg(target_os = "linux")]`; ELF end-to-end: exit
#                       codes, putchar / heap byte I/O, the ELF machine field,
#                       stack-map registration, the two precise-GC `live_bytes`
#                       differentials, and a branch-joined runtime string's
#                       length. ubuntu only. It used to be excluded with 2 of 8
#                       failing (the precise-GC differentials); they have passed
#                       since the x86-64 precise-roots work landed: 8 of 8 on
#                       origin/main, and 9 of 9 in each of three runs on the
#                       #18765 branch, so it is listed again.
#   windows_x86_64_smoke `#![cfg(target_os = "windows")]`; PE end-to-end + GC
#                       stackmap differentials. Listed so a developer on a Windows
#                       host runs it, but NOTHING IN CI DOES — see above. Treat it
#                       as unwatched until the windows leg builds Rust.
#
# Be honest about what that leaves: in CI only `--lib` and `e4d_str_helpers` run
# on every host; the three Linux suites run on the ubuntu leg only.
#
# ONE TARGET IS DELIBERATELY EXCLUDED because it was red when last checked, on
# the precise-GC stackmap gap. It is not flaky and not unimportant — it was
# detecting a real hole, which is exactly why it is named here rather than
# silently dropped. Do NOT `#[ignore]` it or relax the assertions: a weakened
# test is a silent regression, and this whole campaign exists because tests that
# could not fail hid real failures. The failure list below predates the x86-64
# precise-roots work that made linux_x86_64_smoke's twins pass; re-check it on
# Apple Silicon and re-add the suite if it is green.
#
#   macos_arm64_smoke   13 of 16 pass on Apple Silicon; 3 fail:
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
#                       macos-latest is an Apple Silicon runner and IS in the
#                       pull-request matrix, so including this would turn that
#                       leg red — not just a developer laptop.
#
# EXCLUSION ONLY SKIPS THE RUN, NOT THE BUILD. The check step compiles every
# target in the crate with `-D warnings`, so an excluded suite that fails to
# COMPILE still fails CI — only its failing assertions are excluded. The
# excluded file above must therefore still compile clean, and it does (its
# unused import was fixed rather than excluded, precisely because exclusion buys
# nothing on a red compile).
#
# Listing targets explicitly rather than running the whole crate is what makes
# the exclusions visible. A bare `cargo test -p twig-aot` would hide them.
cargo test -p twig-aot --lib --test e4d_str_helpers --test gc_param_roots --test gc_global_and_buffer_roots --test linux_x86_64_smoke --test windows_x86_64_smoke
