---
category: Rust
---

# Rollback diagnostics must distinguish retained destinations from actual original backups

The independent CV02 review injected a later installation failure and a failed
rollback removal for a destination that did not exist before compilation. The
error named a retained `old` backup, although no original existed and cleanup
removed the staging directory. Report the actual destination whose removal is
unconfirmed and explicitly state that no original backup was created. Reserve
backup recovery paths for a tracked backup creation; tests must compare the
diagnostic with actual remaining files.
