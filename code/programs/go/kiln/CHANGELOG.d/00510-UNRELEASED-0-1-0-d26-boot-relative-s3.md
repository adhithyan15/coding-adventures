## [0.1.0] - UNRELEASED

### Fixed (D26: S3's start-time gate is boot-relative)

- The survivor scan's S3 gate ("did this process start after `prepare`?")
  compared the wall-clock `binding.prepared_at` with a start time derived from
  `/proc/<pid>/stat` starttime through `btime`. A backward wall-clock step of
  more than 2 s between `prepare` and the step could therefore exclude a
  genuinely new process from S3 (S1 and S2 were never affected).
- `prepare` now seals `binding.prepared_ticks`: its OWN start time, field 22 of
  `/proc/self/stat`, in clock ticks (USER_HZ, 100 per second) since boot, read
  FIRST THING in `Prepare`, before it starts any process. It is not a later
  clock reading: `prepare` runs the structure and toolchain probes (PR-editable
  argv, timeouts up to minutes), and a leftover of one must count as new; its
  own start is earlier than all of them by construction, and it is the same
  field, unit and epoch as the comparand. S3 compares each process's stat
  field 22 with it directly, less 200 ticks of slack (`SurvivorSlackTicks`);
  `btime`, `/proc/uptime` and the wall clock are out of the gate. `prepared_at`
  stays for display and audit.
- Sealed-schema change: `kiln-record-binding/1` becomes `kiln-record-binding/2`
  with the new field `prepared_ticks`. The decoder refuses a missing,
  non-positive or absurd value (over `recfmt.MaxClockTicks`, 100 years, the one
  shared bound; `startTicks` uses it too), so an old binding fails closed.
  `SurvivorQuery.Since` (a `time.Time`) became `SinceTicks` (int64), the `procFS`
  `Boot` method is gone, and `startedAt` became `startTicks`.
- `Options.SelfStartTicks` is the injectable hook (default `ReadSelfStartTicks`,
  which reuses `startTicks`); `prepare` refuses when it fails or is out of range.
- `collect` checks the sealed value against its own start time
  (`CollectOptions.SelfStartTicks`): if its own start is earlier than
  `prepared_ticks` (a reboot, or a different clock source) or unreadable, the
  S3 bound is zero and every process counts.
- Tests: a simulated 10 s backward wall-clock step no longer excludes a new
  process (and the old arithmetic is replayed to show it did), a process older
  than prepare minus slack is still excluded, the boundary is inclusive,
  `prepare` reads its start before any probe runs, `prepare` refuses a bad
  start time, `collect`'s sanity check (later, equal, earlier, unreadable), and
  binding validation of the new field. A Linux-only test spawns a child and
  pins that its `/proc/<pid>/stat` start is on the same clock as the sealed one.
