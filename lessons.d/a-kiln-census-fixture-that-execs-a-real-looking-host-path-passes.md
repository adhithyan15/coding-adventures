---
category: Cross-platform & Windows BUILD_windows
---

# A Kiln census fixture that execs a real-looking host path passes on macOS but fails on Linux, because the classifier resolves symlinks and ELF loaders on the host

Census PR C's fixture (`censusDirFix`) plays a canned trace whose step execs
`/bin/sh`, and the tests expected a clean census. They passed on macOS and failed
on the Linux CI job (`TestCensusEndToEnd`, `...HostileNames`, `...UntrustedInputs`,
`...EmptyAndMissing...`) with host reads of `/usr/bin/sh`, `/usr/bin/dash`,
`/lib64` and the ELF loader. The classifier does not take the trace's text on
trust: it follows symlinks on the REAL filesystem and reads an ELF image's
PT_INTERP loader. On merged-/usr Linux `/bin` is a symlink to `usr/bin`, so the
resolved path escapes the fixture's `/bin` image root; macOS has a plain `/bin/sh`
with no loader to chase, so nothing escaped.

Fix: declare `/usr`, `/lib` and `/lib64` as roots in the fixture, kind
`ephemeral` (image roots also compare ctime with the job start, which a rebuilt
runner image can fail). Rules for next time: a fixture whose expectation depends
on the host filesystem (any path the classifier will lstat, resolve or read)
must pin the roots it needs rather than lean on what this machine happens to
have; and `GOOS=linux go test -c` / `go vet` only COMPILE the Linux build, they
never run it, so a green cross-compile says nothing about Linux behaviour. Read
the classifier's host-root path before trusting a "clean" expectation.

**Second trap (found by the next CI run).** Covering `/usr`, `/lib` and `/lib64`
was not enough. The default `/bin` root is an *image* root, which compares the
ctime of what it covers with the fixture's fixed `JobStart` (1794000000). A
runner image built after that stamp makes the `/bin` link itself
`host-read:setup-modified:/bin`. A fixture that tests something other than the
host-root rules must not depend on host file times: declare the roots the exec
reaches as `ephemeral` (a fixture entry for `/bin` now replaces the default image
root). Keep at least one image root, because prepare refuses an empty `ro_roots`.
When a host-dependent test fails in CI, make the failure print what it found
(here: the census summary). One diagnostic run beats a second blind guess.
