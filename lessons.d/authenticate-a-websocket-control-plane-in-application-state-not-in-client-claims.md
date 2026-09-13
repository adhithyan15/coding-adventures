# Authenticate a WebSocket control plane in application state, not in client claims

A portable WebSocket server cannot assume Unix-socket peer credentials, and the
RFC 6455 handshake layer may intentionally expose no HTTP headers to its
application handler. The clean boundary is a connection-local unauthenticated
state whose first successful application request exchanges one bounded opaque
credential through an injected authenticator. Store only the opaque authority
returned by that adapter, drop it with the connection, and ask the adapter again
for every operation; never treat a request field such as `principal` or `role`
as authority.

Two less obvious bounds matter at this seam:

1. A 64 KiB JSON limit applied *after* WebSocket assembly still lets the runtime
   allocate its larger default message limit. Clamp both frame and assembled
   message limits when binding the API, then retain the pre-parse check as
   defense in depth.
2. Recursive duplicate-key rejection implemented by scanning every prior key is
   quadratic. An attacker can spend the whole request budget on object keys, so
   use a per-object `HashSet` and keep the parser recursion-depth capped.

Encode revisions and nanosecond timestamps as decimal strings in JSON. They are
opaque or 64-bit values; passing them through an IEEE-754-backed JSON consumer
can otherwise silently destroy the exact compare-and-swap or health evidence.

The matching client codec belongs beside the server codec, not inside the CLI.
A hand-built CLI request can look correct while omitting response-version checks,
accepting the wrong request ID, or interpreting a malformed error envelope. A
typed sequential client should generate IDs, require an exact response ID, apply
the same size/depth/duplicate-key bounds in the reverse direction, and return
only a validated result or stable remote error. Then the terminal layer owns
only argument parsing, secure credential acquisition, and human rendering.

---
