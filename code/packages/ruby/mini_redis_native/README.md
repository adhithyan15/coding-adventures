# coding_adventures_mini_redis_native

Ruby Mini Redis server backed by the Rust embeddable TCP runtime.

Rust owns the TCP listener, native event loop, connection lifecycle, and socket
writes. Ruby owns the Mini Redis application protocol: per-stream RESP buffers,
command execution, and RESP responses. The two sides communicate with the same
JSON-line `generic-job-protocol` shape used by the Python Mini Redis worker.

## Usage

```ruby
require "coding_adventures_mini_redis_native"

server = CodingAdventures::MiniRedisNative::Server.new(port: 6380)
server.serve
```

For tests or embedding, run the server on a Ruby thread:

```ruby
server = CodingAdventures::MiniRedisNative::Server.new(port: 0)
thread = server.start

puts "listening on #{server.host}:#{server.port}"

server.stop
thread.join
server.close
```

Supported Mini Redis commands:

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

## Development

```bash
bash BUILD
```

On Windows RubyInstaller builds, the Rake/extconf tasks select the Rust GNU
target that matches Ruby's ABI and run Cargo through `ridk exec` when available.
