---
category: Testing & coverage
---

# A test that runs go against the shared build cache fails with a compile error when another process cleans that cache

**What happened.** `semantic-ir-to-go/tests/twig_pairs_and_closures.rs`
failed twice in one run -- both of its tests at once -- with a Go "compile
error", then passed fifteen reruns, and was filed as a flake.  The emitted
program was fine (its source is byte-identical across 40 processes).  `go run`
compiles and links against the standard library's export data in the build
cache, `~/.cache/go-build`, which every Go command on the machine shares.
Another session ran `go clean -cache` (the usual advice when that cache
fills a tight disk), deleting entries between the go command finding them
and the compiler or linker opening them:

```text
# command-line-arguments
.../link: cannot open file $GOCACHE/06/067a…-d: no such file or directory
```

Reproduced by running the test binary while a loop runs `go clean -cache` on
the cache it uses: 9 failures in 9 runs.

**Fix.** Every `go` the SIR tests run comes from
`sir_test_scratch::go_command()`, which sets `GOCACHE` to
`<target>/<profile>/sir-go-build-cache`.  Go's cache is safe for many
concurrent `go` processes; it only breaks when something outside deletes from
it, and nothing outside this checkout's tests knows that directory.  The same
stress then passed 24 runs in 24.

**Do differently.** "Passes on rerun" is a symptom.  When a toolchain error
names a file under a cache or temp directory rather than a line of the
program, ask who else uses that directory, and reproduce by doing to it what
those others do (clean it, fill the disk, write the same name).  A test that
runs an external toolchain should own every mutable directory that toolchain
uses.
