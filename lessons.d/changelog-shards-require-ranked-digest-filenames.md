---
category: Workspace & package metadata
---

# Changelog shards require ranked digest filenames

The repo-wide metadata job rejected a date-based changelog filename even though its level-three heading was correct. New documentation shards require a positive five-digit rank, uppercase ASCII slug and eight lowercase hex digits from SHA-256 of the raw heading line (excluding newline). Read the doc-shard contract and check the filename as well as the heading before pushing. Existing grandfathered names are not examples for new shards. This occurred in VisiCalc PR18240; rename the shard without changing its content or weakening the validator.
