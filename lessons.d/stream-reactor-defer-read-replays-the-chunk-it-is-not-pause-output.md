# stream-reactor `defer_read` REPLAYS the chunk — it is not "pause output" (WEB01b-1a, PR #6047)

`MailboxHttpServer` (the new mailbox/deferred-response HTTP server) framed a
request, submitted it to the worker pool, and returned
`TcpHandlerResult::defer_read()` intending "pause this connection's reads until
the response is written." The test passed locally on macOS but failed on the
Linux CI runner: the client received a spurious `400 Bad Request` written
*before* the real `200 OK` on the same connection.

Root cause: in `code/packages/rust/stream-reactor`, `defer_read` does **not**
mean "pause output." It means **"I did NOT consume these bytes — buffer this
chunk and *replay* it (re-invoke the handler with the same bytes) when reads
resume."** The mailbox response-router calls `resume_all_reads()` for *every*
connection's response, so a deferred chunk gets replayed (`progress_reads_with_state`
→ `apply_read_chunk`, stream-reactor/src/lib.rs:651-668,720-724). Because the
handler had already *consumed* the bytes (drained its buffer + submitted the
job), the replay re-fed the chunk. On macOS the whole request arrived in one TCP
segment, so the replay merely double-submitted (the leading `200` still satisfied
the assertion). On Linux under load the request was TCP-segmented, so the
replayed **trailing fragment** (`ection: close\r\n\r\n`) parsed as a malformed
head → `pop_request` returned `Err` → a `400` was queued before the real `200`.

Fixes:
1. After a successful consume+submit, return `TcpHandlerResult::default()`
   (keep reading) — **never** `defer_read()`. Only return `defer_read` when you
   genuinely did NOT consume the bytes and want them replayed (e.g. the
   QueueFull backpressure case in embeddable-tcp-server). There is currently no
   "pause-without-replay" primitive; a per-connection in-flight gate needs a
   reorder buffer (deferred to WEB01b-1b).
2. Drain **every** complete request a read delivered by looping `pop_request`
   until `Ok(None)` — one TCP read can carry multiple coalesced/pipelined
   requests; popping once strands the extras in the buffer and hangs a client
   that sent them together and then waited. (Caught by the security review.)

Lesson: a handler-result flag named for an *intent* ("defer") may be implemented
as a *mechanism* ("replay") — read the reactor's drain path before reusing it,
and never trust a same-host test to expose a TCP-segmentation-dependent bug
(loopback coalescing hides it; the Linux runner under parallel load splits the
read and surfaces it).
