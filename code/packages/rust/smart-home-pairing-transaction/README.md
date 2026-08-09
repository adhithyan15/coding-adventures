# Smart Home Pairing Transaction

This crate coordinates credential pairing across the sealed Vault and durable
smart-home runtime without pretending the two stores share one transaction.
It writes a secret-free journal before credential creation, requires the new
opaque reference and Vault key to carry the transaction id, uses collision-safe
Vault writes and expected-revision runtime completion, and retains enough
opaque revision metadata to recover or roll back after a process crash.

Recovery is idempotent across four states: prepared, Vault written, runtime
committed, and cleanup complete. A prepared entry with no credential is
discarded. A durable credential is either completed into the expected runtime
revision or deleted at its exact Vault revision. Once runtime completion is
durable, the previous credential is deleted only at the revision captured
before the transaction began. Partial runtime references are retained for
inspection and never trigger credential deletion.

Journal records contain identifiers, opaque Vault references, revisions,
authorization identity, timestamps, and completion metadata. Credential bytes,
ciphertext, keys, nonces, and authentication material never enter the journal.
