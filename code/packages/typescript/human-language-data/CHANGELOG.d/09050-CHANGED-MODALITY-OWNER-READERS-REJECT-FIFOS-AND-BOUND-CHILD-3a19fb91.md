### Changed — modality owner readers reject FIFOs and bound child lifetime

- Open Unix modality owners with `O_NONBLOCK` as well as `O_NOFOLLOW`, then
  refuse anything other than an opened regular file before reading it. A FIFO
  replacing a listed owner can no longer park the pinned reader indefinitely.
- Give each bounded Python owner child a 20-second deadline. Kill an expired or
  oversized child and wait for close before rejecting; never return a partial
  manifest or bypass the exact owner and source-derived comparisons.
- Add a finite-deadline Unix FIFO regression and controlled TypeScript child
  tests for timeout, normal completion, child error, and oversized output. Existing corpus
  test, hook, worker, heap, and coverage gates remain unchanged.
