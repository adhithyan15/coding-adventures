# irc-net-stdlib (Go)

Goroutine-per-connection TCP event loop for IRC servers.

## Overview

`irc-net-stdlib` is the networking layer of the IRC stack. It accepts TCP
connections, reads raw bytes off the wire, and dispatches lifecycle events to
a `Handler` interface. It knows nothing about IRC framing or protocol — that
is left to the caller.

Thread-safety: the `Handler` callbacks are serialised through `handlerMu` so
the handler implementation never needs its own locking for handler methods.
The connection map (`conns`) is protected by a separate `connsMu` so that
`SendTo` can proceed concurrently with handler dispatch.

## Architecture

```
TCP listener
  |
  +-- goroutine per connection
        |
        +-- OnConnect(connID, host)
        +-- OnData(connID, data)     (called for every read)
        +-- OnDisconnect(connID)
```

The `EventLoop` manages connection lifecycle. `SendTo` can be called from any
goroutine; it writes directly to the connection's `net.Conn`.

## API

```go
loop := irc_net_stdlib.NewEventLoop()

// handler is your irc_server adapter
err := loop.Run(":6667", handler)   // blocks until Stop() is called
loop.Stop()

// Send bytes to a specific connection (from any goroutine).
loop.SendTo(connID, data)
```

## Handler interface

```go
type Handler interface {
    OnConnect(connID ConnID, host string)
    OnData(connID ConnID, data []byte)
    OnDisconnect(connID ConnID)
}
```

## Coverage

96%+ statement coverage.
