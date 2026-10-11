---
category: Repo policy / workflow reminders
---

# A git fetch of several refs fetches none of them if one ref is missing

After a PR merged, its head branch was deleted on the remote. The next
`git fetch origin main claude/<branch>` failed with "couldn't find remote ref
claude/<branch>" and fetched **nothing**, `main` included. The next command,
`git checkout -B claude/<branch> origin/main`, then rebuilt the branch from a
stale `main` that did not contain the PR that had just merged. The first sign
was a cherry-pick conflict in the workspace `Cargo.toml` that made no sense.
Then `git diff` showed the spec "losing" the merged PR's sections.

What to do instead:
- After a merge, fetch `main` on its own (`git fetch origin main`). Do not
  chain the deleted head branch into the same fetch.
- Never put `&&`-less `;` between a fetch and the checkout that relies on it.
- Before cherry-picking onto a fresh base, check that `git log -1 origin/main`
  is the merge you expect.
