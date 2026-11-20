## [0.1.0] - UNRELEASED

### Changed (host-roots policy: /proc/self/fd and four absent probes, KILN04 D37)

- `code/kiln/legacy/host-roots.json`: `ephemeral` gains `/proc/self/fd` and
  `/proc/self/fd/*` (clang readlinks one per header it opens); `absent-ok` gains
  the exact names `/sys/kernel/mm/transparent_hugepage/hpage_pmd_size`,
  `/pyvenv.cfg`, `/opt/rh` and `/include`.
- Unchanged by design: an open through `/proc/self/fd/N` is still classified by
  the descriptor's `-y` path; `/proc/self/fd/N/x` is still `proc-path`
  (`procMagicBeyond`); other pids stay `proc-path`; a present read of an
  absent-ok name stays `host-read`; `transparent_hugepage/enabled` and `defrag`
  stay flagged.
- No `RecorderVersion` bump: the policy file is in `consumed` and so in the
  digest, as for D27.

### Added

- `internal/traced/procfd_test.go`: the real policy file against each case above.
