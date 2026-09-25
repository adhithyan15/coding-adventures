---
category: CI & GitHub Actions
---

# A standalone workflow that lists ci.yml in its paths runs its whole matrix on every CI edit, and a push trigger on every branch runs each PR twice

**What went wrong.** `build-ocaml.yml` (OCAML03) is a separate workflow outside
the build tool's diff-based detection. Its path filter included
`.github/workflows/ci.yml`, and it triggered on `push` to `'**'` as well as
`pull_request`. So:

- every PR that edited `ci.yml` for an unrelated reason (a Swift toolchain pin,
  a Journal UI lane, a Compose test step) ran the full three-OS OCaml matrix of
  fresh solves and locked fixtures;
- each such PR ran it **twice**, once for the push and once for the pull request
  (12 OCaml jobs instead of 6);
- merging `main` into a branch counts `main`'s commits in the push event, so a
  PR that touched no OCaml at all (#16000) ran the matrix because the merge
  brought in someone else's `ci.yml` change.

On a busy day those jobs held runners that other PRs were queued behind, and
they are also where the opam flakes come from.

**Fix.** `push` is limited to `main`, and `ci.yml` is dropped from the paths.
`ci.yml`'s OCaml wiring is still validated by `ci.yml` itself, which runs
`test_ocaml_toolchain_lock.py` and `validate-repository-report` on every change
to it. The lock contract (`ocaml_toolchain_lock.py`) pins the narrower
triggers, and its test refuses widening them again.

**Do differently.**
- A standalone workflow's `paths` should list what *its* jobs consume. If the
  only reason to watch a file is to validate a contract, run that validation
  where the file is already checked, rather than rerunning a whole matrix.
- Use `push: branches: [main]` plus `pull_request`, never `push: ['**']` plus
  `pull_request`: the second doubles every PR's run.
- A `push` path filter sees every commit the push brings in, including merges
  from `main`.
