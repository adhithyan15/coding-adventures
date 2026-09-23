# ONE COMMAND PER LINE. The Go build tool reads this file with `readLines` and
# runs EACH LINE as its own `sh -c`; a trailing `\` is not honoured, so a
# wrapped command silently runs only its first line. See lang-aot/BUILD, where
# that mistake cost this repo 40 unrun test targets.
#
# `--test iir_identity` is named explicitly rather than relying on a bare
# `cargo test`, for the same reason: it is PREP01 slice 1's acceptance
# criterion, and a suite that compiles but never runs is worse than no suite,
# because it reads as coverage.
cargo build -p coding-adventures-macrooct-iir-compiler
cargo test -p coding-adventures-macrooct-iir-compiler --lib --test iir_identity
