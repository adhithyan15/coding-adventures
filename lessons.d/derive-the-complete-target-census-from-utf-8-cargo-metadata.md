---
category: Rust
---

# Derive the complete target census from UTF-8 Cargo metadata

A Venn validation supplement assumed that two affected packages meant two
executable harnesses. Fresh Cargo metadata instead declared17: two libraries
and15 integration targets. The guard stopped before executing those targets
or doctests. A separate inspection also failed when Windows' default code
page decoded the UTF-8 metadata; explicitly reading UTF-8 fixed inspection.

Use the complete metadata target inventory, not package or library counts.
Join every enabled harness to its exact package/target/source and executable,
run the discovered integrations and separate doctests, and retain disabled
platform zero-test results as zero. Test-disabled examples still need their
appropriate build/lint gate. Preserve the initial refusal without filtering
away the newly discovered work.
