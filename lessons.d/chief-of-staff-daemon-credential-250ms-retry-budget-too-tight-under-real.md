# `chief-of-staff-daemon-credential`: 250ms retry budget too tight under real CI contention (ubuntu-latest, non-root)

`concurrent_creators_converge_without_overwrite` spawns 8 threads that all race
`load_or_create_credential()` on the same not-yet-existing path. Exactly one
thread wins the `O_CREAT|O_EXCL` (Unix) / `CREATE_NEW` (Windows) create; the
other seven get `Exists` and fall back to polling `open_existing` in a loop
(`PUBLICATION_RETRIES` iterations of a 1ms sleep) waiting for the winner to
finish `write_all` + two `sync_all` + `fchmod`/ACL-set on the file it just
created — until then the file is either mode-000 (Unix) or opened exclusively
(Windows), so losers see `EACCES`/`ERROR_SHARING_VIOLATION` → `Busy` → retry.

`PUBLICATION_RETRIES = 250` (a ~250ms budget) is enough on an idle machine but
not guaranteed on a loaded/shared GitHub Actions runner: real CI (ubuntu-latest,
**not** root, so this is not the root-permission-bypass artifact documented
elsewhere in this file) showed multiple losing threads exhausting the retry
loop and surfacing `AccessFailed` from `tests::concurrent_creators_converge_
without_overwrite` — a genuine robustness gap in the retry budget, not a logic
bug in the create/publish protocol itself. Fix: widen `PUBLICATION_RETRIES` to
3000 (~3s) in both `unix.rs` and `windows.rs` — same protocol, just more
headroom for CI scheduling noise. This is a case where an env-specific-looking
failure (multiple threads, same panic site, same error) needs the same "is
this the known root-sandbox artifact or a real gap?" check as any other
concurrency test: reproduced locally as root it presents as
`InsecurePermissions` (root bypasses the mode-000 gate entirely, so the loser
opens+reads before the winner's `fchmod`, then fails the strict-permissions
check) — a different symptom from the real CI's `AccessFailed`, confirming
these are two distinct issues and the local root reproduction cannot be used
to validate this particular fix.
