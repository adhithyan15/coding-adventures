# mini-redis-python-worker

Python Mini Redis command worker for the Rust TCP runtime prototype.

The Rust side owns TCP sockets, native event-loop integration, RESP request
framing, and response writes. This package owns the Redis-like application
logic. That split lets us prove the shape we eventually want for Ruby, Python,
Perl, and other C-FFI/native-extension consumers: the language runtime can focus
on application jobs while Rust keeps the transport plane safe and fast.

## Protocol

The worker reads one generic job-protocol frame per line from stdin. Request
arguments are hex-encoded inside the payload so binary RESP data does not depend
on text encodings:

```json
{"version":1,"kind":"request","body":{"id":"job-1","payload":{"argv_hex":["50494e47"]},"metadata":{"affinity_key":"7","sequence":1}}}
```

The worker writes one generic response frame per line to stdout. Successful
responses carry a hex-encoded RESP frame inside the response payload:

```json
{"version":1,"kind":"response","body":{"id":"job-1","result":{"status":"ok","payload":{"resp_hex":"2b504f4e470d0a"}},"metadata":{"affinity_key":"7","sequence":1}}}
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

`SELECT` state is tracked by `metadata.affinity_key`, because database selection
belongs to the Redis session rather than to the global worker.

## Usage

```bash
python -m mini_redis_python_worker.stdio_worker
```

The companion Rust crate `embeddable-tcp-server` can start this worker as a
child process and delegate parsed RESP commands to it. Python Mini Redis is one
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
