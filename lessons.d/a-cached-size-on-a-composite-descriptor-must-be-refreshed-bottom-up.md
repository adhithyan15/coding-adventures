---
category: Python
---

# A cached size on a composite descriptor must be refreshed bottom-up, children before parents

The warrant oracle's `domains._refresh` recomputed a variant's cached per-constructor sizes before the tuple or record beneath it had
dropped its own cache, so after a 16-bit leaf was demoted the variant kept the stale size and a valid corpus was refused `W0706`
(CV2-F17). A cache derived from children is only correct if the children are refreshed first: walk post-order (children, then the
node), never a stack that visits the parent first. When a tree caches a value computed from its children, test every nesting shape
(variant over tuple, over record, over variant, under a record, under an option), not only a leaf directly under the cached node.
