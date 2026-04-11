# DT24 — TCP Server

## Overview

A TCP server is the bridge between your data structures and the outside world.
All the algorithms in DT00–DT23 are useless in isolation — they only become
valuable when a client can connect over a network and ask questions.

This spec builds a TCP server from first principles: raw socket syscalls,
non-blocking I/O, and an event loop. We go all the way down to the operating
system, then come back up to a clean handler API.

```
What the world sees:             What we're building:

redis-cli -p 6380 GET foo        ┌─────────────────────────────────────┐
        │                        │ TCP Server                           │
        │ TCP connection          │                                      │
        ▼                        │  socket → bind → listen → accept    │
┌───────────────┐                │  read bytes → call handler           │
│  mini-redis   │                │  write response bytes                │
│   (DT25)      │                │  close on disconnect                 │
└───────────────┘                └─────────────────────────────────────┘
```

The server has no opinion about the bytes it carries. It reads bytes from
a client, hands them to a handler function, and writes back whatever the
handler returns. The handler (DT25) is where RESP parsing and command
dispatch happen.

## Layer Position

```
DT22: bloom-filter     ← (not used by TCP server)
DT23: resp-protocol    ← pure encode/decode, no I/O

DT24: tcp-server       ← [YOU ARE HERE]
                          I/O layer. Handles connections, reads,
                          writes, and the event loop.
                          Knows nothing about RESP or Redis commands.

DT25: mini-redis       ← uses DT24 for connectivity and DT23 for framing
```

**Depends on:** DT23 (the server does not parse RESP itself — it passes raw
bytes to a handler and the handler uses DT23 to decode them).

**Does not depend on:** Any of DT00–DT22. The server is completely generic.
You could use it to build an HTTP server, an FTP server, or a chat server
with zero changes.

## Concepts

### The Seven Syscalls

Every TCP server in every language — regardless of whether it's written in
Python, Go, Rust, or C — is ultimately calling these seven operating system
functions. The language's standard library wraps them in convenient abstractions,
but understanding the raw syscalls removes all the mystery.

```
                         ┌────────────────────────────────────────┐
                         │           Operating System             │
                         │                                        │
  socket()  ─────────────┤ Create a new file descriptor.          │
                         │ Returns fd (just an integer, like 5).  │
                         │ Not connected to anything yet.         │
  bind()    ─────────────┤ Attach fd to an IP address + port.     │
                         │ "This fd owns 0.0.0.0:6380"            │
  listen()  ─────────────┤ Mark fd as a passive listener.         │
                         │ OS starts queuing incoming connections. │
  accept()  ─────────────┤ Dequeue one connection from the queue. │
                         │ Returns a NEW fd for that client.      │
                         │ The listening fd continues listening.  │
  read()    ─────────────┤ Receive bytes from a client fd.        │
                         │ Returns 0 when client disconnects.     │
  write()   ─────────────┤ Send bytes to a client fd.             │
                         │ May send fewer bytes than requested!   │
  close()   ─────────────┤ Tear down the fd. Free OS resources.   │
                         └────────────────────────────────────────┘
```

In C, these syscalls look like:

```c
int server_fd = socket(AF_INET, SOCK_STREAM, 0);
bind(server_fd, &addr, sizeof(addr));
listen(server_fd, 128);           // 128 = connection backlog
int client_fd = accept(server_fd, &client_addr, &addr_len);
ssize_t n = read(client_fd, buffer, sizeof(buffer));
write(client_fd, response, response_len);
close(client_fd);
```

The entire TCP server, reduced to its essence.

### The Full Server Lifecycle

```
Server startup:
                              OS Kernel
  socket() ─────────────────►│ allocate fd = 5           │
                              │ type = TCP, not bound     │
  bind(5, "0.0.0.0:6380") ──►│ fd 5 owns port 6380       │
  listen(5, backlog=128) ────►│ fd 5 accepts connections  │
                              │                           │

  ┌───────────────────────────────────────────────────────┐
  │                    Event Loop                         │
  │                                                       │
  │  accept(5) ◄────────────── client connects           │
  │  → fd = 7 (new client connection)                    │
  │                                                       │
  │  read(7, buf, 4096) → bytes from client              │
  │  → call handler(buf)                                  │
  │  → write(7, response, len(response))                  │
  │                                                       │
  │  read(7, buf, 4096) → returns 0 (client closed)      │
  │  → close(7)                                           │
  │                                                       │
  │  accept(5) ◄────────────── next client connects      │
  │  ...                                                  │
  └───────────────────────────────────────────────────────┘

Server shutdown:
  close(5)   ← stop accepting new connections
  close all active client fds
```

### The Concurrency Problem: C10K

"C10K" (10,000 simultaneous connections) was a famous 1999 paper identifying
that most servers of the era could not handle 10,000 concurrent connections.
The problem was the threading model.

**Approach 1: One Thread Per Connection**

```
  Connection 1 ──→ Thread A ──→ reads client bytes, processes, writes
  Connection 2 ──→ Thread B ──→ reads client bytes, processes, writes
  Connection 3 ──→ Thread C ──→ reads client bytes, processes, writes
  ...
  Connection 10,000 ──→ Thread J ──→ (10,000 threads!)

  Problem:
  - Each thread needs a stack: ~8 MB by default → 80 GB for 10K threads
  - Context switching between 10K threads: CPU overhead dominates
  - Most threads are idle (waiting for the next client message)
  - Wasting resources on threads that do nothing
```

**Approach 2: Busy Poll (Non-Blocking + Spin)**

```
  while True:
      for fd in all_fds:
          data = read(fd)  # non-blocking: returns immediately if no data
          if data:
              handle(data)

  Problem:
  - Burns 100% CPU even when all clients are idle
  - The loop runs millions of times per second doing nothing useful
  - Works fine in theory; catastrophic in practice
```

**Approach 3: Event Loop (the right answer)**

```
  Register all fds with the OS (epoll/kqueue)
  while True:
      ready_fds = epoll_wait()  ← OS puts us to sleep here
                                ← wakes us ONLY when a fd has data
      for fd in ready_fds:
          if fd == server_fd:   accept new connection, register it
          else:                 read client bytes, call handler, write

  Advantages:
  - Single thread handles thousands of connections
  - Sleeps when idle (zero CPU usage)
  - OS does the hard work of monitoring fds efficiently
  - redis uses this model and achieves >1M ops/sec on a single thread
```

### The Event Loop in Detail

The key insight: the OS maintains a watch list of file descriptors. You
register a fd with `epoll_ctl` (Linux) or `kqueue` (macOS/BSD), then call
`epoll_wait` or `kevent` to block until at least one fd is ready.

```
Linux (epoll):                    macOS/BSD (kqueue):
  epoll_create() → efd              kqueue() → kfd
  epoll_ctl(efd, ADD, server_fd)    kevent(kfd, changelist=[server_fd])
  loop:                             loop:
    events = epoll_wait(efd)          events = kevent(kfd, NULL, nevents)
    for event in events:              for event in events:
      handle(event.fd)                  handle(event.ident)

Windows (IOCP):
  CreateIoCompletionPort()
  GetQueuedCompletionStatus()
  (Different model: completion-based instead of readiness-based)
```

The interrupt chain: how a client message reaches your handler:

```
Client types "PING\r\n" and presses Enter.
        │
        ▼
  Network interface card (NIC) receives TCP segment.
        │
        ▼  (hardware interrupt)
  OS kernel: "data arrived on socket fd 7"
        │
        ▼  (kernel wakes the epoll/kqueue waiter)
  Your process: epoll_wait() returns [{fd: 7, event: READABLE}]
        │
        ▼
  Your code: read(7, buf) → b"*1\r\n$4\r\nPING\r\n"
        │
        ▼
  RESP parser (DT23): decode(buf) → ["PING"]
        │
        ▼
  Command dispatcher (DT25): handle("PING") → "+PONG\r\n"
        │
        ▼
  write(7, b"+PONG\r\n") → bytes sent back to client
```

### Blocking vs Non-Blocking I/O

By default, `read()` and `write()` are **blocking**: they halt your process
until the operation completes. This is fine for one-thread-per-connection,
but breaks the event loop model.

The fix: set fds to **non-blocking mode**:

```c
fcntl(fd, F_SETFL, O_NONBLOCK);
```

Now `read()` returns immediately with either:
- Bytes that were available (n > 0)
- `EAGAIN` / `EWOULDBLOCK` — no data yet, try again later
- 0 — client disconnected

And `write()` returns immediately with either:
- n bytes written (n may be less than requested!)
- `EAGAIN` / `EWOULDBLOCK` — kernel send buffer full, try again later

```
Non-blocking read lifecycle:

  epoll says fd 7 is readable
       │
       ▼
  n = read(7, buf, 4096)
       │
       ├── n > 0: we got n bytes, append to read_buffer[7]
       │          → try to parse a complete RESP message
       │          → if complete: dispatch command
       │          → if incomplete: wait for next readable event
       │
       ├── n == 0: client closed the connection
       │          → clean up, close(7)
       │
       └── n == -1, errno == EAGAIN: no data right now
                  → this shouldn't happen since epoll said readable,
                    but handle gracefully: just wait for next event
```

### The Partial Read / Write Problem

A single `read()` may return fewer bytes than a full RESP message.
A single `write()` may send fewer bytes than the full response.
Both must be handled correctly.

**Partial reads** — use a per-connection read buffer:

```
Connection state:
  read_buffer: bytes = b""   # accumulates bytes across read() calls

On READABLE event:
  chunk = read(fd, 4096)     # get however many bytes the OS gives us
  read_buffer += chunk       # append to accumulated buffer
  while True:
      msg, consumed = decode(read_buffer)   # DT23 decoder
      if consumed == 0:
          break              # incomplete RESP message, wait for more bytes
      read_buffer = read_buffer[consumed:]  # advance buffer
      response = handler(msg)
      write_all(fd, encode(response))       # DT23 encoder
```

**Partial writes** — loop until all bytes are sent:

```
write_all(fd, data):
    total_sent = 0
    while total_sent < len(data):
        n = write(fd, data[total_sent:])
        if n == -1:
            if errno == EAGAIN:
                # Kernel send buffer full. Options:
                # 1. Add to a per-connection write queue, wait for WRITABLE
                # 2. Busy-retry (simpler but burns CPU)
                continue    # simple approach for DT24
            raise IOError("write failed")
        total_sent += n
```

For high-throughput servers, option 1 (write queue + WRITABLE event) is
correct. For DT24/DT25 (educational implementation), the simple retry loop
is acceptable.

### Per-Language Event Loop Approaches

Every language has a different name for "event loop," but they all call
epoll/kqueue under the hood:

```
Language      Mechanism              How
────────────────────────────────────────────────────────────────────────
Python        asyncio                async/await, single-threaded event
                                     loop over epoll (Linux) or kqueue
                                     (macOS). asyncio.start_server().

TypeScript    Node.js (libuv)        libuv manages epoll/kqueue/IOCP.
              (no user code)         net.createServer(). Callbacks or
                                     async/await via Promises.

Ruby          Async gem /            Fiber-based concurrency. Event loop
              EventMachine           backed by nio4r (Java NIO wrapper)
                                     which wraps epoll/kqueue.

Rust          Tokio                  async/await over epoll via mio.
                                     tokio::net::TcpListener. Zero-cost
                                     abstractions compile to efficient poll.

Go            goroutines +           One goroutine per connection (not one
              net package            thread). Go runtime maps goroutines to
                                     OS threads. net.Listener uses epoll
                                     internally. Simplest to write.

Elixir        BEAM scheduler +       BEAM processes are extremely light
              :gen_tcp               (~300 bytes). One process per connection
                                     is idiomatic. :gen_tcp.accept loop in
                                     a dedicated acceptor process.

Lua           luasocket +            Blocking I/O with coroutines for
              coroutines             concurrency. copas library wraps
                                     luasocket select(). Limited scalability.

Perl          AnyEvent /             AnyEvent provides a portability layer
              IO::Async              over EV (libev), POE, or Glib event
                                     loops. Non-blocking with callbacks.

Swift         SwiftNIO               Apple's event-driven framework. Uses
                                     kqueue on Apple platforms, epoll on
                                     Linux. Channel pipeline model.
────────────────────────────────────────────────────────────────────────
```

## Representation

```
Server (stateful, one instance per server process):
  host: str                     # e.g. "127.0.0.1"
  port: int                     # e.g. 6380
  handler: Callable             # (Connection, bytes) → bytes
  server_fd: int                # the listening socket fd
  connections: dict[int, ConnectionState]   # fd → state

ConnectionState (one per connected client):
  fd: int                       # client socket fd
  address: (str, int)           # (remote_host, remote_port)
  read_buffer: bytes            # accumulated unprocessed bytes
  write_queue: list[bytes]      # data waiting to be written
  alive: bool                   # False after client disconnects

EventLoop (OS-level, managed internally):
  epoll_fd / kqueue_fd: int     # OS event queue handle
  watched_fds: set[int]         # fds we're monitoring
```

## Algorithms (Pure Functions)

The server itself is stateful (it wraps OS resources), but its core
logic functions are pure for testability.

### parse_address(address: str) → (str, int)

```
parse_address("127.0.0.1:6380")  → ("127.0.0.1", 6380)
parse_address("0.0.0.0:6380")    → ("0.0.0.0", 6380)
parse_address(":6380")           → ("0.0.0.0", 6380)
```

### should_close(data: bytes) → bool

```
# A read() return value of b"" means the client closed the connection.
should_close(data):
    return len(data) == 0
```

### drain_read_buffer(buffer: bytes, handler: Callable) → (bytes, list[bytes])

```
# Given a buffer of accumulated bytes, parse as many complete RESP
# messages as possible, call handler on each, collect responses.
# Returns (remaining_buffer, responses_to_send).
drain_read_buffer(buffer, handler):
    remaining = buffer
    responses = []
    while True:
        msg, consumed = decode(remaining)   # from DT23
        if consumed == 0:
            break
        remaining = remaining[consumed:]
        response = handler(msg)
        responses.append(encode(response))  # from DT23
    return remaining, responses
```

This is a pure function — it takes a buffer and returns a new buffer plus
a list of encoded responses. The actual `write()` calls happen separately.

## Public API

```python
from typing import Callable


class Connection:
    """
    Represents one connected client.
    Passed to the handler so it can send data back or close the connection.
    """

    def send(self, data: bytes) -> None:
        """
        Queue data to be sent to this client.
        The server's event loop will write it when the socket is ready.
        """

    def close(self) -> None:
        """
        Gracefully close this connection after all queued writes are sent.
        """

    @property
    def address(self) -> tuple[str, int]:
        """The remote (host, port) of the connected client."""

    @property
    def is_alive(self) -> bool:
        """False if the client has disconnected or close() was called."""


class TcpServer:
    """
    A simple event-loop TCP server.

    Usage:
        def my_handler(conn: Connection, data: bytes) -> None:
            # data is raw bytes from the client.
            # Call conn.send() to reply.
            conn.send(b"echo: " + data)

        server = TcpServer("127.0.0.1", 6380, my_handler)
        server.start()  # blocks until server.stop() is called
    """

    def __init__(
        self,
        host: str,
        port: int,
        handler: Callable[[Connection, bytes], None],
        *,
        backlog: int = 128,
        read_chunk_size: int = 4096,
    ) -> None:
        """
        host: IP address to bind to. "127.0.0.1" for localhost only,
              "0.0.0.0" for all interfaces.
        port: TCP port number (1–65535). Ports below 1024 require root.
        handler: called with (Connection, bytes) for each chunk of data
                 received from a client. Handler calls conn.send() to reply.
        backlog: maximum number of queued connection attempts.
        read_chunk_size: bytes to read per read() syscall.
        """

    def start(self) -> None:
        """
        Bind, listen, and run the event loop. Blocks the calling thread.
        Handles KeyboardInterrupt (Ctrl+C) gracefully.
        """

    def stop(self) -> None:
        """
        Signal the event loop to stop after the current iteration.
        Safe to call from a signal handler or another thread.
        """

    @property
    def address(self) -> tuple[str, int]:
        """The (host, port) this server is bound to."""

    @property
    def connection_count(self) -> int:
        """Number of currently connected clients."""
```

The handler receives raw bytes. It is the handler's responsibility to
parse RESP messages (using DT23) and build responses. This separation means
DT24 has no dependency on DT23 — they compose at the DT25 layer.

```python
# Example: echo server
def echo_handler(conn: Connection, data: bytes) -> None:
    conn.send(data)

# Example: RESP-aware handler (the DT25 pattern)
def redis_handler(conn: Connection, data: bytes) -> None:
    conn.read_buffer += data
    while True:
        msg, consumed = decode(conn.read_buffer)   # DT23
        if consumed == 0:
            break
        conn.read_buffer = conn.read_buffer[consumed:]
        response = dispatch_command(msg)            # DT25
        conn.send(encode(response))                 # DT23
```

## Composition Model

The TCP server is inherently stateful (OS sockets), so it cannot be a pure
function. But we isolate the I/O effects behind the `TcpServer` class and
keep the business logic (parsing, dispatch) in pure functions.

### Python — asyncio

```python
import asyncio

class AsyncTcpServer:
    def __init__(self, host, port, handler):
        self.host, self.port, self.handler = host, port, handler
        self._server = None

    async def _client_connected(self, reader, writer):
        addr = writer.get_extra_info("peername")
        conn = AsyncConnection(writer, addr)
        read_buffer = b""
        try:
            while True:
                chunk = await reader.read(4096)
                if not chunk:
                    break
                read_buffer += chunk
                self.handler(conn, chunk)  # handler uses conn.send()
        finally:
            writer.close()
            await writer.wait_closed()

    async def start_async(self):
        self._server = await asyncio.start_server(
            self._client_connected, self.host, self.port
        )
        async with self._server:
            await self._server.serve_forever()

    def start(self):
        asyncio.run(self.start_async())
```

### Go — One Goroutine Per Connection

```go
// Go's net package handles epoll internally.
// Each connection gets its own goroutine — simpler than manual epoll.
// The Go runtime multiplexes goroutines onto OS threads efficiently.

type TcpServer struct {
    host, port string
    handler    func(*Connection, []byte)
    listener   net.Listener
    done       chan struct{}
}

func (s *TcpServer) Start() {
    var err error
    s.listener, err = net.Listen("tcp", s.host+":"+s.port)
    if err != nil { panic(err) }
    defer s.listener.Close()
    for {
        conn, err := s.listener.Accept()
        select {
        case <-s.done: return
        default:
        }
        if err != nil { continue }
        go s.handleConn(conn)  // one goroutine per client
    }
}

func (s *TcpServer) handleConn(netConn net.Conn) {
    defer netConn.Close()
    conn := &Connection{netConn: netConn, addr: netConn.RemoteAddr()}
    buf := make([]byte, 4096)
    for {
        n, err := netConn.Read(buf)
        if err != nil { return }
        s.handler(conn, buf[:n])
    }
}
```

### Rust — Tokio Async

```rust
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct TcpServer {
    host: String,
    port: u16,
}

impl TcpServer {
    pub async fn start<F, Fut>(&self, handler: F)
    where
        F: Fn(Arc<Connection>, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Vec<u8>> + Send,
    {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();
        let handler = Arc::new(handler);
        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            let handler = Arc::clone(&handler);
            tokio::spawn(async move {
                handle_connection(stream, addr, handler).await;
            });
        }
    }
}

async fn handle_connection(mut stream: TcpStream, addr: SocketAddr, handler: Arc<impl Fn(Arc<Connection>, Vec<u8>) -> impl Future<Output = Vec<u8>>>) {
    let mut buf = vec![0u8; 4096];
    let conn = Arc::new(Connection::new(addr));
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,    // client disconnected
            Ok(n) => {
                let response = handler(Arc::clone(&conn), buf[..n].to_vec()).await;
                if stream.write_all(&response).await.is_err() { break; }
            }
            Err(_) => break,
        }
    }
}
```

### Elixir — One Process Per Connection

```elixir
defmodule TcpServer do
  def start(port, handler) do
    {:ok, listen_socket} = :gen_tcp.listen(port, [
      :binary, active: false, reuseaddr: true
    ])
    accept_loop(listen_socket, handler)
  end

  defp accept_loop(listen_socket, handler) do
    {:ok, client_socket} = :gen_tcp.accept(listen_socket)
    # Spawn a new process per connection — extremely lightweight in BEAM
    spawn(fn -> handle_client(client_socket, handler, <<>>) end)
    accept_loop(listen_socket, handler)  # tail-recursive, no stack growth
  end

  defp handle_client(socket, handler, buffer) do
    case :gen_tcp.recv(socket, 0) do
      {:ok, data} ->
        new_buffer = buffer <> data
        {responses, remaining} = handler.(new_buffer)
        Enum.each(responses, &:gen_tcp.send(socket, &1))
        handle_client(socket, handler, remaining)
      {:error, _} ->
        :gen_tcp.close(socket)
    end
  end
end
```

## Test Strategy

### Basic Echo

```python
def test_echo_server():
    """Server echoes back whatever it receives."""
    def echo(conn, data):
        conn.send(data)

    server = TcpServer("127.0.0.1", 16380, echo)
    with run_server_in_thread(server):
        with socket.create_connection(("127.0.0.1", 16380)) as s:
            s.sendall(b"hello world")
            response = s.recv(1024)
            assert response == b"hello world"
```

### Multiple Concurrent Connections

```python
def test_multiple_clients():
    """Server handles N clients concurrently."""
    import threading

    def echo(conn, data):
        conn.send(data)

    server = TcpServer("127.0.0.1", 16381, echo)
    with run_server_in_thread(server):
        results = {}
        errors = []

        def client_task(i):
            try:
                with socket.create_connection(("127.0.0.1", 16381)) as s:
                    msg = f"client_{i}".encode()
                    s.sendall(msg)
                    results[i] = s.recv(1024)
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=client_task, args=(i,)) for i in range(50)]
        for t in threads: t.start()
        for t in threads: t.join()

        assert not errors
        for i in range(50):
            assert results[i] == f"client_{i}".encode()
```

### Partial Read Buffering

```python
def test_partial_reads():
    """Server correctly buffers fragmented RESP messages."""
    received = []

    def resp_handler(conn, data):
        conn.read_buffer = getattr(conn, "read_buffer", b"") + data
        while True:
            msg, n = decode(conn.read_buffer)
            if n == 0: break
            conn.read_buffer = conn.read_buffer[n:]
            received.append(msg)
            conn.send(encode("OK"))

    server = TcpServer("127.0.0.1", 16382, resp_handler)
    with run_server_in_thread(server):
        with socket.create_connection(("127.0.0.1", 16382)) as s:
            full_msg = encode(["SET", "k", "v"])   # 28 bytes
            # Send one byte at a time to force partial reads
            for i in range(len(full_msg)):
                s.send(bytes([full_msg[i]]))
                time.sleep(0.001)
            response = s.recv(1024)
            assert response == encode("OK")

    assert received == [[b"SET", b"k", b"v"]]
```

### Clean Disconnection

```python
def test_client_disconnect():
    """Server handles client disconnect without crashing."""
    server = TcpServer("127.0.0.1", 16383, lambda conn, data: None)
    with run_server_in_thread(server):
        s = socket.create_connection(("127.0.0.1", 16383))
        s.close()   # immediate disconnect
        time.sleep(0.1)
        # Server should still be running and accepting new connections
        with socket.create_connection(("127.0.0.1", 16383)) as s2:
            assert s2  # successfully connected
```

### Connection Count

```python
def test_connection_count():
    server = TcpServer("127.0.0.1", 16384, lambda conn, data: None)
    with run_server_in_thread(server):
        assert server.connection_count == 0
        s1 = socket.create_connection(("127.0.0.1", 16384))
        s2 = socket.create_connection(("127.0.0.1", 16384))
        time.sleep(0.05)
        assert server.connection_count == 2
        s1.close()
        time.sleep(0.05)
        assert server.connection_count == 1
        s2.close()
```

### drain_read_buffer Pure Function Tests

```python
def test_drain_single_message():
    buf = encode(["PING"])   # b"*1\r\n$4\r\nPING\r\n"
    responses = []
    def handler(msg): return "PONG"
    remaining, resps = drain_read_buffer(buf, handler)
    assert remaining == b""
    assert len(resps) == 1

def test_drain_multiple_messages():
    buf = encode(["PING"]) + encode(["PING"]) + encode(["PING"])
    calls = []
    def handler(msg):
        calls.append(msg)
        return "PONG"
    remaining, resps = drain_read_buffer(buf, handler)
    assert remaining == b""
    assert len(calls) == 3

def test_drain_incomplete_message():
    full = encode(["SET", "k", "v"])   # complete message
    partial = encode(["GET", "k"])[:5] # incomplete
    buf = full + partial
    calls = []
    def handler(msg):
        calls.append(msg)
        return "OK"
    remaining, resps = drain_read_buffer(buf, handler)
    assert remaining == partial
    assert len(calls) == 1
```

## Future Extensions

**TLS/SSL:** Wrap the TCP socket with TLS using the OS SSL library (Python's
`ssl.wrap_socket`, Rust's `tokio-rustls`, Go's `tls.Server`). Redis supports
TLS since version 6.0. The server API stays the same — TLS is transparent
to the handler.

**Connection Limits:** Track `max_connections` and reject new connections
with an error response when the limit is reached. Prevents memory exhaustion
under load.

**Timeouts:** Per-connection idle timeout. If a client sends no data for N
seconds, close the connection. Implemented via a min-heap (DT04) keyed on
last-activity timestamp, polled on each event loop iteration.

**Write Queues:** For truly non-blocking writes, maintain a per-connection
queue of pending data. Register the fd for WRITABLE events; flush the queue
when the fd becomes writable. This allows the server to continue handling
other fds even when one client's send buffer is full.

**SO_REUSEPORT:** Setting this socket option allows multiple processes to
bind to the same port. The OS load-balances incoming connections across
processes. This is how nginx and Redis Cluster achieve multi-core scaling
without a single-threaded bottleneck.

**Connection Multiplexing:** Instead of one handler call per read(), buffer
entire RESP messages (using DT23) and call the handler once per complete
command. This is the pattern DT25 uses — the TCP server does not need to
know about RESP, but DT25's handler accumulates bytes until a message is
complete before dispatching.

**Graceful Shutdown:** On SIGTERM, stop accepting new connections, wait for
all in-flight requests to complete, then close all connections. Ensures no
commands are half-executed when the server restarts.
