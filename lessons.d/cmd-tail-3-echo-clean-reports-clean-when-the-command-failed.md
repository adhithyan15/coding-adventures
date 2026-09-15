# `cmd | tail -3 && echo clean` reports clean when the command failed

Running clippy on a new crate:

```sh
cargo clippy -q -p coding_adventures_base64 --all-targets -- -D warnings 2>&1 | tail -3 && echo "  clean"
```

printed both the compile errors **and** "clean". `&&` tests the exit status of
the last element of the pipeline — `tail`, which succeeded — not `cargo`. The
happy word was printed by a shell that had no idea whether anything passed.

I have used that pattern repeatedly this session, and it has been reporting
success for whatever the pipeline's final stage happened to return. When the
command genuinely passed, the output was indistinguishable from this. Use
`set -o pipefail`, or test explicitly:

```sh
if cargo clippy ... ; then echo clean; else echo FAILED; fi
```

The general shape is one this file already has several instances of: **a status
line that is not derived from the thing it claims to describe.** A byte scan
that could not see undefined symbols, a mutation test whose mutation never
applied, a guard whose fixtures encoded the same assumption it did — and now a
"clean" that was just `tail` exiting zero.
