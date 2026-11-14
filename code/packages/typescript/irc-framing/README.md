# @coding-adventures/irc-framing

Stateful TCP byte-stream to IRC line framer — assembles `\r\n`-terminated IRC lines from raw socket bytes.

## The Problem

TCP delivers a continuous byte stream.  A single `data` event may return:

- Half a message: `"NICK ali"`
- Exactly one message: `"NICK alice\r\n"`
- Three messages concatenated

This package solves the reassembly problem with zero dependencies and no I/O.

## Usage

```typescript
import { Framer } from '@coding-adventures/irc-framing';

const framer = new Framer();

socket.on('data', (chunk: Buffer) => {
  framer.feed(chunk);
  for (const line of framer.frames()) {
    const msg = parse(line.toString('utf-8'));
    handleMessage(msg);
  }
});

socket.on('close', () => framer.reset());
```

## RFC 1459 Compliance

- Lines longer than 510 bytes (512 including CRLF) are silently discarded
- A pending (not yet LF-terminated) line is capped at 511 bytes: 510 content
  bytes plus a CR (RFC 1459 / RFC 2812 section 2.3).  Past that the partial
  line is dropped and input is skipped up to the next LF, so memory stays
  constant however much a peer sends without an LF (SEC-011).  No API change.
- Both `\r\n` (CRLF) and bare `\n` (LF) are accepted as line terminators
- The terminator is stripped from each returned `Buffer`
