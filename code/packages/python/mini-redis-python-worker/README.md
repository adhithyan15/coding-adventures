# mini-redis-python-worker

Python Mini Redis command worker for the Rust TCP runtime prototype.

The Rust side owns TCP sockets, native event-loop integration, RESP request
framing, and response writes. This package owns the Redis-like application
logic. That split lets us prove the shape we eventually want for Ruby, Python,
Perl, and other C-FFI/native-extension consumers: the language runtime can focus
on application jobs while Rust keeps the transport plane safe and fast.

## Protocol

The worker reads one JSON object per line from stdin. Request arguments are
hex-encoded bytes so binary RESP payloads do not depend on text encodings:

```json
{"id":"job-1","connection_id":"7","sequence":1,"argv_hex":["50494e47"]}
```

The worker writes one JSON object per line to stdout. Successful responses carry
a hex-encoded RESP frame:

```json
{"id":"job-1","connection_id":"7","sequence":1,"ok":true,"resp_hex":"2b504f4e470d0a"}
```

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

`SELECT` state is tracked by `connection_id`, because database selection belongs
to the Redis session rather than to the global worker.

## Usage

```bash
python -m mini_redis_python_worker.stdio_worker
```

The companion Rust crate `python-mini-redis-tcp` starts this worker as a child
process and delegates parsed RESP commands to it.

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
