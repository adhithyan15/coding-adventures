---
category: Testing & coverage
---

# Persist a native subprocess exit before parsing or post-run assertions

Independent preflight of an unexecuted cumulative-Timeline validation runner
found that it updated the native exit code and raw-stream hashes in memory,
then parsed output and checked frozen inputs before saving the receipt. A
failure in that interval could leave a durable row falsely appearing unfinished
despite an actual completed process. Preserve the unexecuted runner and add an
immediate receipt save after the exit/hash update in a versioned successor.
Do not use output parsing, wrapper success, or a later assertion as a substitute
for first recording the actual native result. No failing runner execution or
native acceptance was claimed for the rejected candidate.
