---
category: Repo policy / workflow reminders
---

# Collaboration waits have a ten-second minimum

The collaboration wait tool rejected a 1,000 ms timeout because its supported
range starts at 10,000 ms. When polling a security-review agent, use at least
10,000 ms and at most the tool's documented upper bound instead of treating it
like a subsecond process poll.
