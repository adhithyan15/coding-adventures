---
category: Workspace & package metadata
---

# Scope Cargo.lock edits to the exact package stanza; matching the first dependency list can attach a valid crate to an unrelated package.

While adding the first-party JSON parser dev dependency for the hash-functions fixture test, I applied a lockfile patch using only the start of a generic `dependencies` list. It matched the earlier `actionscript-compiler` stanza instead of `coding_adventures_hash_functions`. I removed the misplaced entry and added it beneath the exact hash-functions package name. For future lock updates, use Cargo's lockfile maintenance command where the selected workspace supports it; for manual edits, anchor on the exact package name and verify the resulting stanza before running the package test.
