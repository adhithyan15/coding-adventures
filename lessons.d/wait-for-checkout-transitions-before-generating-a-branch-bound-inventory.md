---
category: Repo policy / workflow reminders
---

# Wait for checkout transitions before generating a branch-bound inventory

The recovery inventory was launched while `git switch` was still running.
It reported the previous checkout's 1,501 identities, and that count was
incorrectly attributed to the new main revision. The next fresh inventory
correctly found 1,506 identities and five new Rust broker singleton roots.

Checkout transitions and inventory generation are dependent operations, so
await a successful transition before scanning. Capture the full revision before
and after the report and reject results when those identities differ. If Git
state changed during a previous report, regenerate and classify the delta
before selecting any new work; never reuse it as a current-main result.
