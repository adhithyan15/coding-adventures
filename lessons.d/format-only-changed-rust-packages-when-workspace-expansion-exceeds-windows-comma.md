# Format only changed Rust packages when workspace expansion exceeds Windows command limits

Running `cargo fmt --all` in a very large Rust workspace can expand into a
Windows command line that exceeds the operating system limit before rustfmt
starts. The error is environmental and does not indicate source corruption.

Format the packages changed by the task with `cargo fmt -p <package>` (or use
the package manifest path) and keep the formatting scope bounded. A failed
workspace expansion should be followed by package-scoped formatting, not an
attempt to increase global OS limits.
---
category: Cross-platform & Windows BUILD_windows
---
