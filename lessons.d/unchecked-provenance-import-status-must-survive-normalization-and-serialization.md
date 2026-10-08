---
category: Security boundaries
---

# Unchecked provenance import status must survive normalization and serialization

Independent CV02 review reproduced nested duplicate metadata and omitted entry
arrays that direct checked import rejected, while allocator import silently
normalized them. Queries and a save followed by checked reload then accepted
the discarded/defaulted evidence as complete.

Generic imports now retain private allocator-only status. Strict validation and
queries reject it; canonical export includes unchecked_import:true, which
bounded checked import rejects. Generic reload retains that state even after
new records are added. Controlled constructors keep their ordinary snapshot
shape. A graph validator cannot reconstruct information already lost during
an unchecked decode; normalization must not erase that trust boundary.
