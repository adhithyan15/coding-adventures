# Historical Web Warrant slice

`code/specs/historical_web.warrant` is the single source of truth for the
small executable wire model currently owned by this package. It models the
HTTP/0.9 request shape (`GET ` + address + CRLF), accepts only printable
non-space ASCII addresses (`!` through `~`), pins representative paths and
rejections, and quantifies the same guarded result over one-byte addresses.

The existing Warrant checker type-checks this model, but it is model-only: a
live generated host harness is not claimed. The current binding/codegen lane is
Rust-oriented, the Python function returns bytes/raises `ValueError` rather
than this model's `Result`, and no Python jail adapter is available here. HTML
tree claims are also deferred because the current Warrant profile has no
recursive `Element`/`Node` host binding or recursive model support. These are
precise integration gaps, not generated evidence. Do not label this slice as
proof or as full package verification.
