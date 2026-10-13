---
category: Testing & coverage
---

# chief-of-staff-daemon-credential's concurrent-creator test fails as root, because root opens a mode-000 file

`load_or_create_credential` publishes a new credential file by creating it
with mode `000`, writing it, and only then making it `0600`. A concurrent
loader that opens the half-written file gets `EACCES`, and retries: that is
how it knows the file is not finished.

Root is not refused by mode bits. Run as root (as in a cloud dev container),
the loader opens the mode-`000` file at once, sees mode bits it rejects, and
fails with `InsecurePermissions`. So
`tests::concurrent_creators_converge_without_overwrite` fails every time as
root and passes as any other user, which is what CI runs.

What to do: before treating a failure in a permission-based protocol as a
regression, run the test binary unprivileged
(`setpriv --reuid=65534 --regid=65534 --clear-groups <test-binary>`). The
same trap as reading another process's `/proc` entries: root sees what the
code is designed not to let anyone see.
