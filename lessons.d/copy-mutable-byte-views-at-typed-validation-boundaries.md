# Copy mutable byte views at typed validation boundaries

The lower-level TypeScript DER framing package intentionally exposes zero-copy
`Uint8Array` views, but carrying those views through a typed validation layer
lets callers rewrite bytes after validation and invalidate canonicality checks.
Snapshot framing bytes when constructing a validated element, return defensive
copies from public element and cursor accessors, and copy decoded byte-string
values so later caller mutation cannot change trusted decoder state.
