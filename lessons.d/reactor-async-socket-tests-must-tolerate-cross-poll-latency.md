---
category: Testing & coverage
---

# Reactor / async / socket tests must tolerate cross-poll latency

Don't assert that two independent readiness sources appear in the same `poll()` batch — accumulate observations across iterations. Don't assume a single `write_ready()` step makes the other side immediately readable. For nonblocking accept, try `accept()` first and only wait for readiness on `WouldBlock`.
