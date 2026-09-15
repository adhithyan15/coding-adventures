---
category: TypeScript / JavaScript
---

# Per-spec security review BEFORE pushing catches things linters miss

The `/security-review` skill spawns a sub-agent to audit every diff; ~50% of overnight PRs surface findings (most LOW/INFO, occasionally MEDIUM, rare CRITICAL) the main agent didn't see. Treat as mandatory pre-push, not optional.
