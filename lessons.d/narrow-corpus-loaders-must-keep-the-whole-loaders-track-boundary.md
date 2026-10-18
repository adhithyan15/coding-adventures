---
category: Security boundaries
---

# Narrow corpus loaders must keep the whole loader's track boundary

The Spanish mock-stem reporter stopped loading every language to reduce
latency. The first narrow version called the track-specific curriculum and
lesson loaders directly, but those follow a linked `spanish` directory while
the whole-corpus lesson loader rejects it. A reporter could then credit words
from outside the curriculum tree. Before any track-specific read, check with
`lstat` that the selected track itself is a real directory, and pin the
linked-directory case in a regression test. Narrowing I/O must not silently
narrow the security checks that guarded the original read.
