# Verify pinned-tool assertions against the pinned CLI before publishing CI

The OCaml toolchain workflow pinned opam `2.5.2` but tried to attest its
repository with `opam repository get-url default`. That subcommand does not
exist in opam 2.5, so all three runners finished compiler setup and then failed
the same preflight before dependency installation.

Lessons:

1. Pinning a tool version also pins its command surface. Run every scripted
   assertion against that exact version, rather than relying on a plausible
   subcommand name.
2. Do not assume a materialized `repo/<name>` checkout survives setup. The
   setup action may populate opam's repository cache and then remove unused
   mirrors, as it does on Windows. Query opam's own color-disabled repository
   report, assert the exact configured name set, and require the reviewed
   commit-qualified URL exactly once.
3. A cross-platform failure at the same post-setup step is a shared contract
   defect until logs prove otherwise; wait for the matrix logs and fix the one
   common assertion.
4. Exact machine comparisons must explicitly disable color. A CI action can
   export `CLICOLOR_FORCE=1`, causing a correct version such as `1.9.0` to
   arrive wrapped in ANSI escapes unless the probe passes `--color=never`.
