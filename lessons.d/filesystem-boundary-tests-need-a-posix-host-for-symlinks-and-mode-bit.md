---
category: Testing & coverage
---

# Filesystem-boundary tests need a POSIX host for symlinks and mode-bit permissions

The build-tool source-boundary test module creates symlinks and uses restrictive
mode bits to verify that unsafe or unreadable inputs are rejected. On a Windows
host without symlink privilege, `Path.symlink_to` fails before those assertions
run. Windows also does not interpret POSIX `chmod` mode bits as unreadable-file
permissions, so those fixtures can remain readable. A failed Windows run is not
evidence that these security checks passed or that the implementation is wrong.

Run the full module on a supported POSIX CI host. Keep the tests and their
assertions intact; do not skip the cases or report a Windows run as a substitute.
The repository metadata-contracts job already runs on Ubuntu, so invoke the
module there alongside the other build-tool conformance checks.

Swift Testing can still report an overall successful test run when a
host-capability test was skipped. For the directory-symlink regression, CI must
check the named test event itself: require its `passed` event and reject both a
`skipped` event and a missing event. An aggregate green summary alone does not
prove that the symlink branch ran.
