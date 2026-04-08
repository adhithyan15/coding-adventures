# resp-protocol

Pure Python implementation of the RESP2 (Redis Serialization Protocol) wire format.

## What it does

Encodes Python values to RESP bytes and decodes RESP bytes back to Python values.
This is the serialization layer used by every Redis client and server — the bytes
that flow over the TCP connection between `redis-py` and `redis-server`.

## Supported types

| RESP type | Wire format | Python type |
|---|---|---|
| Simple String | `+OK\r\n` | `str` |
| Error | `-ERR msg\r\n` | `RespError` |
| Integer | `:42\r\n` | `int` |
| Bulk String | `$6\r\nfoobar\r\n` | `bytes` |
| Null Bulk String | `$-1\r\n` | `None` |
| Array | `*2\r\n...\r\n` | `list` |
| Null Array | `*-1\r\n` | `None` |
| Inline command | `PING\r\n` | `list[bytes]` |

## Usage

```python
from resp_protocol import encode, decode, RespError, RespDecoder, decode_all

# Encode Python values to RESP bytes
encode(b"hello")              # b"$5\r\nhello\r\n"
encode(42)                    # b":42\r\n"
encode(None)                  # b"$-1\r\n"
encode([b"SET", b"k", b"v"]) # b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"

# Decode RESP bytes — returns (value, bytes_consumed)
decode(b"+OK\r\n")            # ("OK", 5)
decode(b":42\r\n")            # (42, 5)
decode(b"$-1\r\n")            # (None, 5)
decode(b"*2\r\n:1\r\n:2\r\n") # ([1, 2], 12)
decode(b"$3\r\nfoo")          # (None, 0) — incomplete, need more bytes

# Error replies
err_bytes = b"-ERR unknown command\r\n"
err, _ = decode(err_bytes)
err.error_type  # "ERR"
err.detail      # "unknown command"

# Streaming decoder (TCP read loop)
decoder = RespDecoder()
decoder.feed(b"*2\r\n$3\r\nfoo\r\n")  # partial
decoder.has_message()  # False
decoder.feed(b"$3\r\nbar\r\n")        # completes the array
decoder.has_message()  # True
decoder.get_message()  # [b"foo", b"bar"]
```

## Layer position

```
DT23: resp-protocol    ← this package
  ↑
DT24: tcp-server       (uses RESP to frame messages in a TCP stream)
DT25: mini-redis       (commands decoded via RESP, responses encoded via RESP)
```

## Running tests

```bash
uv venv .venv --quiet --no-project
uv pip install --python .venv -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
```
