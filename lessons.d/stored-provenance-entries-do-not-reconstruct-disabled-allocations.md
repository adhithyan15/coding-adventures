---
category: Rust
---

# Stored provenance entries do not reconstruct disabled allocations

An opt-in compact allocator needs a counter separate from graph entries.
CVLog issues IDs even when storage is disabled. Reconstructing the counter
only from saved entries would therefore reuse IDs after loading an empty or
partially stored log. Such reuse can silently conflate entities or overwrite
evidence when tracing becomes enabled again.

CV01 saves an explicit allocator version and last-issued watermark, including
unstored allocations. A fixed-width hex string preserves all u64 bits for JSON
clients with double-precision numbers. Tests save disabled and partially stored
logs, reload, enable storage and verify the next allocation remains distinct.
Unknown versions and inconsistent ID/state are errors; exhaustion/collision
must not mutate the counter or graph. Existing infallible signatures fail fast
instead of fabricating IDs, and callers can use fallible allocation methods.

Allocation state is not graph completeness. Filtered exports keep metadata but
declare partial views and cannot be reloaded as full compact logs. NDJSON must
preserve new log-level metadata in its footer, rather than copying only fields
known before the allocator migration. Compact IDs still require separate graph
integrity checks, resource budgets and actual transformation chronology.
