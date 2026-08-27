# browser-bookmarks

Reusable, storage-neutral browser bookmark state.

`BookmarkUrl` uses `url-parser` canonical forms while deliberately preserving
fragments because a user may bookmark a specific document anchor.
`BookmarkCatalog` preserves user-visible ordering, updates equivalent URLs in
place, and rejects duplicate persisted identities. `BookmarkRepository` is an
object-safe load/save boundary; `transact` persists a candidate catalog before
committing it in memory so a failed write cannot create session/disk drift.

The crate knows nothing about files, JSON, windows, HTML, or Venture. Durable
adapters belong in separate crates and any browser shell can reuse this model.
