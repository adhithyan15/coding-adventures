# ircd (TypeScript)

IRC server executable — wires together `irc-proto`, `irc-framing`, `irc-server`, and `irc-net-stdlib`.

## Usage

```bash
# Start with default settings (port 6667, all interfaces):
npx tsx src/index.ts

# Custom settings:
npx tsx src/index.ts --host 127.0.0.1 --port 6668 --server-name irc.example.com --motd "Hello!" --oper-password secret
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `0.0.0.0` | IP address to bind |
| `--port` | `6667` | TCP port |
| `--server-name` | `irc.local` | Server name in welcome messages |
| `--motd` | `Welcome.` | Message of the Day (can repeat) |
| `--oper-password` | `` | Password for OPER command |

## Architecture

```
TCP socket
   ↓ raw bytes
EventLoop (irc-net-stdlib)
   ↓ connId + Buffer
DriverHandler.onData()
   ↓ feeds Framer (irc-framing)
parse() (irc-proto)
   ↓ Message
IRCServer.onMessage() (irc-server)
   ↓ [ConnId, Message][]
serialize() (irc-proto)
   ↓ Buffer
EventLoop.sendTo()
```

Graceful shutdown: SIGINT or SIGTERM closes all connections and exits cleanly.
