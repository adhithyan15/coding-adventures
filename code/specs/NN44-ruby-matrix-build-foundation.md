# NN44: Ruby Matrix Build Foundation

Status: specified before implementation; package gates and exact-head CI pending.

## Prerequisite discovered after NN43

NN43 merged in PR #18235 at exact head `dde20a816746`, squash `32369d29a87`,
with actual four-native 55-case corpus suites, twelve-direction interchange,
package builds and all applicable CI/CodeQL successful. Reprioritize reliability,
prerequisites, portability and smallest complete delivery before Ruby training.

The existing pure-Ruby matrix arithmetic lives in `ruby/matrix/lib/matrix_ml.rb`.
Its 47 tests (74 assertions) pass on Ruby 3.4 locally, but the package has no
BUILD/BUILD_windows, gemspec, package-local Gemfile or Rakefile. A neural-framework
gem must not invent dependency-install commands for an undiscoverable foundation.
Deliver this prerequisite alone; it does not count as native neural training.

## Bounded packaging contract

Make the existing matrix library discoverable, buildable and locally installable
as a gem using repository Ruby conventions and its documented 0.2.0 version.
Supply package metadata, explicit packaged files, license/author/summary,
package-local development dependencies, test/lint/coverage entry points and both
platform BUILD fronts. Keep `require "matrix_ml"`, public Matrix API, numerical
behavior and dependency-free native arithmetic compatible. Do not substitute the
optional Rust binding or Ruby standard-library Matrix as the implementation.

Use a real gem build/install smoke test in an isolated temporary gem environment
with correct cleanup, then require matrix_ml and execute a small dot-product
oracle. Package builds must execute the actual complete matrix test suite,
Standard Ruby lint and coverage (target at least 95% library line coverage).
Add focused missing numerical/boundary tests as needed; do not weaken thresholds
or change public semantics merely to silence tooling. Retain pure-computation
capability metadata and generated-file conventions. Record the source's mutable
public arrays/shape boundary honestly: the later neural API must validate/copy
before constructing matrix objects.

## Delivery and resource boundary

Update README/changelog, build/dependency metadata and roadmap. Validate both
BUILD fronts structurally, run the direct package test/build/lint/coverage and
gem smoke commands, links/lessons/diff checks. Only actual repository CI proves
build-tool discovery and platform installation. Independent security review
precedes every push; one ready PR, all applicable exact-head checks and verified
merge gate completion. Native Ruby framework and five-native checkpoint exchange
remain separate following specifications.

Disk dropped to about 259 MB during handoff. Cleaning only the disposable
NN38 exact-copy workspace's regenerable Cargo target recovered 114 MiB, leaving
about 374 MB; all source copies and unrelated work were preserved. Recheck disk
before builds, keep installs/output bounded and avoid new full checkouts or Rust
builds. The removed target can be regenerated from those retained sources.
