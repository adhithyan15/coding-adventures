category: Workspace & package metadata
---

# Read JSON as UTF-8 and verify bytes before rewriting canonical fixtures

A helper read a UTF-8 JSON file with the Windows default encoding, which
mojibaked comments when it rewrote the file. The changed bytes were caught in
diff review before publication and restored from the exact Git object. Specify
UTF-8 explicitly for canonical JSON and compare raw bytes or hashes before
accepting a rewrite; successful JSON parsing alone does not prove byte safety.
