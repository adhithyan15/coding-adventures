# VER07x — Guest entrypoint integration

Integrate the separately reviewed Mosaic, Starlark, Twig and WASM execution lanes
with one strict guest-envelope support package. These are four guest execution
lanes using the Rust host collector/statistics, alongside eighteen native library
implementations; do not count them as four additional independent statistics ports.

The parent Rust workspace and lockfile must contain all five crates. Remove
obsolete nested workspace locks/consumers when carrying the final support API
forward. Each guest CLI preserves its actual arguments in the sidecar, excluding
the executable name. Empty argument vectors are valid; absent/wrong-type vectors
and arguments containing NUL are invalid. The portable core report stays unchanged.

The Cargo consumer at `code/tests/rust-guest-microbenchmark-consumer` is a shared
test helper, not an independently owned product package. Its external-consumer
runtime check is compiled and run by `native_acceptance.py` from the Mosaic and
Starlark package `BUILD` files. Keep a reasoned `EXCLUDED` entry for this root in
`code/BUILD-EXEMPTIONS`, naming those owners and the report/envelope equality
assertion; do not add a duplicate standalone build front. If either owner stops
running that helper or the fixture moves, update this ownership rule and ledger
entry together.

Keep host clock/no-op and actual empty-guest execution probes distinct. Retain
their configuration, entry counts and raw elapsed observations, including zero
diagnostic readings. Do not subtract these probes from paired workload durations.
Setup and state rebuild costs belong to the boundary description. Never label
host wall-clock nanoseconds as simulated CPU cycles or measured cache events.

Twig/WASM copied fixture tests must compile from their own package contents.
Compare their local fixture to the repository canonical file at runtime only when
that file is available; a compile-time include of a sibling package is insufficient
for separately packaged source. Register these two canonical fixture consumers
alongside the thirty-seven existing readers, for thirty-nine actual consumers.
Keep exact selection, per-language filters, missing-reader failure, graph export
and fallback cache invalidation tests aligned with their discovered Rust names.

Run scoped locked offline tests, formatting and Clippy, downstream consumers,
packaged-content/source checks, and final forty-pair native host runs for all four
guests. Replay each core and sidecar, check digests and raw probe configuration,
and replay core reports through native readers. Obtain independent source and
integration reviews before publishing; registry publication and CPU simulation
remain separate milestones.
