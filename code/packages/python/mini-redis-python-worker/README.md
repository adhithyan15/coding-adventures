# mini-redis-python-worker

Python Mini Redis command worker for the Rust TCP runtime prototype.

The Rust side owns TCP sockets, native event-loop integration, and socket
writes. This package owns the Redis-like application protocol: job queueing,
per-stream buffering, RESP request parsing, Redis command execution, and RESP
response assembly. That split lets us prove the shape we eventually want for
Ruby, Python, Perl, and other C-FFI/native-extension consumers: the language
runtime can focus on application protocol jobs while Rust keeps the transport
plane safe and fast.

## Protocol

The worker reads one generic job-protocol frame per line from stdin. Rust sends
an opaque stream id and the TCP bytes read from that stream:

```json
{"version":1,"kind":"request","body":{"id":"job-1","payload":{"stream_id":"7","bytes_hex":"2a310d0a24340d0a50494e470d0a"},"metadata":{}}}
```

The worker writes one generic response frame per line to stdout. Successful
responses carry zero or more opaque byte chunks for Rust to write back to the
same stream, plus an optional close flag:

```json
{"version":1,"kind":"response","body":{"id":"job-1","result":{"status":"ok","payload":{"writes_hex":["2b504f4e470d0a"],"close":false}},"metadata":{}}}
```

This mirrors the Rust `generic-job-protocol` crate. The Python package does not
own the protocol; it is one language worker implementation that responds to the
shared `JobRequest` / `JobResponse` shape.

## Commands

The prototype supports the Mini Redis subset needed by the Rust TCP integration
tests:

- `PING`
- `SET`
- `GET`
- `EXISTS`
- `DEL`
- `INCRBY`
- `HSET`
- `HGET`
- `HEXISTS`
- `SELECT`

`SELECT` state is stored per `stream_id` inside the Python worker. The worker
knows only an opaque stream id, not a socket handle or native event-loop
resource.

## Usage

```bash
python -m mini_redis_python_worker.stdio_worker
```

The companion Rust crate `embeddable-tcp-server` can start this worker as a
child process and delegate raw TCP byte jobs to it. Python Mini Redis is one
consumer example of the generic embeddable TCP server seam rather than the
identity of the Rust transport crate.

## Development

```bash
# Run tests
bash BUILD
```

Local note: this package targets Python 3.12 for packaging. The pure worker
tests can also be run directly while developing:

```bash
PYTHONPATH=src python -m pytest tests/ -q
```
