# CodingAdventures.IrcFraming.FSharp

Stateful byte-stream to IRC line frame converter for F#.

```fsharp
open System.Text
open CodingAdventures.IrcFraming.FSharp

let framer = Framer()
framer.Feed(Encoding.ASCII.GetBytes "NICK alice\r\nUSER alice 0 * :Alice\r\n")

for frame in framer.Frames() do
    printfn "%s" (Encoding.ASCII.GetString frame)
```

IRC messages end in CRLF and are capped at 512 bytes including CRLF, leaving
510 bytes of content. Overlong frames are discarded.

The pending, not yet LF-terminated, line is capped at 511 bytes: 510 content
bytes plus a CR (RFC 1459 and RFC 2812 section 2.3). Past that the partial line
is dropped and input is skipped up to the next LF, so a peer that never sends LF
cannot grow the buffer (SEC-011). No API change; `Reset()` also clears the
discard state.
