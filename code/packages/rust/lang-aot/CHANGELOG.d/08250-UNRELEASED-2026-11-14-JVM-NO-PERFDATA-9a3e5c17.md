## Unreleased - 2026-11-14 - JVM tests disable hsperfdata

### Fixed

- Pass `-XX:-UsePerfData` to every test `java` invocation that runs a program
  and inspects its output. Parallel JVMs could hit a locked
  `/tmp/hsperfdata_<user>/<pid>` file and print a unified-logging warning to
  stdout, which corrupted byte-for-byte output comparisons (intermittent CI
  failure in `flush_before_trap_is_proven_on_every_executable_column`).
  Test-only; no version bump.
