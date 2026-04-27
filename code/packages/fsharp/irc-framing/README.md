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
