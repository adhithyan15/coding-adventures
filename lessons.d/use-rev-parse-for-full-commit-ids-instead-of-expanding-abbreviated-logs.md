# Use rev-parse for full commit IDs instead of expanding abbreviated logs

A short hash from `git log --oneline` was accidentally expanded by hand when
updating durable state, producing a plausible-looking but nonexistent commit
ID. Always capture the canonical full object name with `git rev-parse
origin/main` and copy that exact output into state; then compare the persisted
value to a fresh `rev-parse` result before committing.
