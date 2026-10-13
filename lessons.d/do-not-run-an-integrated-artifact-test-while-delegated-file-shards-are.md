---
category: Testing & coverage
---

# Do not run an integrated artifact test while delegated file shards are still incomplete; validate the finished shard or wait for integration boundaries.

The coordinator ran the full C2 artifact test while Mock 2's confidential master and key were still being authored in a delegated shard. The suite could only report the known missing files and stopped before providing useful validation of Mock 1. Wait for each delegated owner to declare its bundle complete, or use a shard-specific read-only check that does not require unfinished sibling artifacts.
