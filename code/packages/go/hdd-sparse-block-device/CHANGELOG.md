# Changelog

## Unreleased

- Add a sparse block device that keeps pending writes separate from timed HDD
  media and preflights complete dirty-set barriers before applying effects.
- Preserve exact cut receipts, lower read errors, detached block reads, and
  restart semantics over the durable sparse image.
- Add independent visible/durable model histories, both 512-block profiles,
  every terminal prefix cut, full-page I/O, and clean-controlled faults.
