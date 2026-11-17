## [0.1.0] - UNRELEASED

### Review fixes (stage 4 PR 2b-2c-4)

- S3 is decided from `/proc/<pid>/status` `Uid:` (never skipped on the owner of
  `/proc/<pid>`, which a non-dumpable task makes root: a bypass) and applies only
  to processes that started at or after the sealed `prepared_at` less 2 s
  (`/proc/<pid>/stat` field 22 with `btime`), so a runner's `systemd --user`,
  ssh-agent and keyring daemons no longer void every run. An unreadable status
  or stat fails closed.
- Each scan has a 5 s deadline (`SurvivorScanTimeout`); a scan that blocks is a
  survivor. A mountinfo or cmdline over the 4 MiB bound is an inspection that
  failed (S3, start-time rule only), never parsed partially.
- S2 also names `<dir>/probe/toolchains-sandbox.json`.
- Wording: an unwritable `state-preloaded` marker is a run-void, not a shard-void.
