# irc-net-reactor — Level 3: All-Rust IRC engine on the home-grown reactor

## Overview

`irc-net-reactor` is the layer that finally puts the IRC server on the
**home-grown high-performance TCP runtime** we built from scratch
(`tcp-runtime` → `transport-platform` → raw `kqueue`/`epoll`/IOCP), with **every
line of logic in Rust**.

Where `irc-net-stdlib` (Level 1) spends one OS thread per connection and
`irc-net-selectors` (Level 2) leans on the language's stdlib selector,
`irc-net-reactor` runs the IRC state machine directly on the native event loop
that the rest of this repository's networking stack is built on — the same
engine that powers `mini-redis` and `conduit`/`web-core`.

Crucially, this crate is also the **engine that every language embeds natively**.
A Python, Ruby, or Node program does not re-implement IRC; it loads a thin native
extension (via the repo's zero-dependency `python-bridge`/`ruby-bridge`/
`node-bridge`) that wraps `IrcReactorServer` and exposes a three-call control
surface: **create, serve, stop**. All the IRC and TCP work happens in Rust.

```
irc-server (state machine) + irc-proto + irc-framing      ← reused unchanged
        ↓ wired by
irc-net-reactor   (THIS SPEC: IRCServer on tcp-runtime, broadcast via TcpMailbox)
        ↓ runs on
tcp-runtime → transport-platform → kqueue / epoll / IOCP
        ↑ embedded via per-language native bridge (create / serve / stop)
   python-bridge   node-bridge   ruby-bridge   … (jni / objc / erl-nif / perl later)
```

---

## Why a new layer

The reactor (`tcp-runtime`) already exists and is proven by `mini-redis`. But
`mini-redis` is a strictly **per-connection request/response** server: each
connection only ever writes back to itself. IRC is fundamentally a **broadcast**
server — when one client sends `PRIVMSG #chan`, the bytes must reach *other*
clients' sockets.

The reactor supports this natively: `TcpRuntime::mailbox()` returns a
`TcpMailbox` whose `send(connection_id, bytes)` targets **any** connection id,
not just the one currently being handled. `irc-net-reactor` is the first server
in the repo to use the mailbox for genuine fan-out.

---

## Engine design

Per-connection state is an `irc_framing::Framer` (TCP delivers a byte stream;
the framer reassembles CRLF-terminated IRC lines). One global
`Arc<Mutex<IRCServer>>` holds the channel/nick state — `irc-server` documents
itself as *not* thread-safe, so the transport serializes all access behind the
mutex. The `TcpMailbox` is captured by the connection callbacks through an
`Arc<OnceLock<TcpMailbox>>` that is filled immediately after `bind` and before
`serve` — connections are only accepted inside `serve`, so the cell is always
populated when a callback fires.

The three reactor callbacks map one-to-one onto the three `IRCServer` methods,
and every outbound `Response{conn_id, msg}` is delivered uniformly through the
mailbox (`serialize(msg)` → `mailbox.send(conn_id, …)`), including replies to the
sender. This single delivery path is what makes broadcast and direct replies the
same code:

| Reactor callback                | IRC action                                   |
|---------------------------------|----------------------------------------------|
| `init(info) -> Framer`          | `IRCServer::on_connect(id, peer_ip)`; new `Framer` |
| `handler(info, framer, bytes)`  | feed framer → `parse` each line → `IRCServer::on_message` → deliver |
| `on_close(info, framer)`        | `IRCServer::on_disconnect(id)` → deliver (QUIT broadcast) |

Unparseable lines are skipped silently (RFC-traditional behaviour), exactly as
`irc-net-stdlib`'s Rust driver does. Bytes are decoded with lossy UTF-8 so a
single bad byte never drops a connection.

---

## Public API

```rust
pub struct IrcConfig {
    pub host: String,           // bind address, e.g. "127.0.0.1"
    pub port: u16,              // 0 = OS-assigned ephemeral port (tests)
    pub server_name: String,    // shown in 001 welcome + message prefixes
    pub motd: Vec<String>,      // Message of the Day lines
    pub oper_password: String,  // OPER password; "" disables OPER
    pub max_connections: usize, // connection cap
}

pub struct IrcReactorServer { /* runtime, local_addr, stop_handle, serving */ }

impl IrcReactorServer {
    pub fn bind(config: IrcConfig) -> std::io::Result<Self>; // binds the listener eagerly
    pub fn local_addr(&self) -> std::net::SocketAddr;        // real port after bind
    pub fn serve(&self) -> std::io::Result<()>;              // blocks on kqueue/epoll/IOCP
    pub fn stop(&self);                                      // unblocks serve()
    pub fn is_running(&self) -> bool;
}
```

This mirrors `mini-redis`'s `MiniRedisServer` shape (`new`/`serve`/`stop`/
`address`/`is_running`) so the native bindings can follow the proven `conduit`
embedding pattern.

---

## Native embedding (the point of "all logic in Rust")

Each language ships a `*-native` package wrapping `IrcReactorServer`:

- `server_new(config) -> handle` — build + bind, wrap the boxed engine
  (PyCapsule / N-API external / Ruby TypedData).
- `server_serve(handle)` — **release the GIL** (Python `PyEval_SaveThread`),
  **release the GVL** (Ruby `rb_thread_call_without_gvl`), or run on a
  **background thread** (Node, per the N-API single-thread rule) around the
  blocking `engine.serve()`.
- `server_stop(handle)` — `StopHandle::stop`.
- `server_local_addr(handle)` — for ephemeral-port tests.
- `server_dispose(handle)` — drop the engine.

Because all logic is in Rust, there is **no per-message callback into the host
language** — unlike `conduit`, which dispatches each HTTP route to a language
handler. That makes these bindings strictly simpler (no TSFN, no GIL
re-acquisition for dispatch).

Proven binding templates first: **Python**, **Ruby**, **Node/TypeScript**
(mirroring the three existing `conduit` packages). Java/Kotlin (`jni-bridge`),
Swift (`objc-bridge`), Elixir (`erl-nif-bridge`), and Perl (`perl-bridge`) have
bridge crates but no example native package yet, so they follow as their own
later PRs.

---

## Concurrency & ordering

- **Sharded by default.** The engine runs on a `ShardedTcpRuntime` with one
  reactor shard per CPU (`bind`); `bind_with_worker_count(config, n)` pins the
  count, and `n = 1` reproduces the original single-reactor engine. Each shard is
  an independent reactor on its own thread with its own kqueue/epoll/IOCP
  instance; the kernel load-balances accepted connections across them via
  `SO_REUSEPORT`.
- **One shared brain.** All shards share a single `Arc<Mutex<IRCServer>>`, so the
  nick/channel namespaces stay server-global. The mutex is held only for the
  per-message state transition (`on_connect` / `on_message` / `on_disconnect`);
  message serialization and mailbox dispatch happen *after* the guard is dropped,
  so the serialized critical section is small relative to the parallel work
  (accept, read, CRLF framing, parse, socket writes).
- **Cross-shard delivery is automatic.** Every `ConnectionId` encodes its owning
  shard in its low bits, so `TcpMailbox` routes a response to the reactor that
  owns the target connection — a client on shard A can broadcast to a client on
  shard B with no shared connection registry.
- Per-connection frame ordering is preserved because each connection is owned by
  exactly one reactor and its `Framer` lives in that reactor's per-connection
  state; connections never migrate between shards.

---

## Known limitations

- **Backpressure**: writes are enqueued on the mailbox without IRC-level flow
  control. A pathologically slow reader can accumulate queued bytes up to the
  runtime's `max_pending_write_bytes`, after which the runtime drops/closes per
  its own policy. Adequate for a teaching IRC server; finer per-client
  backpressure is future work.
- **TLS**: none — this is plaintext IRC (port 6667 semantics), matching the rest
  of the `irc-net-*` family.

---

## Testing

Real-socket integration tests (modelled on `mini-redis`'s test harness): bind on
port 0, `serve()` on a background thread, connect two `TcpStream` clients, and
assert:

1. **Registration** — NICK/USER yields the 001 welcome burst.
2. **Broadcast** — both clients `JOIN #test`; client A `PRIVMSG`s; **client B
   receives it** (the fan-out proof, exercising `TcpMailbox.send` to a
   non-sender connection).
3. **QUIT broadcast** — a client disconnects; remaining members see the QUIT.
4. **PING/PONG** — liveness round-trip.

The per-language bindings each repeat the broadcast scenario against a server
started from that language, proving the native control surface end to end.
