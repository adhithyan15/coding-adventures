# Historical Web Warrant slice

`code/specs/historical_web.warrant` is the single source of truth for the
small executable wire claim currently owned by this package. It models the
HTTP/0.9 request shape (`GET ` + address + CRLF), pins representative paths,
and quantifies the same shape over one-byte addresses.

The existing Warrant checker was inspected for this document, but a live
generated host harness is not claimed: the current binding/codegen lane is
Rust-oriented, while the implementation is Python, and no Python jail adapter
is available here. HTML tree claims are also deferred because the current
Warrant profile has no recursive `Element`/`Node` host binding or recursive
model support. These are precise integration gaps, not generated evidence.
Do not label this slice as proof or as full package verification.
