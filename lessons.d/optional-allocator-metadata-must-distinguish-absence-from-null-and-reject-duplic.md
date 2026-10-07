---
category: Rust
---

# Optional allocator metadata must distinguish absence from null and reject duplicate identities

The CV01 import audit found that ordinary `Option<IdentityState>` deserialization
treated `identity: null` exactly like an omitted field. Empty or disabled logs
had no stored compact IDs to expose the mistake, so reload silently selected
legacy allocation. A present field must instead deserialize as a valid state
object; only field absence receives the legacy default. Test empty and disabled
snapshots explicitly, alongside normal round trips.

The same audit found that deserializing entries directly into a HashMap accepted
duplicate JSON identities and retained the last record, overwriting earlier
origin/history evidence. A map visitor now checks each decoded key before reading
its value and rejects duplicates, including equivalent escaped spellings. Test
raw JSON rather than first building a Value, which would already erase duplicate
keys. An invalid duplicate value must still report the duplicate identity: the
value should never be decoded after its key has failed.

Both regressions failed before repair. These are allocator-state and entry-map
integrity checks; they do not establish full graph/schema validity, bounded
import, canonical serialization or actual transformation chronology. Keep those
requirements explicit rather than describing a successful reload as complete
provenance validation.
