---
category: Security boundaries
---

# A read-only staged directory breaks recursive teardown; deny sandbox-child writes without removing host-owner cleanup access

Changing a host-owned snapshot directory from `0700` to `0500` looked like a
portable way to keep plugins from replacing staged code, but it also denied the
host's recursive cleanup its required directory write permission. Every native
integration test completed its assertions and then failed while removing the
fixture. Keep owner access needed for lifecycle operations and express the
untrusted child's narrower rights in the actual OS sandbox policy (for example,
explicit Seatbelt write denies or a read-only namespace mount). Always run the
full integration test, including teardown, after changing staging permissions.
