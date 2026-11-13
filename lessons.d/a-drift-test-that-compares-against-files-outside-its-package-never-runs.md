---
category: CI & GitHub Actions
---

# A drift test that compares against files outside its package never runs when only those files change

**What went wrong.** Kiln's `TestCommittedPolicyFiles` checks that the pins in
`code/kiln/legacy/toolchains.json` match the repository's pin sources
(`mise.toml`, `code/specs/data/toolchain-pins.json`, the version assertions in
`ci.yml`). #19269 bumped Rust from 1.99.1 to 1.100.0 in `mise.toml` and was
green. The build tool selects packages by changed paths, and none of the
changed paths was inside Kiln, so Kiln's tests never ran. `main` stayed red for
Kiln until the next unrelated Kiln branch ran the suite locally.

**Fix.** Bump the pins in `toolchains.json`, and declare the three pin sources
as Kiln's repository inputs (`go/programs/kiln` in the build tool's
`repositoryInputConsumers`), so any change to them selects Kiln.

**Do differently.** When you add a test that reads files outside its own
package, whether as a drift check, a cross-reference or a mirror, register
those files as the package's repository inputs in the same PR. Otherwise the
test only guards changes to the package itself, never changes to the files it
is checking.
