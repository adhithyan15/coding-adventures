---
category: Supply chain & CI pinning
---

# An exact npm sibling version needs a tracked v3 link witness before it is treated as local

**Context:** publishable packages can keep a normal exact semver dependency so their packed tarballs install independently, while repository `npm ci` uses a sibling directory through npm's v3 lockfile link records.

**Failure:** the dependency inventory saw only the semver in `package.json` and reported the first-party sibling as a new external package. Broadly exempting names would let unrelated packages hide behind a matching local manifest.

**Rule:** classify an exact direct runtime dependency as local only when a tracked regular `package-lock.json` v3 has the same exact root dependency and the direct `node_modules/<name>` entry links to a single sibling directory. That lock target must itself be tracked and its lock and manifest name/version must match. A shrinkwrap, missing lock, wrong link, non-sibling target, or incomplete evidence leaves the name external.

**Check:** exercise the policy through a temporary Git index, including missing or untracked lock evidence, then run the focused inventory check and `npm ci` from the package directory.
