---
category: Testing & coverage
---

# Consume mutation producer exits before dependent writes

The guarded minima test applier yielded a live process session; the caller
started lesson creation before consuming that process's terminal result.
The applier subsequently returned actual exit0 and its exact four-path
receipt, and all seven then-present edits were inspected. No failed exit was
relabeled or lost, but the ordering allowed unnecessary overlapping mutations.

When a mutation command yields a session, consume its actual terminal exit
and inspect the result before starting any dependent writes. Independent
read-only work can continue while a producer runs. A yielded session alone
does not authorize treating an application or validation as complete.
