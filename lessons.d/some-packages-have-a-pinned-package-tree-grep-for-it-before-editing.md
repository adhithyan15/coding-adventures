---
category: BUILD files & dependency management
---

# Some packages have a pinned package_tree: grep for it before editing a package, and never attest evidence you did not produce

A mechanical Perl `BUILD_windows` batch also touched `code/packages/perl/barcode-layout-1d` (BUILD_windows, CHANGELOG.md, a test). That package's tree hash is pinned by the lane-evidence contract in `code/specs/fixtures/barcode-layout-1d-v1/targets.json` (`package_tree`, `verified_revision`) and `.claude/package-parity-loop-state.json`, so the "Repo-wide metadata contracts" job failed: changing the tree requires re-running the lane's tests and updating the evidence in the same PR.

The tempting fix is to paste the new tree hash into the registry. Do not: the evidence asserts a test run. I could not run the Perl lane locally (`Test2::V0` is not installable; the proxy blocks CPAN), so I restored the package to its pinned state in the batch PR and moved it to its own PR.

Next time: before a bulk change, run `git grep -l '"package_tree"' -- code .claude` and exclude every pinned package root from the batch; do those individually, with the lane's tests actually run.
