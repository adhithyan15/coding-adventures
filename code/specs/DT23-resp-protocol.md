# DT23 — RESP Protocol

## Overview

RESP (REdis Serialization Protocol) is the wire format Redis uses for every
single byte that travels between a client and server. It was designed with
three goals in mind:

1. **Simple to implement** — Any programmer can write a parser in an afternoon.
2. **Fast to parse** — A server processing 100,000 commands per second cannot
   afford a slow parser.
3. **Human-readable** — You can `telnet` into a Redis server and type commands
   by hand. The protocol is visible to the naked eye.

RESP is not a general-purpose protocol. It is not JSON, not Protobuf, not
MessagePack. It is purpose-built for the request/reply pattern of a database
client talking to a database server. This simplicity is a feature.

```
Without RESP — how would you send "SET foo bar" over raw TCP?

  Option 1: Newline-delimited text
    SET foo bar\n
    Problem: values can contain newlines. Binary data breaks everything.

  Option 2: Length-prefixed binary
    \x00\x00\x00\x0bSET foo bar
    Problem: hard to read, harder to debug, still unclear which bytes
    belong to which "argument".

  Option 3: RESP (what Redis actually uses)
    *3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
    Pros: binary-safe (lengths tell you exactly how many bytes to read),
    self-delimiting (parsers can determine message boundaries), readable
    (type bytes and numbers visible in plain text).
```

## Layer Position

```
DT17: hash-functions   ← (not used by RESP itself)
DT18: hash-map         ← (not used by RESP itself)
DT19: hash-set         ← (not used by RESP itself)
DT20: skip-list        ← (not used by RESP itself)
DT21: hyperloglog      ← (not used by RESP itself)
DT22: bloom-filter     ← (not used by RESP itself)

DT23: resp-protocol    ← [YOU ARE HERE]
                          Binary serialization layer.
                          Knows nothing about commands or storage.
                          Pure encode/decode functions over bytes.

DT24: tcp-server       ← Uses RESP to frame messages within a TCP stream.
DT25: mini-redis       ← Commands are decoded via RESP, responses encoded via RESP.
```

**Depends on:** Nothing. RESP is a pure data transformation — bytes in, typed
values out. No data structures required.

**Used by:** Every Redis client library in existence (redis-py, Jedis, ioredis,
go-redis, etc.), DT24 (to frame TCP stream into messages), DT25 (to speak the
Redis wire protocol so real clients can connect).

## Concepts

### The Seven RESP2 Types

Every RESP value begins with a single type byte, followed by data, followed
by `\r\n` (carriage return + newline, ASCII 13 + 10). This terminator is
mandatory on every line — it is part of the protocol, not a formatting choice.

```
Type           Prefix   Wire format
─────────────────────────────────────────────────────────────────────
Simple String    +      +<text>\r\n
Error            -      -<error message>\r\n
Integer          :      :<number>\r\n
Bulk String      $      $<length>\r\n<bytes>\r\n
Null Bulk String $      $-1\r\n
Array            *      *<count>\r\n<element> <element> ...
Null Array       *      *-1\r\n
─────────────────────────────────────────────────────────────────────
```

Walk through each type with concrete examples:

**Simple String** — used for short, non-binary status replies:
```
+OK\r\n                     → the string "OK"
+PONG\r\n                   → the string "PONG"
+QUEUED\r\n                 → the string "QUEUED" (MULTI/EXEC)
```
Restriction: the text cannot contain `\r` or `\n`. Use Bulk String for
arbitrary data.

**Error** — used for error replies. Conventionally, the first word is an
error class (ERR, WRONGTYPE, NOSCRIPT, etc.):
```
-ERR unknown command 'foo'\r\n
-WRONGTYPE Operation against a key holding the wrong kind of value\r\n
-NOSCRIPT No matching script\r\n
```
Clients should surface the error class and message separately.

**Integer** — used for numeric replies (INCR result, LLEN count, etc.):
```
:0\r\n       → 0
:42\r\n      → 42
:-1\r\n      → -1 (used as a sentinel by some commands, e.g., LPOS not found)
:1000000\r\n → 1000000
```

**Bulk String** — binary-safe string. The length comes first, then the bytes:
```
$6\r\nfoobar\r\n      → "foobar" (6 bytes)
$0\r\n\r\n            → "" (empty string, 0 bytes)
$-1\r\n               → null (key does not exist, e.g., GET of missing key)
```
The length field tells the parser exactly how many bytes to read. After those
bytes, there is always a mandatory `\r\n` terminator. This means a bulk string
can contain any bytes — nulls, newlines, binary data — without ambiguity.

```
Binary data example:
  A PNG header contains \x89PNG\r\n which includes \r\n.
  With Simple String, this would break the parser.
  With Bulk String:
    $8\r\n\x89PNG\r\n\x1a\n\r\n
    The parser reads exactly 8 bytes after the first \r\n, then
    expects the closing \r\n. The \r\n inside the data is ignored.
```

**Array** — an ordered list of any RESP values (including nested arrays):
```
*0\r\n                          → empty array []
*2\r\n:1\r\n:2\r\n              → [1, 2]
*-1\r\n                         → null array (some commands use this)
```

### Encoding "SET foo bar"

Every Redis command from a client is sent as an Array of Bulk Strings.
The command name is the first element. Arguments follow.

```
Command: SET foo bar
Array of 3 Bulk Strings:

*3\r\n          ← array with 3 elements
$3\r\n          ← first element: bulk string, 3 bytes
SET\r\n         ← the bytes "SET"
$3\r\n          ← second element: bulk string, 3 bytes
foo\r\n         ← the bytes "foo"
$3\r\n          ← third element: bulk string, 3 bytes
bar\r\n         ← the bytes "bar"

Total: 28 bytes to send "SET foo bar"
```

Another example: `ZADD leaderboard 100 alice`:
```
*4\r\n
$4\r\n
ZADD\r\n
$11\r\n
leaderboard\r\n
$3\r\n
100\r\n
$5\r\n
alice\r\n
```

And the server's reply to SET (a Simple String):
```
+OK\r\n
```

### The Framing Problem

TCP is a **byte stream**. When your server calls `read()` on a socket, it gets
some bytes — but there is absolutely no guarantee about how many. The OS may
give you:

```
Scenario 1: Fragment — less than one message
  Buffer after read():  "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nb"
  What we have: the start of "SET foo bar" — missing the last "ar\r\n"
  What to do:  store in a read buffer, call read() again, append, retry parse

Scenario 2: Exact — exactly one message
  Buffer after read():  "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
  What we have: one complete command
  What to do:  parse it, dispatch it, clear buffer

Scenario 3: Boundary — one and a half messages
  Buffer after read():  "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n*1\r\n$4\r\nPING"
  What we have: "SET foo bar" complete + start of "PING"
  What to do:  parse first message, leave remainder in buffer

Scenario 4: Burst — many messages
  Buffer after read():  (10 complete commands concatenated)
  What to do:  parse in a loop until buffer is empty or incomplete
```

The read buffer is the solution. It accumulates bytes from `read()` calls.
The parser is called on the buffer after each `read()`. If a complete message
is found, it is consumed and dispatched. Leftover bytes stay in the buffer.

```
Read buffer lifecycle:

   ┌──────────────────────────────────────────────────────────┐
   │                      Read Buffer                         │
   │                                                          │
   │  [ already parsed | current parse attempt | future data ]│
   │  └─ consumed ──────┘└─ parse cursor here  ┘└─ not yet ─ ┘│
   └──────────────────────────────────────────────────────────┘

After successful parse:
   consume(bytes_consumed)  → shift the buffer left

After incomplete parse (need more data):
   wait for next read() call, append to buffer, retry
```

Worked example: parsing `*2\r\n$3\r\nfoo\r\n$3\r\nb` (incomplete):

```
Step 1: Read type byte → '*'     (array)
Step 2: Read until \r\n → "2"   (array has 2 elements)
Step 3: Parse element 1:
  Read type byte → '$'           (bulk string)
  Read until \r\n → "3"          (length = 3)
  Need 3 bytes + \r\n = 5 bytes. Buffer has "foo\r\n" → success
  Element 1 = "foo"
Step 4: Parse element 2:
  Read type byte → '$'
  Read until \r\n → "3"          (length = 3)
  Need 3 + 2 = 5 bytes. Buffer has "b" → ONLY 1 BYTE! INCOMPLETE.
  → Return None (not enough data yet), set bytes_consumed = 0
  → Wait for more data to arrive via read()
```

### The Parser State Machine

A recursive-descent parser with these transitions:

```
                      ┌─────────────────────────────────────────┐
         start        │                                         │
           │          │                                         ▼
           ▼          │  '+' ──→ READ_LINE ──→ emit SimpleString
    READ_TYPE_BYTE    │  '-' ──→ READ_LINE ──→ emit Error
           │          │  ':' ──→ READ_LINE ──→ parse int → emit Integer
      ─────┼─────     │  '$' ──→ READ_LINE → parse length:
           │          │            if length == -1 → emit NullBulkString
           │          │            else → READ_BYTES(length) → READ_CRLF
           │          │                 → emit BulkString
           │          │  '*' ──→ READ_LINE → parse count:
           │          │            if count == -1 → emit NullArray
           │          │            else → recurse count times
           │          │                 → emit Array
           │          │
           └──────────┘ (loop back for next message)

READ_LINE: read bytes until \r\n, return everything before \r\n
READ_BYTES(n): read exactly n bytes
READ_CRLF: read and discard exactly \r\n
```

The parser is **zero-copy** when possible: it takes a slice/view of the
input buffer rather than allocating new strings for each component.

### RESP3: The Upgrade

Redis 6.0 introduced RESP3. Clients negotiate it by sending `HELLO 3` as
their first command. RESP3 adds richer types:

```
New type     Prefix   Purpose
─────────────────────────────────────────────────────────────────────
Map          %        Like Array but alternating key/value pairs
Set          ~        Unordered collection (like a mathematical set)
Double       ,        IEEE 754 floating point
Big Number   (        Integers too large for 64-bit signed
Boolean      #        #t\r\n or #f\r\n
Blob Error   !        Like Error but binary-safe (like Bulk String)
Verbatim     =        String with a 3-char encoding prefix (txt, mkd)
Push         >        Out-of-band push messages (pub/sub, keyspace)
─────────────────────────────────────────────────────────────────────
```

RESP3 also enables "attribute" data (metadata attached to any value) and
changes some server replies from Arrays to Maps for better type safety
(e.g., HGETALL returns a Map, not a flat Array of alternating keys/values).

**DT23 implements RESP2.** RESP3 is a future extension. All major RESP2
clients connect to a RESP3 server transparently — the server detects the
`HELLO` command and falls back to RESP2 if it's not sent.

## Representation

```
RespValue (algebraic data type / tagged union):
  SimpleString(value: str)
  Error(message: str)
  Integer(value: int)
  BulkString(value: bytes | None)   # None = null bulk string
  Array(elements: list[RespValue] | None)  # None = null array

Encoder output: bytes
Decoder input:  bytes (the accumulated read buffer)
Decoder output: (RespValue, int)  # value + bytes consumed
                 or (None, 0)     # incomplete — need more data
```

The decoder returns a `(value, consumed)` pair rather than mutating a
cursor. This pure-function style makes it easy to test and compose:

```
buffer = b"*2\r\n:1\r\n:2\r\n+OK\r\n"
val1, n1 = decode(buffer)       # val1=[1,2], n1=14
val2, n2 = decode(buffer[n1:])  # val2="OK", n2=5
```

## Algorithms (Pure Functions)

### encode_simple_string(s: str) → bytes

```
encode_simple_string(s):
    # Precondition: s contains no \r or \n
    return b"+" + s.encode("utf-8") + b"\r\n"

Examples:
  encode_simple_string("OK")   → b"+OK\r\n"
  encode_simple_string("PONG") → b"+PONG\r\n"
```

### encode_error(msg: str) → bytes

```
encode_error(msg):
    return b"-" + msg.encode("utf-8") + b"\r\n"

Examples:
  encode_error("ERR unknown command")
    → b"-ERR unknown command\r\n"
  encode_error("WRONGTYPE value is not a list")
    → b"-WRONGTYPE value is not a list\r\n"
```

### encode_integer(n: int) → bytes

```
encode_integer(n):
    return b":" + str(n).encode("ascii") + b"\r\n"

Examples:
  encode_integer(0)   → b":0\r\n"
  encode_integer(42)  → b":42\r\n"
  encode_integer(-1)  → b":-1\r\n"
```

### encode_bulk_string(s: bytes | None) → bytes

```
encode_bulk_string(s):
    if s is None:
        return b"$-1\r\n"
    length = len(s)
    return b"$" + str(length).encode("ascii") + b"\r\n" + s + b"\r\n"

Examples:
  encode_bulk_string(b"foobar")  → b"$6\r\nfoobar\r\n"
  encode_bulk_string(b"")        → b"$0\r\n\r\n"
  encode_bulk_string(None)       → b"$-1\r\n"
  encode_bulk_string(b"foo\r\nbar")  → b"$7\r\nfoo\r\nbar\r\n"
    (binary-safe: \r\n inside data is fine because we use length framing)
```

### encode_array(items: list | None) → bytes

```
encode_array(items):
    if items is None:
        return b"*-1\r\n"
    if len(items) == 0:
        return b"*0\r\n"
    header = b"*" + str(len(items)).encode("ascii") + b"\r\n"
    body = b"".join(encode(item) for item in items)
    return header + body
```

### encode(value: Any) → bytes

Dispatches on type to produce the canonical RESP encoding:

```
encode(value):
    if value is None:
        return encode_bulk_string(None)
    if isinstance(value, bool):
        return encode_integer(1 if value else 0)
    if isinstance(value, int):
        return encode_integer(value)
    if isinstance(value, str):
        return encode_bulk_string(value.encode("utf-8"))
    if isinstance(value, bytes):
        return encode_bulk_string(value)
    if isinstance(value, list):
        return encode_array(value)
    if isinstance(value, RespError):
        return encode_error(value.message)
    raise TypeError(f"Cannot encode {type(value)}")
```

### decode(buffer: bytes) → (RespValue | None, int)

```
decode(buffer):
    if len(buffer) == 0:
        return None, 0
    type_byte = buffer[0:1]
    rest = buffer[1:]

    if type_byte == b"+":
        line, n = read_line(rest)
        if line is None: return None, 0
        return SimpleString(line.decode("utf-8")), 1 + n

    if type_byte == b"-":
        line, n = read_line(rest)
        if line is None: return None, 0
        return RespError(line.decode("utf-8")), 1 + n

    if type_byte == b":":
        line, n = read_line(rest)
        if line is None: return None, 0
        return int(line), 1 + n

    if type_byte == b"$":
        line, n = read_line(rest)
        if line is None: return None, 0
        length = int(line)
        if length == -1:
            return None, 1 + n               # null bulk string
        if len(rest) < n + length + 2:
            return None, 0                   # need more data
        data = rest[n : n + length]
        return data, 1 + n + length + 2      # +2 for trailing \r\n

    if type_byte == b"*":
        line, n = read_line(rest)
        if line is None: return None, 0
        count = int(line)
        if count == -1:
            return None, 1 + n               # null array
        offset = 1 + n
        elements = []
        for _ in range(count):
            elem, consumed = decode(buffer[offset:])
            if elem is None and consumed == 0:
                return None, 0               # incomplete element
            elements.append(elem)
            offset += consumed
        return elements, offset

    raise ValueError(f"Unknown RESP type byte: {type_byte!r}")


read_line(buffer) → (bytes | None, int):
    # Read bytes up to and including the first \r\n.
    # Returns (content_before_crlf, total_bytes_consumed_including_crlf)
    # Returns (None, 0) if \r\n not yet in buffer.
    pos = buffer.find(b"\r\n")
    if pos == -1:
        return None, 0
    return buffer[:pos], pos + 2
```

### decode_all(buffer: bytes) → (list[RespValue], int)

Decode as many complete messages as possible from the buffer:

```
decode_all(buffer):
    messages = []
    offset = 0
    while offset < len(buffer):
        value, consumed = decode(buffer[offset:])
        if consumed == 0:
            break           # incomplete message — wait for more bytes
        messages.append(value)
        offset += consumed
    return messages, offset
```

## Public API

```python
from __future__ import annotations
from typing import Any


class RespError:
    """An error value returned by the server."""
    def __init__(self, message: str) -> None:
        self.message = message
        # Convention: first word is the error type
        parts = message.split(" ", 1)
        self.error_type = parts[0]        # e.g. "ERR", "WRONGTYPE"
        self.detail = parts[1] if len(parts) > 1 else ""

    def __repr__(self) -> str:
        return f"RespError({self.message!r})"


# RespValue is the union of all possible decoded types:
# str (Simple String), RespError, int, bytes (Bulk String),
# None (Null Bulk String or Null Array), list (Array)
RespValue = str | RespError | int | bytes | None | list


def encode_simple_string(s: str) -> bytes:
    """Encode a Simple String. Must not contain \\r or \\n."""

def encode_error(msg: str) -> bytes:
    """Encode an Error reply."""

def encode_integer(n: int) -> bytes:
    """Encode an Integer."""

def encode_bulk_string(s: bytes | None) -> bytes:
    """Encode a Bulk String. None encodes as $-1\\r\\n (null)."""

def encode_array(items: list | None) -> bytes:
    """Encode an Array. None encodes as *-1\\r\\n (null array)."""

def encode(value: Any) -> bytes:
    """
    Dispatch encoder: convert a Python value to its RESP encoding.

    Python → RESP:
      None         → $-1\\r\\n  (null bulk string)
      int          → :<n>\\r\\n
      str          → $<len>\\r\\n<utf-8 bytes>\\r\\n
      bytes        → $<len>\\r\\n<bytes>\\r\\n
      list         → *<n>\\r\\n<element>...
      RespError    → -<msg>\\r\\n
    """

def decode(buffer: bytes) -> tuple[RespValue, int]:
    """
    Attempt to decode one RESP value from the start of buffer.

    Returns (value, bytes_consumed).
    Returns (None, 0) if buffer does not contain a complete message.
    Raises ValueError on malformed input.
    """

def decode_all(buffer: bytes) -> tuple[list[RespValue], int]:
    """
    Decode as many complete RESP messages as possible from buffer.

    Returns (messages, total_bytes_consumed).
    Unconsumed bytes (an incomplete final message) remain in buffer.
    Caller should keep buffer[total_bytes_consumed:] for the next read.
    """
```

## Composition Model

RESP is a pure transformation layer — inputs and outputs are bytes. It has
no persistent state. All functions are pure (no side effects).

### Python / Ruby / TypeScript — Module of Pure Functions

```python
# Python: no classes needed, just functions in a module
# resp.py

def encode(value):
    if value is None:
        return b"$-1\r\n"
    if isinstance(value, int):
        return b":" + str(value).encode() + b"\r\n"
    if isinstance(value, (str, bytes)):
        b = value.encode() if isinstance(value, str) else value
        return b"$" + str(len(b)).encode() + b"\r\n" + b + b"\r\n"
    if isinstance(value, list):
        return b"*" + str(len(value)).encode() + b"\r\n" + \
               b"".join(encode(v) for v in value)
    raise TypeError(value)
```

### Rust — Enum + Pattern Match

```rust
// Rust: ADT makes the type system enforce exhaustive handling
#[derive(Debug, PartialEq, Clone)]
pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),    // None = null
    Array(Option<Vec<RespValue>>),  // None = null array
}

impl RespValue {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            RespValue::SimpleString(s) =>
                format!("+{}\r\n", s).into_bytes(),
            RespValue::Error(e) =>
                format!("-{}\r\n", e).into_bytes(),
            RespValue::Integer(n) =>
                format!(":{}\r\n", n).into_bytes(),
            RespValue::BulkString(None) =>
                b"$-1\r\n".to_vec(),
            RespValue::BulkString(Some(b)) => {
                let mut out = format!("${}\r\n", b.len()).into_bytes();
                out.extend_from_slice(b);
                out.extend_from_slice(b"\r\n");
                out
            }
            RespValue::Array(None) =>
                b"*-1\r\n".to_vec(),
            RespValue::Array(Some(items)) => {
                let mut out = format!("*{}\r\n", items.len()).into_bytes();
                for item in items {
                    out.extend(item.encode());
                }
                out
            }
        }
    }
}
```

### Go — Interface + Switch

```go
// Go: sum type via interface
type RespValue interface{ respValue() }

type SimpleString struct{ Value string }
type RespError    struct{ Message string }
type Integer      struct{ Value int64 }
type BulkString   struct{ Value []byte } // nil Value = null
type Array        struct{ Items []RespValue } // nil Items = null array

func Encode(v RespValue) []byte {
    switch val := v.(type) {
    case SimpleString:
        return []byte("+" + val.Value + "\r\n")
    case RespError:
        return []byte("-" + val.Message + "\r\n")
    case Integer:
        return []byte(fmt.Sprintf(":%d\r\n", val.Value))
    case BulkString:
        if val.Value == nil { return []byte("$-1\r\n") }
        return []byte(fmt.Sprintf("$%d\r\n", len(val.Value)) +
               string(val.Value) + "\r\n")
    case Array:
        if val.Items == nil { return []byte("*-1\r\n") }
        b := []byte(fmt.Sprintf("*%d\r\n", len(val.Items)))
        for _, item := range val.Items { b = append(b, Encode(item)...) }
        return b
    }
    panic("unknown type")
}
```

### Elixir — Recursive Pattern Matching

```elixir
defmodule Resp do
  # Encode any Elixir term as RESP bytes.
  # nil         → null bulk string
  # integer     → RESP Integer
  # string      → RESP Bulk String
  # {:error, m} → RESP Error
  # list        → RESP Array

  def encode(nil),             do: "$-1\r\n"
  def encode(n) when is_integer(n), do: ":#{n}\r\n"
  def encode({:error, msg}),   do: "-#{msg}\r\n"
  def encode(s) when is_binary(s) do
    "$#{byte_size(s)}\r\n#{s}\r\n"
  end
  def encode(list) when is_list(list) do
    items = Enum.map_join(list, "", &encode/1)
    "*#{length(list)}\r\n#{items}"
  end
end
```

## Test Strategy

### Encode-Decode Round Trip

```python
def test_round_trip():
    """encode(decode(encode(x))) == encode(x) for all types."""
    cases = [
        "OK",
        "",
        "hello world",
        b"",
        b"binary\x00data",
        b"has\r\nnewlines",
        0,
        42,
        -1,
        None,
        ["SET", "foo", "bar"],
        [1, None, "ok", b"bytes"],
        [],
    ]
    for value in cases:
        encoded = encode(value)
        decoded, consumed = decode(encoded)
        assert consumed == len(encoded), f"Consumed {consumed} of {len(encoded)}"
        re_encoded = encode(decoded)
        assert re_encoded == encoded, f"Round trip failed for {value!r}"
```

### Incomplete Buffers

```python
def test_incomplete_returns_none():
    """Partial messages must return (None, 0), never raise."""
    full = b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
    # Every prefix shorter than the full message must be incomplete
    for i in range(1, len(full)):
        partial = full[:i]
        value, consumed = decode(partial)
        assert value is None and consumed == 0, \
            f"Expected incomplete at {i} bytes, got value={value!r}"
    # The full message must parse
    value, consumed = decode(full)
    assert value == [b"SET", b"foo", b"bar"]
    assert consumed == len(full)
```

### Null Handling

```python
def test_null_bulk_string():
    encoded = encode_bulk_string(None)
    assert encoded == b"$-1\r\n"
    value, consumed = decode(encoded)
    assert value is None
    assert consumed == 5

def test_null_array():
    encoded = encode_array(None)
    assert encoded == b"*-1\r\n"
    value, consumed = decode(encoded)
    assert value is None
    assert consumed == 5
```

### Binary Safety

```python
def test_binary_safe():
    """Bulk strings must handle arbitrary bytes including \r\n."""
    payload = bytes(range(256))   # all 256 byte values
    encoded = encode_bulk_string(payload)
    value, consumed = decode(encoded)
    assert value == payload

def test_bulk_string_with_crlf():
    data = b"line1\r\nline2\r\nline3"
    encoded = encode_bulk_string(data)
    value, _ = decode(encoded)
    assert value == data
```

### decode_all Streaming Simulation

```python
def test_streaming():
    """Simulate TCP fragmentation: feed bytes one at a time."""
    messages = [
        encode(["SET", "k", "v"]),
        encode(["GET", "k"]),
        encode(["DEL", "k"]),
    ]
    full_stream = b"".join(messages)

    # Feed the stream byte by byte
    buffer = b""
    received = []
    for byte in full_stream:
        buffer += bytes([byte])
        parsed, consumed = decode_all(buffer)
        received.extend(parsed)
        buffer = buffer[consumed:]

    assert len(received) == 3
    assert received[0] == [b"SET", b"k", b"v"]
    assert received[1] == [b"GET", b"k"]
    assert received[2] == [b"DEL", b"k"]
```

### Error Type Parsing

```python
def test_error_parsing():
    raw = b"-WRONGTYPE Operation against wrong type\r\n"
    value, consumed = decode(raw)
    assert isinstance(value, RespError)
    assert value.error_type == "WRONGTYPE"
    assert consumed == len(raw)
```

## Future Extensions

**RESP3 Types:** Add Map (`%`), Set (`~`), Double (`,`), BigNumber (`(`),
Boolean (`#`), BlobError (`!`), VerbatimString (`=`), Push (`>`). RESP3
requires the client to send `HELLO 3` to the server first. All new types
follow the same `<prefix><data>\r\n` pattern.

**Inline Commands:** Redis also accepts a simplified inline format for
human use via telnet. A line like `PING\r\n` (no RESP framing) is valid.
Inline parsing is simple: split on spaces, treat each token as a bulk string.

**Pipelining Support:** Clients can send many commands without waiting for
replies. `decode_all` already handles this — just collect all replies and
match them to the sent commands in order.

**RESP2 → RESP3 Negotiation:** The server sends a Map reply to `HELLO 3`
describing its capabilities. Add a `negotiate_version(hello_command)` that
returns the appropriate reply and switches the encoder/decoder mode.

**Zero-Copy Parser:** The current decoder allocates Python bytes objects.
A production implementation would use a `memoryview` or buffer protocol to
avoid allocation. In Rust, use `&[u8]` slices with lifetime tracking.

**Streaming Encoder:** For very large bulk strings (e.g., large blobs), an
iterator-based encoder avoids materialising the full encoded form in memory.
Yields chunks: header bytes first, then data bytes, then `\r\n`.
