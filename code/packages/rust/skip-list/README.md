# skip_list

DT20 ordered collection implemented in Rust.

This crate exposes the skip-list API from the spec:

- `insert`
- `search`
- `delete`
- `contains`
- `range_query`
- `rank`
- `by_rank`
- `to_list`
- `min`
- `max`

The implementation preserves the observable sorted behavior required by the
spec and the existing tests.
