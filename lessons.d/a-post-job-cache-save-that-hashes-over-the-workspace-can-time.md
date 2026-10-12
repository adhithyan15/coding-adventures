---
category: CI & GitHub Actions
---

# A post-job cache save that hashes ** over the workspace can time out after a full build

**What happened:** on PR #17185, `build (macos-latest)` built and tested all
535 affected packages with no failures, but the job still failed:

```
subosito/flutter-action/.../action.yaml (Line: 135, Col: 14):
hashFiles('**/pubspec.lock') couldn't finish within 120 seconds.
```

`subosito/flutter-action` with `cache: true` also turns on its pub-dependency
cache, whose key is `hashFiles('**/pubspec.lock')` over the whole workspace.
GitHub evaluates that expression again in the post-job cache save. By then the
full build has filled the tree with node_modules, target directories and other
build output, and the glob ran past the 120-second hashFiles limit.

**Fix:** keep `cache: true` (the Flutter SDK cache, keyed by version) and set
`pub-cache: false` in `ci.yml`, so the unbounded `**` glob is never evaluated.

**Do differently:** before you enable a third-party action's cache in a job
that builds the whole monorepo, read its `action.yaml` for `hashFiles('**/…')`
keys. Prefer a key with no workspace glob, or a glob narrowed to a known small
directory.
