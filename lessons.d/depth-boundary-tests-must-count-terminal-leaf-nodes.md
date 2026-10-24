---
category: Testing & coverage
---

# Depth-boundary tests must count terminal leaf nodes

A renderer boundary test initially counted only nested container and paragraph
nodes, so its supposedly depth-256 tree also contained an uncounted text leaf
at depth 257 and failed correctly.  When testing an exact recursive depth
ceiling, enumerate the full root-to-leaf path—including terminal inline or
scalar nodes—before deciding how many wrappers belong in the accepted case.
