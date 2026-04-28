# CodingAdventures.IrcFraming.CSharp

Stateful byte-stream to IRC line frame converter for C#.

```csharp
using CodingAdventures.IrcFraming;
using System.Text;

var framer = new Framer();
framer.Feed(Encoding.ASCII.GetBytes("NICK alice\r\nUSER alice 0 * :Alice\r\n"));

foreach (var frame in framer.Frames())
{
    Console.WriteLine(Encoding.ASCII.GetString(frame));
}
```

IRC messages end in CRLF and are capped at 512 bytes including CRLF, leaving
510 bytes of content. Overlong frames are discarded.
