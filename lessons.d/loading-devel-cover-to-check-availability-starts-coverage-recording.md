---
category: Perl
---

# Loading Devel::Cover to check availability starts coverage recording

A read-only Arc2D audit used `require Devel::Cover` as an availability probe.
Loading the module started coverage and wrote a new `cover_db` tree in the
checkout, despite no test run being intended. The artifact timestamps and
paths were checked against the probe and the exact new tree was moved intact
to the recovery directory; unrelated checkpoint files were untouched.

Check module availability through installed module files or package metadata
without loading side-effecting instrumentation. If a coverage-enabled probe
is necessary, run it from a dedicated output directory with explicit output
configuration. Read-only intent does not make module initialization passive.
