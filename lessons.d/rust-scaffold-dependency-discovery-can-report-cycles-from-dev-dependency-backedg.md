---
category: BUILD files & dependency management
---

# Rust scaffold dependency discovery can report cycles from dev-dependency backedges

The Rust scaffold generator walks both ordinary and development path
dependencies when it tries to topologically order a requested dependency set.
Security-composition crates can have test-only backedges through shared fixture
packages, so requesting several related verifier crates at once can report a
cycle even though Cargo's production dependency graph is valid. The generator
aborts before creating the package.

When this happens, rerun the scaffold without `--depends-on`, then add only the
new crate's direct production and test dependencies to its generated
`Cargo.toml`. Use Cargo metadata and the package BUILD to validate the real
graph. Do not repeatedly remove dependencies from the requested set until the
generator happens to pass: that can silently omit a direct dependency from the
new package contract.
