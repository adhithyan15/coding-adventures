### Main CI repair: font-subset oracle interpreter pinning

- Fixed: `rust/font-subset`'s `fonttools_oracle` tests were panicking with
  "fontTools is required" on main CI despite the "Install fontTools" step
  succeeding earlier in the same job. The test resolves its Python
  interpreter via `FONTTOOLS_PYTHON`, defaulting to bare `python3` if unset;
  across a ~3-hour job with dozens of toolchain-setup steps between the
  install and the test run, `python3` re-resolving to the same interpreter
  hours later isn't guaranteed. The install step now pins `FONTTOOLS_PYTHON`
  to the literal interpreter path it just installed into, via `$GITHUB_ENV`,
  removing the dependency on PATH staying stable for the rest of the job.

