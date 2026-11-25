## 0.1.3 - security review of the fuzz engine (VER06o)

- MEDIUM: retained corpus seeds, crash inputs and mutated candidates are `B.copy`d (a slice pinned up to 1 MiB while
  only its length was charged); a test checks the stored entry does not share its parent buffer.
- MEDIUM: leaked masked workers are counted per campaign; later kill waits are 100 ms; four leaks abort the campaign
  with the `workers-leaked` verdict reason (never a pass). The allocation limit not being delivered under `mask` is
  documented.
- MEDIUM (documented): the wall clock needs the target built with `-fno-omit-yields`; README, spec 5.4 and the Executor
  header say so, and an out-of-process runner is recorded as future work.
- LOW: `Handle` is abstract (`Fuzz.Handle.Internal` holds the representation and test seams); the allocation-counter
  tampering limit is documented; the masked-spinner test releases its worker through a stop flag; the vector replayer
  validates tokens (40-digit integer cap, hex shape, arity) before converting them.
