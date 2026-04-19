# udp-client

Transport-agnostic UDP datagram client — NET07.

`udp-client` wraps `std::net::UdpSocket` with the small bits of policy Venture
needs from a reusable datagram transport: resolved socket addresses,
configurable read/write timeouts, deterministic datagram-size guards, connected
and unconnected UDP modes, and structured errors.

It deliberately treats payloads as opaque bytes. DNS, games, QUIC experiments,
or local test fixtures can all use this package without teaching UDP about any
application protocol.

## Usage

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::Duration;
use udp_client::{send_and_receive, UdpClient, UdpOptions};

fn main() -> Result<(), udp_client::UdpError> {
    let server = UdpClient::bind(UdpOptions {
        bind_addr: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)),
        read_timeout: Some(Duration::from_secs(2)),
        write_timeout: Some(Duration::from_secs(2)),
        ..UdpOptions::default()
    })?;
    let server_addr = server.local_addr()?;

    thread::spawn(move || {
        let datagram = server.recv_from().expect("receive request");
        server
            .send_to(&datagram.payload, datagram.source)
            .expect("send response");
    });

    let response = send_and_receive(
        server_addr,
        b"\x12\x34 raw application bytes",
        UdpOptions {
            read_timeout: Some(Duration::from_secs(2)),
            ..UdpOptions::default()
        },
    )?;
    println!("received {} bytes", response.payload.len());

    Ok(())
}
```

## API

- `UdpClient::bind(options)` — open a UDP socket on a concrete local address
- `connect(remote)` — record a default UDP peer using the OS socket operation
- `send_to(payload, destination)` — send one datagram to an explicit peer
- `send(payload)` — send one datagram to the connected peer
- `recv_from()` — receive one datagram plus source/destination metadata
- `send_and_receive(destination, payload, options)` — simple one-shot request/response helper

## Spec

See `code/specs/NET07-udp-client.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
