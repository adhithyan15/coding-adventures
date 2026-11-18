---
category: Testing & coverage
---

# Redaction tests must separate stable algorithm labels from numeric payloads

A diagnostic-redaction test rejected the substring `256` even though the
stable, payload-free error vocabulary intentionally names the SHA-256 profile.
The assertion failed without identifying any caller-controlled length leak.

Use distinctive caller values that cannot also occur in approved algorithm or
error labels, then assert those exact values are absent. Stable profile names
should remain testable and readable; redaction coverage should target secret or
caller-controlled payloads rather than every digit that happens to appear in a
public algorithm name.
