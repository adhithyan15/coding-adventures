---
category: Rust
---

# A short page is not the last page: follow the backend's cursor

The first draft of `chief-of-staff-vault-secret-store`'s loader paged with
`SealedStore::list` and treated a page shorter than `page_size` as the end of
the listing. `SealedStore::list` throws away the backend's `next_cursor`, so
the loader rebuilt it from the last key and guessed "done" from page length.

That guess is wrong for `storage-fs`. Its `list` sorts the directory, cuts it
to `page_size`, then reads each key and quietly skips any that come back
`None`, which happens when the key is deleted between the scan and the read.
The page comes back one short and `next_cursor` is still set. A loader that
stopped there silently lost every record after the first page. For a vault
loader that promises all-or-nothing, that is the worst failure: a secret goes
missing, and the only symptom is a refused legitimate caller far from the
cause. The security review caught it before CI did.

The fix was to add `SealedStore::list_page`, which keeps `next_cursor`, page on
it, and stop only when it is `None`. The regression test wraps
`InMemoryStorageBackend` in a backend that drops a record from every non-final
page while keeping the cursor. Against the old loop it lists 127 of 300
records.

General rule: page length is not a protocol. If an API returns a cursor,
follow it. If a wrapper drops the cursor, fix the wrapper. Don't infer the
cursor from page length.
