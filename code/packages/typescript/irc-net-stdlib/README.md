# @coding-adventures/irc-net-stdlib

Node.js TCP event loop for the IRC stack — accepts connections, delivers bytes, no protocol knowledge.

## The Design

This package implements the "transport" layer: raw TCP in, lifecycle callbacks out.  It knows nothing about IRC protocol — that knowledge lives in `irc-server` and `irc-framing`.

```typescript
import { EventLoop, Handler, ConnId } from '@coding-adventures/irc-net-stdlib';

const loop = new EventLoop();

await loop.run("0.0.0.0", 6667, {
  onConnect(connId: ConnId, host: string) {
    console.log(`New connection ${connId} from ${host}`);
  },
  onData(connId: ConnId, data: Buffer) {
    // Feed raw bytes into a Framer, parse IRC messages, dispatch to IRCServer.
    loop.sendTo(connId, Buffer.from("PONG :server\r\n"));
  },
  onDisconnect(connId: ConnId) {
    console.log(`Connection ${connId} closed`);
  },
});
```

## Node.js Event Loop

Unlike the Python `irc-net-stdlib` (which spawns one OS thread per connection), this package uses Node.js's single-threaded async model:

- No threads, no locking needed
- All callbacks run sequentially on the event loop thread
- `sendTo()` uses `socket.write()` which is non-blocking

## Stack Position

```
ircd          ← wires all layers together
    ↓
irc-net-stdlib ← THIS PACKAGE: TCP connections
    ↓
irc-framing   ← byte-stream to lines
    ↓
irc-proto     ← parse/serialize
    ↓
irc-server    ← IRC state machine
```
