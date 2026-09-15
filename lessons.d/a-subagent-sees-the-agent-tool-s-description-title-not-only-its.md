---
category: Repo policy / workflow reminders
---

# A subagent sees the Agent tool's `description`/title, not only its `prompt` — never leak the answer there

In a blind-evaluation experiment (ADJ52: a domain-blind ingester that must classify a clinical case without knowing the diagnosis), an `Agent` call's `description` was "Domain-blind ingester on McArdle case". The subagent picked up "McArdle" from the description and referenced it in its reasoning, contaminating the supposedly blind run — even though the prompt itself was scrubbed. Rule: **every field that reaches a subagent (description AND prompt) must be scrubbed of the ground truth / answer / case name.** Keep descriptions generic ("Ingest problem statement into IR"). The same leak applies to file paths handed to a sandboxed agent — a path like `.../cases/mcardle-pmr/...` names the answer; pass inputs inline, not by revealing the path. Discard and re-run any blind output produced after such a leak.
