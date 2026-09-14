---
category: Cryptography & security review
---

# Stateful TCP servers must cap per-connection buffered-input size

Partial-frame buffers (RESP, HTTP body) without a max let a slow-stream attacker exhaust heap. On overflow, clear, send protocol error if possible, close.
