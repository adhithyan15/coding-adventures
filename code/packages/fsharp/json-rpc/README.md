# CodingAdventures.JsonRpc.FSharp

JSON-RPC 2.0 message model, Content-Length framing, and a small dispatch server.

```fsharp
open System.Text.Json.Nodes
open CodingAdventures.JsonRpc.FSharp

Server(input, output)
    .OnRequest("initialize", fun _ _ -> JsonObject() :> obj)
    .OnNotification("exit", fun _ -> ())
    .Serve()
```

The package implements request, response, notification, error envelopes,
Content-Length readers/writers, and sequential server dispatch.
