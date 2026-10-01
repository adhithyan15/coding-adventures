# Changelog

## 0.1.0 — 2026-10-01

- Added the `forme-linux-v1` exact-snapshot native launcher.
- Added user/mount/PID/network/IPC/UTS namespaces, private root, seccomp,
  `no_new_privs`, cgroup v2, rlimit, and descriptor enforcement.
- Added platform probes for filesystem, network, process, memory, and fd denial.
- Added post-chroot digest verification, capability dropping, delegated-cgroup
  validation, a monotonic wall-clock watchdog, and mount/session escape denial.
- Enabled unprivileged user namespaces in the dedicated hosted-runner gate when
  Ubuntu's AppArmor sysctl otherwise blocks the required namespace primitive.
- Replaced the optional Linux `AF_ALG` digest dependency with a package-local
  portable SHA-256 implementation derived from the repository's
  FIPS-vector-tested C package, without adding cross-platform build edges.
- Preserved bounded child setup exit codes for host-side attestation diagnostics.
