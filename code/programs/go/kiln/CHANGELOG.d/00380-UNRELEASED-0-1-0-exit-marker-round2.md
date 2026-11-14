## [0.1.0] - UNRELEASED

### Fixed (review round 2 of the signals PR)

- `Trace.InitExitLine` is built from per-thread `exit_group` candidates (earliest
  one that did not return; a failed or withdrawn call is no candidate) and, when
  there is none, the start line of the LAST raw `exit` that did not return. A
  failing `exit_group` held open by one thread can no longer hide another
  thread's real one, and a single-threaded init ending with a raw `exit(2)` has a
  marker. Marker 0 now means the init group made no exit call and died by a
  signal, so the step is already failed and incomplete.
- Spec: the D19 yield note (two accepted patterns may be flagged
  `daemon-left` nondeterministically; count it per action in the first runs).
- Test harness: all four pipe ends are closed when strace fails to start.
