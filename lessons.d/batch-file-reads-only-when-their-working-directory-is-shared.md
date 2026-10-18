---
category: Testing & coverage
---

# Batch file reads only when their working directory is shared

A batched read again combined a repository-relative Rust path with an
artifact-relative audit directory while declaring the repository working
directory. The artifact lookup failed; the Rust read succeeded. Use absolute
artifact paths in mixed batches, or separate tool calls by working directory.
Recording this mistake then created an untracked lesson during a validation
producer's clean-worktree hold. Preserve that receipt as unaccepted even when
the unchanged Rust sources compile and show the intended failures; commit the
lesson after the producer ends and rerun validation on the clean successor.
Never move a lesson out of view to make a held validation appear unchanged.
