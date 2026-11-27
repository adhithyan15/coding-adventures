---
category: Testing & coverage
---

# Ruby report fixtures must supply the full metadata group

When a Ruby report fixture adds any driver metadata, it must add all six
standard group fields: workload, invocation, revision, runtime, OS and arch.
Partial fixtures that once replayed only because the core API projected the
metadata away no longer represent valid reports. Use a complete metadata
fixture for accepted inputs and remove one field at a time only in negative
tests.
