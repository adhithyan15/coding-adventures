---
category: Security boundaries
---

# Scope an exemption by who is looking, not by a list of things

Applying an identity check to tool outputs globally broke the smart-home audit readers, which exist to report on principals. A per-tool exemption list would have drifted; scoping by *runtime* worked because "is this an agent surface" is already answered by a gate that refuses peer-naming tools there.
