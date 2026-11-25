## [0.1.0] - UNRELEASED

### Changed (KILN04 D44a, `kiln:recorder` 13 to 14)

- The dying spawn D35 drops is now classified on the exit record (`thread`: a
  clone whose flags include `CLONE_THREAD`; `process`: fork, vfork, a clone
  without it, or flags that cannot be read) in both trace readers, replacing
  the bool D44 carried.
- When every dropped spawn in a trace is a `thread`, the D44 orphan pids are
  listed in `Trace.ThreadOrphans` and `traced.DaemonLeft` leaves their exits
  out of the survivor test: a thread cannot outlive its group, and the group's
  other members are still tested. One fork, vfork or non-thread clone anywhere
  keeps D44's fail-closed behaviour. The accept set grows, so every weak key
  changes.
