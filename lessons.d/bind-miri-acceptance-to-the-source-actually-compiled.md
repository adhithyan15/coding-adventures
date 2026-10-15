---
category: Rust
---

# Bind Miri acceptance to the source actually compiled

During CV04a, the separate Miri target had compiled lexer metadata at 22:36:27 UTC, before the final raw-acceptance source edit at 22:38:21 UTC. It kept testing the earlier lexer even though later ordinary Cargo suites and reviews passed the final bytes. A separate target prevents artifact collisions but does not establish final-source acceptance. Stop the exact owned stale process tree after checking its workspace descendants, preserve its log as rejected evidence, and restart the full required script against frozen source. Record candidate source hashes and compiled metadata timing. Do not push until that final-source script exits successfully. Documentation-only updates can be checked by blob comparison; any tested-source mutation requires the affected verification to restart.
