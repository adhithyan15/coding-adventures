---
category: BUILD files & dependency management
---

# Package BUILD commands must run from the package directory

A package `BUILD` file records commands under the build tool's contract that
each line runs with the package path as its working directory. Executing the
file itself from the repository root changes that context; relative commands
such as `cargo test -p ...` then fail before validating the package. Run the
listed command from the package directory, or use the repository build tool,
and do not treat a root-relative `bash path/to/BUILD` invocation as equivalent.

This recurred while revalidating an X.509 package after rebase: the earlier
focused Cargo gates passed, but `bash code/packages/rust/<crate>/BUILD` from the
checkout root failed because both internal Cargo commands searched from the
wrong directory. Run `./BUILD` with the crate directory as `cwd`.
