---
category: CI & GitHub Actions
---

# A release tag spliced into a run: script is code execution for anyone who can push a tag

**What went wrong.** `publish.yml` began its tag parser with
`TAG="${{ github.event.release.tag_name }}"`. GitHub substitutes `${{ }}`
expressions into the script text before bash runs it, so the tag became
shell source, not data. Git ref names may legally contain `$`, `(`, `)`,
`;`, `|` and `${IFS}` (`git check-ref-format` accepts
`python/x$(curl …)/v1.0.0`). Anyone able to push a tag and publish a release,
including an agent's token, could therefore run commands in `parse-tag`.
From there they could forge `pkg_dir`, which later jobs spliced into their own
scripts, one of them the job that writes `RUBYGEMS_API_KEY` to disk. The
workflow also failed outright on every other product's release tag, because
it ran for all releases.

**The fix.**

- Pass the tag in through `env:`, so the shell only ever sees it as a
  variable.
- Check it against a strict pattern
  (`^(python|ruby)/[A-Za-z0-9_-]+/v<semver>$`) before writing anything to
  `$GITHUB_OUTPUT`.
- Skip tags the workflow does not own with a job-level
  `startsWith(github.event.release.tag_name, …)` filter.
- Read every derived output through `env:` in later jobs too.
- Give `gem push` its key as `GEM_HOST_API_KEY`, never as a file built from
  an expression.

**What to do differently.** Treat any `${{ }}` inside `run:` as code
generation. Event fields (tag and branch names, titles, bodies, inputs) and
anything derived from them go through `env:`. Validate them before they
become job outputs, because outputs carry the same taint into jobs that hold
secrets.
