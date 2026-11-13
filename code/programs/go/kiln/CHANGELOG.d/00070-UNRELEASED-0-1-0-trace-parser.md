## [0.1.0] - UNRELEASED

### Added

- `internal/trace` (KILN04 stage 1a): a pure parser for
  `strace -f --quiet=attach -y -s 4096` output. A hand-written scanner (no
  regular expressions) pairs `<unfinished ...>` / `<... X resumed>` lines per
  pid, unescapes strings and `-y` annotations exactly, and classifies every
  system call through the closed handled / ignored / refused table of KILN04
  section 4.4. It builds the process tree and tracks each process's working
  directory (`chdir`, `fchdir`, fork/vfork/clone copy, `CLONE_FS` sharing), the
  last `execve` path, exit lines, and `sockaddr` destinations (`AF_INET`,
  `AF_INET6`, `AF_UNIX` pathname and abstract). `Parse(r, initialCwd, limits)`
  returns ordered `Event`s with paths as printed; resolution against a tree is
  stage 1b.
- Every doubt is a `trace-incomplete:<detail>` error (unknown or refused call,
  `CLONE_UNTRACED` / `CLONE_NEW*` / `clone3`, personality change, truncated,
  ambiguous, non-UTF-8 or deleted path, unmatched resume, missing exit line,
  line or total size cap). Tests use canned strace 6.x fixtures; `FuzzParse`
  asserts no panic and only `trace-incomplete` errors.
- Security hardening: a child's cwd is snapshotted when the parent's
  `fork`/`clone` succeeds; a split call's cwd is sampled at its start;
  `chdir`/`fchdir` on a cwd shared by live `CLONE_FS` processes is
  `cwd-race`; argument counts are bounded while scanning; every record counts
  against `MaxEvents` and the default caps are a resident-memory bound (64 MiB,
  1M records); a final line without a newline, pids with leading zeros, NULL
  `connect`/`bind` addresses, non-symbolic socket domains and truncated
  `sendmsg`/`sendmmsg` arrays are refused; the parent chain is walked
  iteratively; `AT_EMPTY_PATH` uses the descriptor path; and
  `ParseOptions` with `HelperRoot` discards sandbox-helper setup before its
  first successful `execve`.
- KILN04 section 4.5 records the parser decisions of stage 1a.
