# @coding-adventures/irc-proto

Pure IRC message parsing and serialization — converts between raw IRC wire lines and structured `Message` objects.

## The Problem

IRC is a text protocol.  Every message on the wire looks like:

```
[:prefix] command [param1 param2 ... :trailing param]\r\n
```

For example:
```
:alice!alice@127.0.0.1 PRIVMSG #general :hello world\r\n
:irc.local 001 alice :Welcome to the IRC Network, alice!alice@host\r\n
NICK alice\r\n
```

This package turns those raw strings into structured objects and back — with zero dependencies.

## Usage

```typescript
import { parse, serialize, Message, ParseError } from '@coding-adventures/irc-proto';

// Parsing
const msg = parse(":alice!a@h PRIVMSG #chan :hello world");
// → { prefix: "alice!a@h", command: "PRIVMSG", params: ["#chan", "hello world"] }

// Serialization
const buf = serialize({ prefix: "irc.local", command: "001", params: ["alice", "Welcome!"] });
// → Buffer(":irc.local 001 alice Welcome!\r\n")

// Error handling
try {
  parse(""); // throws ParseError
} catch (e) {
  if (e instanceof ParseError) console.error("Bad IRC line:", e.message);
}
```

## RFC 1459 Compliance

- Commands are normalised to uppercase
- Trailing parameters (those containing spaces) are serialized with a leading `:`
- Maximum 15 parameters; extras are silently discarded
- Empty or whitespace-only lines throw `ParseError`
- Prefix-only lines (no command) throw `ParseError`

## Stack Position

```
irc-proto   ← THIS PACKAGE: parse() / serialize()
    ↑
irc-framing ← feeds complete lines to parse()
    ↑
irc-net-*   ← raw TCP bytes
```
