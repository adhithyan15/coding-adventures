---
category: Repo policy / workflow reminders
---

# Respect the security-review wait tools minimum timeout

The first mailbox wait after dispatching the mandatory security review used a
1,000 ms timeout, but the collaboration wait API requires at least 10,000 ms.
Use the tool's documented minimum for short review polls and longer bounded
waits when no other local work remains; the rejected wait does not cancel the
reviewer.

This recurred on the constrained-anchor DNS slice. Do not infer that a quick
poll may use an arbitrary small value: `wait_agent` validates the documented
10,000 ms minimum before it checks mailbox state.
