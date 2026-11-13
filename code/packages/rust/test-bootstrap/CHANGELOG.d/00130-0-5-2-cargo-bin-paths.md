Cargo 1.100 places each test executable in a per-target `OUT_DIR`. The L0
process tests now use Cargo-provided `CARGO_BIN_EXE_*` paths for the self-corpus
and exit-fixture binaries, preserving the existing harness-free corpus run and
sabotage check.
