## 0.2.2

- Round-2 security fixes: the target's parent is a throwaway launcher (heap-limited
  and polled), so a flood sent "to its parent" never reaches the engine; the engine
  traps exits, monitors its caller and is linked to launcher and target, so a dead
  caller or an expired bound leaves no orphan; the engine itself has a heap limit and
  a mailbox watch.
- The wall clock is a real monotonic deadline; `:memory` is polled every slice and
  the `:binary` item every 4th; more than `budget / 48` binary references is over
  budget; the caller's bound has real slack.
- Memory tests shrunk for CI (8 x 32 MiB, 128 MiB copy, 100k messages) with generous
  wall-clock asserts; docs correct the linked-child-survives-normal-exit note and list
  the accepted limits.
