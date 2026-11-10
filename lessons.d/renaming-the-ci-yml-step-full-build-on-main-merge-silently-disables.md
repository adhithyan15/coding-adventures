---
category: CI & GitHub Actions
---

# Renaming the ci.yml step "Full build on main merge" silently disables the build-tool validators and breaks CI acceptance tests

PR #19040 made the forced, sharded full build run on a 6-hour schedule instead
of every `main` push, and renamed the `build` job step from `Full build on main
merge` to `Full build (forced and sharded)` because the old name was no longer
accurate. The `Repo-wide metadata contracts` job then failed.

The step name is an interface, not a label. Every build-tool validator (Go,
Python, Elixir, Lua, Perl, .NET) returns early with "nothing to check" when
`ci.yml` does not contain the literal string `Full build on main merge`, so the
rename would have turned off the check that every needed toolchain is wired into
the forced full build -- with no failure to say so. `ci_package_durations.py`
parses the step by that name from job logs, and
`test_mosaic_compose_runtime_ci_acceptance.py` indexes into the job text on it,
which is the assertion that actually failed.

The fix was to keep the original step name and leave a comment above it saying
why it must not change. When the *condition* for a CI step changes, change the
condition and its comments; leave the name alone unless you also update every
consumer. Before renaming any step in `ci.yml`, `grep -rn "<old name>" code/`
and treat each hit as a dependent that must change in the same PR.
