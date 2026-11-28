---
category: Repo policy / workflow reminders
---

# Wait for the coordinator fetch before reading its new commit

In a partial clone, a metadata query against a commit that has not arrived yet
can start its own promisor fetch. During one integration, that happened while a
coordinator fetch for the same main update was already running, duplicating the
large object transfer. The query had `GIT_NO_LAZY_FETCH=1`; that did not prevent
the observed fetch in this case. The coordinator fetch completed successfully,
and only the duplicate local query process tree was stopped.

Before comparing commit metadata, wait until the coordinator confirms that the
requested ref and objects are available locally. If an accidental duplicate
fetch starts, identify its exact process tree and stop only that owned tree;
leave the coordinator's fetch untouched. This records the observed behavior,
not a claim about why the Git setting did not prevent it.
