---
category: CI & GitHub Actions
---

# A workflow that triggers on both push (all branches) and pull_request runs twice for every push to a PR branch

Six path-filtered workflows (`build-swift`, `build-kotlin`,
`lang-runtime-safety`, `matrix-rust-napi`, `matrix-rust-python`,
`mycin-2026`) listed `push:` with no branch limit, or `branches: ['**']`, next
to `pull_request:`. A push to a branch that has an open pull request fires both
events, so the workflow ran twice for the same commit. On the macOS runners that
`Build Swift Apps` uses this was the expensive case: over two days there were 30
push runs on non-main branches against 25 pull-request runs, two `macos-15` jobs
each.

It was worse than a plain duplicate. A `push` path filter looks at the files in
the pushed commits, and merging `origin/main` into a branch brings main's changes
to `code/programs/swift/**` into that push. Branches that never touched Swift
still ran the Swift build.

The fix is `push: branches: [main]` and leaving `pull_request:` as it was. Main
still runs the workflow after a merge, and a pull request runs it once, on the
pull request's own diff. Required checks are the `CI gate` and
`Human Languages Books gate`, so nothing the branch protection needs depends on
the branch-push run. When adding a workflow with a path filter, give `push:` a
`branches: [main]` limit unless the workflow is meant for branches that have no
pull request.
