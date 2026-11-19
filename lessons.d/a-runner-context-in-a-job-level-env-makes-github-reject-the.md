---
category: CI & GitHub Actions
---

# A runner context in a job-level env makes GitHub reject the whole workflow, and only a dispatch reveals it

**What went wrong.** `kiln-census.yml` (#19702) set
`CENSUS_OUT: ${{ runner.temp }}/census-out` in the job's `env:`. The `runner`
context does not exist there; it exists only inside steps. GitHub refused the
first `workflow_dispatch` with HTTP 422, "Unrecognized named-value: 'runner'".
Every pre-merge check was green:
- PyYAML only checks YAML syntax;
- the repo's workflow contract tests check content rules, not expression contexts;
- a `workflow_dispatch`-only workflow never runs on its own PR, so CI never parsed it.

**Fix.** The first step writes `CENSUS_OUT=$RUNNER_TEMP/census-out` to
`$GITHUB_ENV`. Using `${{ runner.temp }}` inside a step's `with:` is fine.

**Do differently.**
- For a new dispatch-only workflow, either lint it with actionlint (it knows
  which contexts each key allows), or prove that GitHub parses it before merging.
- To prove it, push the branch and run `gh workflow run <file> --ref <branch>`.
  GitHub parses the branch's copy and answers 422 on an expression error. The
  dispatch works once the file exists on the default branch; for a brand-new
  file, rely on actionlint.
- Use `$RUNNER_TEMP` (the shell variable) in `run:`, and `${{ runner.* }}` only in
  step-level keys.
