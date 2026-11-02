# sir-test-scratch

**Where the Semantic IR backends' execution tests put their files.**

Every `semantic-ir-to-*` backend is tested by running what it emits: the
test writes the emitted program to a file and hands it to the real toolchain
(`node`, `python3`, `go run`, `rustc`, `cc`, `ruby`).  This dev-only crate
(`publish = false`) owns the two resources those tests share with the rest
of the machine, so that nothing else can reach into a test while it runs.

```
  test ──▶ Scratch::new("sir_go_test_")      fresh dir, random name, 0700
             │
             ├─ write_new("prog.go", src)     create_new: never reuses a file
             │
             └─ go_command().arg("run") …     GOCACHE = <target>/<profile>/sir-go-build-cache
  drop ──▶ the directory and everything in it is removed (panics included)
```

## Private scratch directories

The tests used to write `/tmp/sir_go_twig_pc_car_<pid>_<n>.go`: unique among
one checkout's tests, but predictable and in a directory every process on the
host shares.  Another process could create the name first, replace it, or
plant a symlink there; beside a `.py` file, it could plant a module that
`python3` imports from the script's own directory.

`Scratch::new(prefix)` creates `<tmp>/<prefix><random>` with `create_dir`
(through the `tempfile` crate), mode `0700` on Unix, and removes it with its
contents on drop.  `write_new` and `create_new` open with `create_new`
(`O_CREAT | O_EXCL`), which refuses an existing file and a symlink alike.

A directory per **call**, not per process: a per-process directory would have
to live in a `static`, which Rust never drops, so it would be left behind
after every run.

## An isolated Go build cache

`go run` compiles against the standard library's export data in the build
cache, shared by default by every Go command on the machine
(`~/.cache/go-build`).  A `go clean -cache` in another session -- the usual
advice on a full disk -- can delete those files between the go command
finding them and the compiler or linker opening them, and the test reports a
compile error in a program that compiles fine:

```text
# command-line-arguments
.../link: cannot open file $GOCACHE/06/067a…-d: no such file or directory
```

That was the intermittent failure of `semantic-ir-to-go`'s
`tests/twig_pairs_and_closures.rs`.  To reproduce it, run a Go test binary
while a loop cleans the cache it uses:

```sh
export GOCACHE=$PWD/shared-gocache
( while true; do sleep 2.5; go clean -cache; done ) &
target/debug/deps/twig_pairs_and_closures-<hash> --test-threads=16
```

Before this crate that failed 9 runs out of 9; with `go_command()`, which
sets `GOCACHE` to `<target>/<profile>/sir-go-build-cache`, it passed 24 out
of 24.  Go's cache is safe for any number of concurrent `go` processes; only
deletion from outside breaks it, and nothing outside these tests knows this
directory.  `cargo clean` removes it.

## Usage

```toml
[dev-dependencies]
sir-test-scratch = { path = "../sir-test-scratch" }
```

```rust
let scratch = sir_test_scratch::Scratch::new("sir_go_test_");
let src = scratch.write("prog.go", &artifact.source).expect("write");
let out = sir_test_scratch::go_command().arg("run").arg(&src).output();
// `scratch` drops at the end of the scope, removing `prog.go` with it.
```

Used by every `semantic-ir-to-*` backend's tests, `sir-conformance` and
`sir-bench`.
