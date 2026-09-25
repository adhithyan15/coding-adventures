---
category: CI & GitHub Actions
---

# A CI job that asserts an exact toolchain version must install that version, not rely on the runner image's preinstalled one

**What went wrong.** The `unicode17-swift-conformance` job ran `command -v swift` on `ubuntu-24.04`. It then asserted the exact version line `Swift version 6.3.3 (swift-6.3.3-RELEASE)`, but it never installed Swift: it used whatever GitHub had preinstalled on the image.

When GitHub rolled out runner image `20260920.314.1`, the bundled Swift changed. The assertion failed before a single Unicode vector ran, on every PR that triggered the job. The job had passed on image `20260907.300.1`. Nothing in the repo had changed, so the failure looked like a regression in whichever PR happened to trigger it (#15971, a Journal PR).

**Fix.** The job now installs the pinned toolchain itself:
- it downloads the official swift.org `swift-6.3.3-RELEASE-ubuntu24.04.tar.gz`;
- it checks a pinned SHA-256 (`sha256sum --check --strict`);
- it extracts the archive under `$RUNNER_TEMP`;
- it prepends the toolchain's `usr/bin` to `$GITHUB_PATH`.

When the pin was recorded, the tarball's signature was verified against the Swift 6.x Release Signing Key (`EF80A866B47A981F`). The exact-version assertion stays as a guard.

**Do differently.**
- Asserting a version you did not install is a time bomb: GitHub updates runner images every week or two. Either install the exact version (pinned URL + hash), or assert a range you actually tolerate.
- Record the hash from a signature-verified download, not from the same unauthenticated fetch CI will do.
- When a required job fails on a PR that touches none of its inputs, suspect the runner image first. Compare the `Image: … Version:` line in the job log against the last green run.
