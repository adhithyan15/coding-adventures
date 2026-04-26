# CodingAdventures.JsonRpc.CSharp

JSON-RPC 2.0 message model, Content-Length framing, and a small dispatch server.

```csharp
using CodingAdventures.JsonRpc;
using System.Text.Json.Nodes;

var server = new Server(input, output)
    .OnRequest("initialize", (_, _) => new JsonObject { ["capabilities"] = new JsonObject() })
    .OnNotification("exit", _ => { });

server.Serve();
```

The package implements request, response, notification, error envelopes,
Content-Length readers/writers, and sequential server dispatch.
