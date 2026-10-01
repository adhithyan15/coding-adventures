# Changelog

## 0.1.0 — 2026-10-01

- Added the `forme-windows-v1` exact-snapshot native launcher.
- Added AppContainer, low-integrity restricted-token, mitigation-policy, Job
  Object, handle inheritance, and handle-count enforcement.
- Added platform probes for filesystem, network, process, memory, and handle denial.
- Added pinned snapshot handles, random ephemeral profile identities, complete
  failed-launch profile cleanup, an abnormal-exit profile janitor, a private
  cancellation channel, and a native wall-clock watchdog.
- Made the environment isolation probe clean under current MSVC secure-CRT
  warnings-as-errors builds.
- Preserved the native process-creation error in trusted-launcher stderr
  when process creation fails before readiness attestation.
- Switched process creation to the documented AppContainer security-
  capabilities flow so Windows derives the capability-free low-integrity token
  and namespace without a conflicting separately supplied primary token.
