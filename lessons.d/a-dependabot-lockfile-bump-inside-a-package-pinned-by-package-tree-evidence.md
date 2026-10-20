---
category: CI & GitHub Actions
---

# A Dependabot lockfile bump inside a package pinned by package_tree evidence fails the metadata contract

`code/specs/fixtures/barcode-layout-1d-v1/targets.json` records each lane's
`package_tree`: the Git tree ID of the whole package root, lockfile included.
`test_barcode_layout_1d_fixtures.py` requires that tree to equal both
`<verified_revision>:<package_root>` and `HEAD:<package_root>`. Dependabot
PR #17522 bumped `source-map-js` (dev-only) in
`code/packages/typescript/barcode-layout-1d/package-lock.json`. That moved
the tree from `91e40aec` to `ebf60961`, and "Repo-wide metadata contracts"
went red with no source change.

Updating `package_tree` alone cannot pass: `verified_revision` must also hold
the new tree, must equal the backlog owner's `implementation_revision` in
`.claude/package-parity-loop-state.json`, and is shared by every lane of that
owner. The only commit with the new tree is in the PR, and the repo
squash-merges, so pinning it breaks main for every later PR (see
`evidence-pinned-to-a-pr-s-head-commit-breaks-main-once-the.md`). #17185
re-pinned anyway and had to require a merge-commit merge.

The fix on #17522 was to restore main's lockfile for that one package, as
#15723 held setup-ocaml back for its contract. The bump for that package
is left to the PR that re-verifies its evidence.

**Do instead:** when a Dependabot PR touches a package root named in a
fixture `targets.json` (`git grep -l '"package_tree"' code/specs/fixtures`),
decide up front: either drop that package from the bump, or re-verify
and re-pin it in a PR that will land as a merge commit. Never edit
`package_tree` by itself.
