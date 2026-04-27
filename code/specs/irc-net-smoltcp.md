# irc-net-smoltcp — Level 4: Userspace TCP via smoltcp (Rust Only)

## Overview

`irc-net-smoltcp` is the fourth layer of the Russian Nesting Doll and the deepest layer
available before the unikernel. It bypasses the OS TCP stack entirely.

In all previous layers, the Linux kernel managed TCP: the SYN/SYN-ACK/ACK handshake,
retransmission timers, sliding window flow control, congestion control (CUBIC, BBR), and
TIME_WAIT state. Our code called `recv()` and `send()` — the kernel did the rest.

Here, the kernel does nothing but forward raw Ethernet frames. **smoltcp** — a TCP/IP stack
written in pure Rust, designed for `no_std` environments — implements every TCP and IP layer
behaviour in userspace. The IRC application code is completely unchanged. Only the network layer
below the `Connection` interface is replaced.

This spec is **Rust only**. Python, Go, and other languages cannot reach this level because
their runtimes are tightly coupled to the OS TCP stack.

---

## Layer Position

```
ircd (program)
    ↓
irc-net-smoltcp         ← THIS SPEC: userspace TCP via smoltcp
    ↓
smoltcp (TCP/IP in userspace)
    ↓
TUN/TAP device (testing) or raw socket (production)
    ↓
Kernel: forwards Ethernet frames, nothing more
```

In the unikernel (next layer), the TUN/TAP device is replaced by a virtio-net driver that
communicates directly with the hypervisor's virtual NIC, bypassing the kernel entirely.

---

## Concepts

### What a Userspace TCP Stack Does

The OS kernel provides two things for TCP networking:
1. A TCP/IP protocol implementation (connection state machine, retransmission, etc.)
2. An API to access it (BSD socket calls: `socket`, `bind`, `listen`, `accept`, `recv`, `send`)

A userspace TCP stack provides item (1) without item (2). It reads raw bytes from the network
device, parses Ethernet/IP/TCP headers itself, and calls your application code directly.

Benefits of a userspace TCP stack:
- **No syscall overhead** per `recv`/`send` call (critical in unikernel mode)
- **Deterministic behaviour** (no kernel scheduler interference)
- **Full control** over TCP parameters (initial window size, congestion algorithm, timeouts)
- **`no_std` compatible** (required for bare-metal/unikernel operation)

### smoltcp Architecture

```
Application code (your IRC server)
    ↓ TcpSocket::recv() / TcpSocket::send()
SocketSet  ← holds all active TCP sockets
    ↓ Interface::poll()
smoltcp IP/TCP layer  ← parses/generates headers; manages state machines
    ↓ Device trait
TUN/TAP device (Linux) or virtio-net (unikernel)
    ↓
Raw Ethernet frames on the wire
```

The key API entry point is `Interface::poll()`. You call it in a tight loop (or after each
network event). It:
1. Reads available Ethernet frames from the device
2. Demultiplexes them to the correct TCP socket
3. Advances TCP state machines (retransmission, ACKs, connection establishment)
4. Calls application code when data is ready or when new connections arrive

### smoltcp Key Types

```rust
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::time::Instant;

// A device represents the "wire" — where Ethernet frames come and go.
// smoltcp provides a TunTapInterface for Linux development and testing.
// In the unikernel, you write your own Device implementation over virtio-net.
use smoltcp::phy::{Device, TunTapInterface, Medium};
```

### TUN vs TAP

| Feature | TUN | TAP |
|---|---|---|
| Layer | Network (IP) | Data link (Ethernet) |
| Frames | IP packets | Ethernet frames |
| Use with smoltcp | Not directly (smoltcp expects Ethernet) | Yes — smoltcp operates at Layer 2 |
| Setup | `ip tuntap add mode tun` | `ip tuntap add mode tap` |

smoltcp requires a TAP device on Linux (it speaks Ethernet). The test harness creates a TAP
interface, assigns it an IP address, and the IRC server binds to that address.

---

## smoltcp Event Loop

```rust
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{TunTapInterface, Medium};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer, State};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use std::collections::HashMap;

const LISTEN_PORT: u16 = 6667;
const BUF_SIZE: usize = 65536;

fn run(handler: &mut dyn Handler) {
    // 1. Open the TAP device (must already exist: `ip tuntap add dev tap0 mode tap`)
    let mut device = TunTapInterface::new("tap0", Medium::Ethernet).unwrap();

    // 2. Configure the interface
    let config = Config::new(EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]).into());
    let mut iface = Interface::new(config, &mut device, Instant::now());
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.push(IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24)).unwrap();
    });

    // 3. Create a listening socket
    let mut sockets = SocketSet::new(vec![]);
    let listener_handle = {
        let rx_buf = SocketBuffer::new(vec![0u8; BUF_SIZE]);
        let tx_buf = SocketBuffer::new(vec![0u8; BUF_SIZE]);
        let mut sock = TcpSocket::new(rx_buf, tx_buf);
        sock.listen(LISTEN_PORT).unwrap();
        sockets.add(sock)
    };

    let mut conn_map: HashMap<smoltcp::socket::SocketHandle, ConnState> = HashMap::new();
    let mut next_id: u32 = 0;

    loop {
        // 4. Advance smoltcp state machines; read/write device frames
        let timestamp = Instant::now();
        iface.poll(timestamp, &mut device, &mut sockets);

        // 5. Check the listener socket for new connections
        {
            let sock = sockets.get_mut::<TcpSocket>(listener_handle);
            if sock.state() == State::Established && !conn_map.contains_key(&listener_handle) {
                // New connection accepted
                let conn_id = ConnId(next_id);
                next_id += 1;
                conn_map.insert(listener_handle, ConnState::new(conn_id));
                handler.on_connect(conn_id);

                // Create a new listening socket to accept the next connection
                // (smoltcp: one TcpSocket = one connection once established)
                let rx_buf = SocketBuffer::new(vec![0u8; BUF_SIZE]);
                let tx_buf = SocketBuffer::new(vec![0u8; BUF_SIZE]);
                let mut new_listener = TcpSocket::new(rx_buf, tx_buf);
                new_listener.listen(LISTEN_PORT).unwrap();
                sockets.add(new_listener);
            }
        }

        // 6. Poll all established connections for data
        for (handle, state) in conn_map.iter_mut() {
            let sock = sockets.get_mut::<TcpSocket>(*handle);
            if sock.can_recv() {
                let mut buf = [0u8; 4096];
                let n = sock.recv_slice(&mut buf).unwrap();
                state.framer.feed(&buf[..n]);
                for frame in state.framer.frames() {
                    if let Ok(msg) = parse(std::str::from_utf8(&frame).unwrap_or("")) {
                        let responses = handler.on_message(state.conn_id, msg);
                        for (target_id, resp) in responses {
                            // find the handle for target_id and write to it
                            enqueue_write(&mut sockets, &conn_map, target_id, serialize(&resp));
                        }
                    }
                }
            }
            if !sock.is_open() {
                let responses = handler.on_disconnect(state.conn_id);
                // dispatch responses...
            }
        }

        // 7. Determine when to call poll() next (smoltcp's timer)
        let wait_duration = iface.poll_delay(Instant::now(), &sockets);
        std::thread::sleep(wait_duration.unwrap_or(smoltcp::time::Duration::from_millis(1)));
    }
}
```

### Key Difference from epoll: The Poll Model

With epoll, the kernel notifies us when data arrives. We are passive — we wait for the OS.

With smoltcp, **we drive the TCP state machine**. We call `Interface::poll()` ourselves, on
a schedule. smoltcp tells us the next deadline (via `poll_delay()`), and we sleep until then.
This is the completion model vs the readiness model: instead of "wake me when ready," we say
"I'll check back in Xms."

In the unikernel, this poll loop replaces the OS entirely. There is no scheduler to sleep in —
the poll loop spins until the NIC interrupt fires.

---

## Mapping smoltcp to the Stable Interfaces

smoltcp's `TcpSocket` is not an fd. It cannot be `close()`d in the POSIX sense. The mapping
requires a wrapper:

```rust
pub struct SmoltcpConnection {
    handle: smoltcp::socket::SocketHandle,
    id: ConnId,
    peer_addr: (String, u16),
    write_buf: Vec<u8>,
}

impl SmoltcpConnection {
    /// Called by the event loop to flush pending writes into the socket buffer.
    pub fn flush(&mut self, sockets: &mut SocketSet) {
        let sock = sockets.get_mut::<TcpSocket>(self.handle);
        while !self.write_buf.is_empty() && sock.can_send() {
            let sent = sock.send_slice(&self.write_buf).unwrap_or(0);
            self.write_buf.drain(..sent);
        }
    }
}
```

The `EventLoop` holds the `SocketSet` and `Interface`. The `Connection` protocol is satisfied
by `SmoltcpConnection`, which delegates to the socket via the event loop's `SocketSet`.

---

## Testing with TUN/TAP

### Setup (Linux only)

```bash
# Create a TAP device (requires root or CAP_NET_ADMIN)
sudo ip tuntap add dev tap0 mode tap user $USER
sudo ip addr add 192.168.69.2/24 dev tap0
sudo ip link set tap0 up

# The IRC server binds to 192.168.69.1 via smoltcp
# Connect a real IRC client from the host:
weechat -r "/connect 192.168.69.1 6667"
```

### Integration Test

The test harness:
1. Creates a TAP device via `ioctl` (no sudo needed in tests with user namespace)
2. Starts the smoltcp event loop in a thread
3. Opens a real `TcpStream` to `192.168.69.1:6667` from the host kernel
4. Sends IRC registration messages
5. Asserts the welcome sequence is received
6. Sends `JOIN #test` and `PRIVMSG #test :hello`
7. Verifies the echo is received

This tests the full stack: host kernel → TAP → smoltcp → irc-framing → irc-proto → irc-server.

---

## What You Learn at This Layer

| Question | Answer revealed here |
|---|---|
| What does the OS kernel do for TCP? | Everything: handshake, retransmission, flow control — now you do it |
| What is the TCP state machine? | smoltcp's `State` enum: `Listen`, `SynReceived`, `Established`, `FinWait1`, etc. |
| What is a socket buffer? | `SocketBuffer` — you size it explicitly; the OS normally hides this |
| What is `poll_delay()`? | The time until the next TCP timer fires (retransmission timeout, keepalive, etc.) |
| What are Ethernet frames? | The raw bytes that flow over the TAP device; smoltcp parses ARP, IPv4, TCP headers |
| Why does `no_std` matter? | smoltcp works here; it will also work in the unikernel where `std` is unavailable |

---

## Future: Peeling to the Unikernel

In this layer, the TAP device is still managed by the Linux kernel. To remove the kernel
entirely:

1. Replace `TunTapInterface` with a custom `Device` implementation that reads/writes virtio-net
   descriptor rings directly.
2. Remove all `std::thread::sleep` calls — the poll loop runs continuously, woken by NIC interrupts.
3. Compile with `#![no_std]`, `#![no_main]`, and a custom allocator.
4. Boot as a VM image.

See `irc-unikernel.md` for the full spec.
