# irc-net-stdlib

Stdlib TCP event loop for IRC: one OS thread per connection.

## What it does

`irc-net-stdlib` provides the concrete TCP networking layer for the IRC stack. It accepts connections, spawns one thread per connection, and calls three `Handler` callbacks for lifecycle events.

## Layer position

```
irc-net-stdlib   ← THIS CRATE: TcpListener + threads
irc-server       ← IRC state machine
irc-framing      ← byte stream → complete lines
irc-proto        ← parse() / serialize()
```

## Usage

```rust
use irc_net_stdlib::{EventLoop, Handler, ConnId};
use std::sync::Arc;

struct MyHandler {
    event_loop: Arc<EventLoop>,
}

impl Handler for MyHandler {
    fn on_connect(&self, conn_id: ConnId, host: &str) {
        self.event_loop.send_to(conn_id, b":irc.local NOTICE * :Welcome\r\n");
    }
    fn on_data(&self, _conn_id: ConnId, _data: &[u8]) {
        // Typically: feed into Framer, parse Message, call IRCServer::on_message
    }
    fn on_disconnect(&self, _conn_id: ConnId) {}
}

let el = Arc::new(EventLoop::new());
let handler = Arc::new(MyHandler { event_loop: Arc::clone(&el) });

// This blocks until stop() is called:
el.run("0.0.0.0:6667", handler).unwrap();
```

## Concurrency model

- One OS thread per connection (worker thread)
- `handler_lock: Mutex<()>` serializes all `Handler` calls — the `IRCServer` inside need not be thread-safe
- `conns_lock: Mutex<HashMap<ConnId, TcpStream>>` protects the connection map
- `send_to()` acquires `conns_lock` briefly for a stream clone, then writes outside the lock

## Running tests

```bash
cargo test -p irc-net-stdlib
```
