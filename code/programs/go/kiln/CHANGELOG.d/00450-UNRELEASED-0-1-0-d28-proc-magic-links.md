## [0.1.0] - UNRELEASED

### Fixed (security, D28)

- The classifier treated `/proc` paths lexically, so
  `open("/proc/self/cwd/../../filesystems")` (a magic link plus `..`) looked
  like an allowed `/proc` read while reading an arbitrary file, leaving the
  record clean (a possible false cache hit). Now fail closed:
  a `..` that is not provably exact after a `/proc` component (and any `..`
  under `/dev/fd`) is `trace-incomplete:proc-path`; a path through a magic
  link (`cwd`, `root`, `exe`, `fd/N`, `map_files`, `ns`, also `/dev/fd/N/x`) is
  `proc-path` even if an ephemeral pattern matches; a successful `/proc` open
  whose `-y` path differs from the name is classified by the real path (or
  `proc-path`), and an open of a magic link with no usable `-y` path is
  `proc-path`. `/proc/mounts` and `/proc/net/...` are accepted with their pid
  spelling.
- `/proc/self/..`, `/proc/<pid>/..` and `/proc/..` still resolve lexically.
- KILN04 sections 5.1 and 5.5: decision D28. Tests: `procmagic_test.go`.
