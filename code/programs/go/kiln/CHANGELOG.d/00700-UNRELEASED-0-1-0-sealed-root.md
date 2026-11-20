## [0.1.0] - UNRELEASED

### Changed (trace: the sealed root, KILN04 D43)

- The sandbox's new root is a sealed tmpfs that holds only what the plan mounts,
  so an absent probe of a path it cannot contain is now a constant of the plan,
  not a `host-read`: `/run/systemd/resolve/stub-resolv.conf` (node's resolver,
  reached through `/etc/resolv.conf`), the `package.json`/`go.mod`/`go.work`
  above the checkout, `/home/package.json`, `/var/run`. `lookup` answers ABSENT
  for them without asking the host's disk. The exists-set (`sealed.go`): the
  checkout, declared roots, `HostBinds`, Home, Tmpdir, the scratch directories,
  the ancestors of those, and `/dev`, `/proc`, `/sys`.
- A call that SUCCEEDED at a sandbox-absent path stays refused (`host-read`),
  for a look and for a write-class call alike: the plan has nothing there.
- The scratch directories (`sandbox.ScratchDirs`: `/tmp`, `/var/tmp`, `/run`)
  are fresh tmpfs like Home and Tmpdir (`underTmp`, `isTmpRoot`), so D39 rule 1b
  and D40 containment apply to them; a scratch file the action made and removed
  is no longer `unstable-path`. Home and Tmpdir are never "inside `/tmp`".
- New `traced.Config.ScratchDirs` (empty keeps every pre-D43 verdict); the
  recorder passes `record.SandboxScratchDirs()`, a copy of `sandbox.ScratchDirs`.
  `traced.SandboxHas` exports the exists-set for the recorder's pinning test.
- New whole-action refusal `reshaped` (a successful rename or exchange of
  anything but a regular file the action made, or one that could not be placed):
  every absent probe allowed by D43, and every absent probe inside a scratch
  directory allowed by D39 rule 1b, becomes `host-read` again, and a FAILED call
  that stopped at a missing component of its text path with a `..` after it
  becomes `unstable-path:dotdot`. The second is a hole found while writing D43
  and true without `ScratchDirs`: a cwd renamed away, then a failed probe
  through `..`, was judged by the stale cwd text.
- `RecorderVersion` 10 to 11: the verdict changes both ways, so every weak key
  changes.

### Added

- `internal/traced/sealed_test.go`, `internal/record/sealedroot_test.go` (pins
  that the exists-set covers everything the plan mounts, and that the scratch
  lists agree), `sandbox_sealed_linux_amd64_test.go` (`TestSandboxSealedRoot`,
  required in `ci/assert_sandbox_tests.py`).
- The fuzzer model seals everything above the mounts (EROFS), seeds `/var/tmp`
  and `/run`, and has two stale-cwd bursts and one scratch-absence burst aimed
  at the sealed root.
- Spec KILN04 decision D43, with the attack analysis.
