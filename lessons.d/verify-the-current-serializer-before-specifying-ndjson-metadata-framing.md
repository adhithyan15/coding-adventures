---
category: Rust
---

# Verify the current serializer before specifying NDJSON metadata framing

The external chronology draft described the existing NDJSON format as a root
header followed by entries. Direct inspection of `checked_export.rs` proved it
writes ordinary CVEntry lines first and a final `_meta` footer. Designing new
journal framing from recollection would have changed an established contract.

Read the current serializer and round-trip tests before specifying wire changes.
Preserve generic-client framing and add explicit opted-in typed event records
with version/state in the final footer. Full reconstruction must preserve event
sequences and references; partial exports cannot claim complete history.
