---
category: Repo policy / workflow reminders
---

# Serialize audit helpers that write the same checkpoint

The Closure heartbeat, PR audit and build-plan audit helpers were treated as
independent searches and run concurrently. Each loaded then rewrote the entire
CV04-active-checkpoint.json. A later writer silently discarded a new build-plan
pointer even though the head-bound artifact receipt existed. Acceptance stayed
PENDING because the required pointer was missing; no old-head/current CI pass
was inferred and no source or hosted check was changed.

Run these state writers sequentially, and wait for actual process exit rather
than merely receiving a running session ID. Parallelize only read-only or truly
independent destinations. The resume wrapper checks each completed subprocess
before starting the next. Restore state from a fresh exact-head artifact audit,
not a stale plan pointer. Preserve this note for the next bounded repository
slice without changing the currently reviewed/published source just for the
checkpoint correction.
