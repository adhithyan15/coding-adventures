---
category: CI & GitHub Actions
---

# GitHub runs shell: bash steps with errexit on, whatever set line the script starts with

**What went wrong.** The census workflow's recording step began with
`set -uo pipefail`. It deliberately left out `-e`, because it captures every
exit status itself (`build_rc=$?`). But `shell: bash` makes GitHub run the
script as `bash --noprofile --norc -e -o pipefail {0}`, so errexit was already
on. Leaving `-e` out of a `set` line does not turn it off.

The build tool exited non-zero because 93 packages failed for missing
toolchains, and the step died right there:
- `build_rc=$?` never ran;
- collect and the census never ran;
- no summary was written.

The job did fail closed, through `.worst`, but the first real census
(run 44625742979) produced nothing.

**Fix.** Start the step with `set +e -uo pipefail`.

**Do differently.** In any step that inspects exit statuses, write `set +e`
explicitly. Read the `shell:` line GitHub logs at the top of each step: it
shows the real flags.
