## Unreleased - strace `???(` for a thread killed at syscall entry (D18)

- `internal/trace` accepted nothing spelled `???`, so a real CI trace whose tail
  was `exit_group(0 <unfinished ...>`, `3486  ???( <unfinished ...>`, ... was refused as
  `trace-incomplete:unparsable-line` (a flake in `TestSandboxRealStrace`). `???` is what strace
  prints when a thread reaches a syscall-entry stop with a fatal signal already pending: the kernel
  returns before dispatch, so the call never ran and the thread only exits. The parser now accepts
  exactly three spellings (`???( <unfinished ...>`, optionally followed by `<... ??? resumed>) = ?`,
  and the one-line `???() = ?`), emits no record for them, and requires the pid's exit line next.
  Every other `???` shape, and any call, second entry or signal line from that pid before its exit,
  stays refused with the existing reasons; no new reason code.
- KILN04 section 4.3 ("Order and split calls") documents the shapes and why they are sound.
