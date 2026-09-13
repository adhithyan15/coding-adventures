---
category: Repo policy / workflow reminders
---

# Plan autonomous work for maximum parallelism: smallest unblocker first, then fan out

When the work queue contains N independent items (e.g., 8 runtime libraries, none of which depend on each other), DO NOT serialize them through the loop one PR at a time. The right pattern: identify the smallest piece of work that unblocks parallel streams, push it as a quick PR, and **while it's in review/CI, open the rest as parallel PRs** — each on its own branch, each with its own babysit. The shared cron then juggles them all: it advances any stream whose PR merged, fixes any stream whose CI broke, rebases any that conflicts. Concretely: for the Phase-2 runtime libraries, the first PR (`mosaic-flux-react`) was the reference implementation that established the API surface; once it was in CI, the next three (`html`, `webcomponent`, `swiftui`) could have been opened in parallel rather than waiting on the chain. Recognizing parallelism is a state-file design choice — list items as `{ id, depends_on: [...], status }` so the driver can pick ALL items whose `depends_on` are merged.
