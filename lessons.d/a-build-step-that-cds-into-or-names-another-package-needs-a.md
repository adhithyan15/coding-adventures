---
category: BUILD files & dependency management
---

# A BUILD step that cds into or names another package needs a build-tool deps= line even when Cargo already depends on it

PR #20168 (VOS-040a) added a new Rust program, `vos-model-sat`. Its BUILD ran a cross-check from
inside another package:

```
cd ../../../packages/rust/vos-sched-contract && cargo run ... -- cross-check .
```

The crate already named `vos-sched-contract` as a Cargo dev-dependency, and every local test,
clippy and fmt run was green. CI's `detect` job still failed the PR on its BUILD/CI validation:

```
rust/programs/vos-model-sat (.../BUILD): undeclared local package refs: rust/vos-sched-contract
```

The build tool's validator (`internal/validator/validator.go`) scans BUILD commands for relative
paths that resolve to another package. Any such package must be either a resolved prerequisite or
declared with `# build-tool: deps=...`. The Rust resolver does not count a Cargo **dev**-dependency
as a prerequisite, so the path in the BUILD line was "hidden".

The fix was one line at the top of both `BUILD` and `BUILD_windows`:

```
# build-tool: deps=rust/vos-sched-contract
```

What to do differently:
- Before pushing a new package, or any BUILD line that `cd`s into, or passes a path to, another
  package, run the validator locally from the repo root. Build it with `go build -o bt .` in
  `code/programs/go/build-tool`, then run `bt -diff-base origin/main -emit-plan plan.json -dry-run`.
  A clean exit means the refs are declared. Local `cargo test` does not run this check.
- Put the `deps=` line in BOTH `BUILD` and `BUILD_windows`. The directive is read only from the
  platform-selected file (see the build-tool platform-directive lesson).
- Watch for cycles. If package A's BUILD declares B, and B's BUILD later runs A (for example a
  contract crate whose BUILD calls a lane tool that cross-checks the contract), the second
  declaration closes a loop. Decide which side owns the step before adding the second reference.
