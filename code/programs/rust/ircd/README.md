# ircd (Rust)

IRC server daemon — wires `irc-proto`, `irc-framing`, `irc-server`, and `irc-net-stdlib`.

## What it does

`ircd` is the top-level wiring layer.  It connects the pure IRC state machine (`irc-server`) to a real TCP network (`irc-net-stdlib`) by implementing the `Handler` trait.

## Architecture

```
TCP socket
   ↓ raw bytes
irc-net-stdlib EventLoop        ← TCP accept loop + threads
   ↓ on_data(conn_id, bytes)
DriverHandler                   ← THIS PROGRAM
   ↓ framing
irc-framing Framer              ← byte stream → complete lines
   ↓ decoded line
irc-proto parse()               ← line → Message
   ↓ Message
irc-server IRCServer            ← state machine → Vec<Response>
   ↓ Vec<Response>
irc-proto serialize()           ← Message → bytes
   ↓ bytes
irc-net-stdlib send_to()        ← bytes → socket
```

## Usage

```bash
# Default: bind to 0.0.0.0:6667
cargo run --release

# Custom configuration:
cargo run --release -- \
  --host 127.0.0.1 \
  --port 6668 \
  --server-name irc.example.com \
  --motd "Welcome to our IRC server!" \
  --oper-password supersecret
```

## Running tests

```bash
cargo test
```

Tests include integration tests that open real TCP connections, register clients, join channels, and exchange messages.
