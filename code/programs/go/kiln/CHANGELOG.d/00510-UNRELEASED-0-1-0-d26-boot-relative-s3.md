## [0.1.0] - UNRELEASED

### Fixed (D26: S3's start-time gate is boot-relative)

- The survivor scan's S3 gate ("did this process start after `prepare`?")
  compared the wall-clock `binding.prepared_at` with a start time derived from
  `/proc/<pid>/stat` starttime through `btime`. A backward wall-clock step of
  more than 2 s between `prepare` and the step could therefore exclude a
  genuinely new process from S3 (S1 and S2 were never affected).
- `prepare` now seals `binding.prepared_ticks`: the first field of `/proc/uptime`
  in clock ticks (USER_HZ, 100 per second) since boot. S3 compares stat field 22
  with it directly, less 200 ticks of slack (`SurvivorSlackTicks`); `btime` and
  the wall clock are out of the gate. `prepared_at` stays for display and audit.
- Sealed-schema change: `kiln-record-binding/1` becomes `kiln-record-binding/2`
  with the new field `prepared_ticks`. The decoder refuses a missing,
  non-positive or absurd (over 100 years) value, so an old binding fails closed.
  `SurvivorQuery.Since` (a `time.Time`) became `SinceTicks` (int64), the `procFS`
  `Boot` method is gone, and `startedAt` became `startTicks`.
- `Options.Uptime` is the injectable boot clock (default `ReadUptimeTicks`);
  `prepare` refuses when it fails or says less than one tick. `ParseUptimeTicks`
  refuses empty, non-numeric, NaN, infinite, non-positive or over-100-year input.
- Tests: a simulated 10 s backward wall-clock step no longer excludes a new
  process (and the old arithmetic is replayed to show it did), a process older
  than prepare minus slack is still excluded, the boundary is inclusive, uptime
  parsing, prepare refusing a bad uptime, and binding validation of the new field.
