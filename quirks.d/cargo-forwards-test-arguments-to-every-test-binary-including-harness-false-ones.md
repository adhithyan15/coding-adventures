---
package: rust/test-bootstrap
lane: rust
rung: E1
status: mitigated
---

# Cargo forwards test arguments to every test binary, including harness=false ones

`cargo test -p X -- --nocapture` passes `--nocapture` to *every* test target
of the package: libtest binaries and `harness = false` binaries alike. Many
BUILD files in the repo end in `-- --nocapture`. A strict L0 harness that
rejected unknown flags would therefore fail as soon as a package mixing
libtest tests and an L0 corpus test used such a BUILD line.

A test-name filter (`cargo test -p X some_name`) is forwarded the same way.

**Handling:** the Rust lane ignores `--nocapture` by name and rejects every
other unknown argument with exit 2 (TF01 §5.3 allows a lane to ignore its
build tool's own flags by name). Filters are rejected rather than silently
ignored, so a filtered run cannot look like a full one. A package that needs
filtered libtest runs should name its targets with `--test`, as most BUILDs
already do.
