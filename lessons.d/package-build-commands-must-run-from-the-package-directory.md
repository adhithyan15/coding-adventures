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
