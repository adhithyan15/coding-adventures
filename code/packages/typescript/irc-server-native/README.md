# coding-adventures-irc-server-native

A **high-performance IRC server for Node.js / TypeScript** — every line of IRC
and TCP logic runs in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor)
engine on the home-grown `kqueue`/`epoll` reactor). Node only launches and
controls the server, through an N-API addon built on the zero-dependency
`node-bridge` (no napi-rs / neon).

## Why

This is the Node member of a family: one Rust IRC engine, embedded natively in
every language. Because all logic lives in Rust, the binding is a pure lifecycle
control surface — **create, serve, stop** — with no per-message callback into
JavaScript (and therefore none of the threadsafe-function machinery a
callback-based framework like `conduit` needs).

## Usage

```ts
import { IrcServer } from "coding-adventures-irc-server-native";

const server = new IrcServer({ port: 6667 });
server.serve();   // returns immediately; the loop runs on a background thread
// ... later ...
server.stop();
```

Bind an ephemeral port (handy for tests):

```ts
const server = new IrcServer({ port: 0 });
server.serve();
console.log(`listening on ${server.localAddr}`);
// ... connect IRC clients to server.localHost:server.localPort ...
server.stop();
```

Point an IRC client (irssi, WeeChat, `nc`) at the host/port, register with
`NICK`/`USER`, `JOIN #channel`, and chat — the broadcast fan-out to other
channel members happens entirely in Rust.

## API

`IrcServer` constructor options: `host`, `port`, `serverName`, `motd`,
`operPassword`, `maxConnections` (all optional).

| member          | description                                                |
|-----------------|------------------------------------------------------------|
| `serve()`       | start the event loop on a background thread (non-blocking) |
| `stop()`        | signal the loop to stop and join the thread                |
| `dispose()`     | release the engine (must be stopped first)                 |
| `running`       | whether the loop is currently running                      |
| `localHost` / `localPort` / `localAddr` | the bound address                  |

`serve()` does **not** block Node's event loop — the blocking I/O loop runs on a
spawned background OS thread, so your program stays responsive.

## How it's built

`BUILD` compiles the Rust cdylib in `ext/irc_native_node`, copies it to the
package root as `irc_native_node.node`, then `npm ci`, `tsc`, and `vitest`. The
addon is loaded with `createRequire(import.meta.url)`.

Windows CI is skipped (`BUILD_windows`) because Node native addons need
`node.lib` to link; Linux and macOS runners cover the package.

## Resilience

`serve()` runs on an **owned clone** of the engine, so a later `dispose()`
cannot free the engine out from under the background thread. The underlying
reactor closes a single connection (never the whole loop) when a client resets,
so one crashed or hostile client can't take the server down.

## Layer position

```
IrcServer (this package — TS facade)
        ↓ irc_native_node N-API addon (node-bridge, background-thread serve)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
