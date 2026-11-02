# Changelog — bounded-source-file

## Unreleased — 0.1.0, #18474 security review

### Added

- `read_to_string(path, limit)`: refuses anything but a regular file
  (checked on the path before opening, and on the open handle), opens with
  `O_NONBLOCK` on Unix so a FIFO or tty swapped in cannot block, refuses a
  file whose size is over `limit` before reading, and reads through
  `take(limit + 1)`. Errors are `io::Error`: `InvalidInput`,
  `FileTooLarge`, `InvalidData`.
- `read_bounded(reader, limit)`: the bounded read alone, for any `Read`.
- Tests for each step, including FIFOs on Unix with a 10 s deadline so a
  regression fails instead of hanging.
