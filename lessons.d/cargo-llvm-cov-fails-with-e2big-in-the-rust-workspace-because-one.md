---
category: Rust
---

# cargo llvm-cov fails with E2BIG in the Rust workspace because one environment variable lists every crate; instrument by hand

`cargo llvm-cov -p <crate>` in `code/packages/rust` dies before compiling
anything with `could not execute process cargo test ... Argument list too long
(os error 7)`. It sets `__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES` to the name
of every crate in the workspace (about 140 KiB), and Linux refuses a single
environment string above 128 KiB (`MAX_ARG_STRLEN`). `--workspace` and
`--no-report` do not help; the variable is built from the whole workspace.

Do instead: instrument by hand with a private target directory so nothing else
is rebuilt:

```sh
export RUSTFLAGS="-C instrument-coverage" CARGO_TARGET_DIR=<scratch>/covtarget
export LLVM_PROFILE_FILE=<scratch>/prof/p-%p-%m.profraw
cargo test -p <crate> --tests
# executables: cargo test --no-run --message-format=json | grep compiler-artifact
llvm-profdata merge -sparse <scratch>/prof/*.profraw -o cov.profdata
llvm-cov report --instr-profile=cov.profdata -object <exe> -object <exe> ...
```

(`llvm-profdata` and `llvm-cov` come with the `llvm-tools` rustup component,
under `~/.rustup/toolchains/*/lib/rustlib/<host>/bin`.) `llvm-cov show` with the
first executable positional and the rest as `-object` prints per-line counts;
intersecting the zero-count lines with `git diff -U0` gives the coverage of the
*changed* code, which is what a PR's ">95% on changed code" is about. Delete the
target directory afterwards: an instrumented debug build is about 1.5 GB.
