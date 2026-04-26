namespace CodingAdventures.JsonRpc.Tests

open System
open System.IO
open System.Linq
open System.Text
open System.Text.Json.Nodes
open Xunit
open CodingAdventures.JsonRpc.FSharp

module JsonRpcTests =
    let private frame (json: string) =
        let payload = Encoding.UTF8.GetBytes json
        Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n\r\n").Concat(payload).ToArray()

    let private frameWithHeader (json: string) (extra: string) =
        let payload = Encoding.UTF8.GetBytes json
        Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n{extra}\r\n\r\n").Concat(payload).ToArray()

    let private assertRpcError code action =
        let ex = Assert.Throws<JsonRpcException>(fun () -> action() |> ignore)
        Assert.Equal(code, ex.Code)

    [<Fact>]
    let ``parses requests notifications and responses`` () =
        match JsonRpc.parseMessage """{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"x":1}}""" with
        | RequestMessage request ->
            Assert.Equal(1, request.Id.GetValue<int>())
            Assert.Equal("initialize", request.Method)
            Assert.Equal(1, request.Params.Value["x"].GetValue<int>())
        | _ -> failwith "expected request"

        match JsonRpc.parseMessage """{"jsonrpc":"2.0","method":"exit"}""" with
        | NotificationMessage notification ->
            Assert.Equal("exit", notification.Method)
            Assert.Equal(None, notification.Params)
        | _ -> failwith "expected notification"

        match JsonRpc.parseMessage """{"jsonrpc":"2.0","id":"abc","result":{"ok":true}}""" with
        | ResponseMessage response ->
            Assert.Equal("abc", response.Id.Value.GetValue<string>())
            Assert.True(response.Result.Value["ok"].GetValue<bool>())
        | _ -> failwith "expected response"

    [<Fact>]
    let ``parses error responses and reports invalid messages`` () =
        match JsonRpc.parseMessage """{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found","data":"hover"}}""" with
        | ResponseMessage response ->
            Assert.Equal(JsonRpc.METHOD_NOT_FOUND, response.Error.Value.Code)
            Assert.Equal("hover", response.Error.Value.Data.Value.GetValue<string>())
        | _ -> failwith "expected error response"

        assertRpcError JsonRpc.PARSE_ERROR (fun () -> JsonRpc.parseMessage "{not json}")
        assertRpcError JsonRpc.INVALID_REQUEST (fun () -> JsonRpc.parseMessage """[{"jsonrpc":"2.0"}]""")
        assertRpcError JsonRpc.INVALID_REQUEST (fun () -> JsonRpc.parseMessage """{"jsonrpc":"1.0","method":"x"}""")
        assertRpcError JsonRpc.INVALID_REQUEST (fun () -> JsonRpc.parseMessage """{"jsonrpc":"2.0","id":null,"method":"x"}""")
        assertRpcError JsonRpc.INVALID_REQUEST (fun () -> JsonRpc.parseMessage """{"jsonrpc":"2.0","id":1,"error":"bad"}""")
        assertRpcError JsonRpc.INVALID_REQUEST (fun () -> JsonRpc.parseMessage """{"jsonrpc":"2.0","id":1,"error":{"code":"oops","message":"bad"}}""")

    [<Fact>]
    let ``converts messages to wire objects`` () =
        let parameters = JsonObject()
        parameters["a"] <- JsonValue.Create(1)
        let request = JsonRpc.messageToNode (RequestMessage { Id = JsonValue.Create(1); Method = "foo"; Params = Some parameters })
        Assert.Equal("2.0", request["jsonrpc"].GetValue<string>())
        Assert.Equal("foo", request["method"].GetValue<string>())
        Assert.Equal(1, request.Item("params").Item("a").GetValue<int>())

        let notification = JsonRpc.messageToNode (NotificationMessage { Method = "exit"; Params = None })
        Assert.False(notification.ContainsKey("id"))
        Assert.False(notification.ContainsKey("params"))

        let error =
            JsonRpc.messageToNode (
                ResponseMessage
                    { Id = Some(JsonValue.Create("id"))
                      Result = None
                      Error = Some { Code = JsonRpc.INVALID_PARAMS; Message = "Bad"; Data = Some(JsonValue.Create("detail")) } }
            )
        Assert.Equal(JsonRpc.INVALID_PARAMS, error.Item("error").Item("code").GetValue<int>())
        Assert.Equal("detail", error.Item("error").Item("data").GetValue<string>())

    [<Fact>]
    let ``reader handles framing and eof`` () =
        let raw1 = """{"jsonrpc":"2.0","id":1,"method":"foo"}"""
        let raw2 = """{"jsonrpc":"2.0","method":"bar"}"""
        use stream = new MemoryStream(Array.append (frame raw1) (frame raw2))
        let reader = MessageReader stream

        match reader.ReadMessage() with
        | Some(RequestMessage _) -> ()
        | _ -> failwith "expected request"

        match reader.ReadMessage() with
        | Some(NotificationMessage _) -> ()
        | _ -> failwith "expected notification"

        Assert.Equal(None, reader.ReadMessage())

        use rawStream = new MemoryStream(frameWithHeader raw1 "Content-Type: application/vscode-jsonrpc; charset=utf-8")
        Assert.Equal(Some raw1, (MessageReader rawStream).ReadRaw())

    [<Fact>]
    let ``reader reports malformed frames`` () =
        assertRpcError JsonRpc.PARSE_ERROR (fun () -> (MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Type: text/plain\r\n\r\nhello")))).ReadMessage())
        assertRpcError JsonRpc.PARSE_ERROR (fun () -> (MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: nope\r\n\r\n{}")))).ReadMessage())
        assertRpcError JsonRpc.PARSE_ERROR (fun () -> (MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: -1\r\n\r\n")))).ReadMessage())
        assertRpcError JsonRpc.PARSE_ERROR (fun () -> (MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: 10\r\n\r\nshort")))).ReadMessage())
        assertRpcError JsonRpc.PARSE_ERROR (fun () -> (MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: 1\r\n\r\n\xFF")))).ReadMessage())

    [<Fact>]
    let ``writer produces content length frames`` () =
        use stream = new MemoryStream()
        let writer = MessageWriter stream
        let parameters = JsonObject()
        parameters["text"] <- JsonValue.Create("日本語")
        writer.WriteMessage(RequestMessage { Id = JsonValue.Create(1); Method = "ping"; Params = Some parameters })

        let data = stream.ToArray()
        let separator = Encoding.UTF8.GetString(data).IndexOf("\r\n\r\n", StringComparison.Ordinal)
        Assert.True(separator > 0)
        let header = Encoding.ASCII.GetString(data[0 .. separator - 1])
        let payload = data[separator + 4 ..]
        let parts = header.Split(':')
        let declared = Int32.Parse(parts[1].Trim())

        Assert.Equal(declared, payload.Length)
        Assert.Contains("日本語", Encoding.UTF8.GetString(payload))

    [<Fact>]
    let ``round trips messages through writer and reader`` () =
        use stream = new MemoryStream()
        let parameters = JsonObject()
        parameters["ok"] <- JsonValue.Create(true)
        MessageWriter(stream).WriteMessage(NotificationMessage { Method = "initialized"; Params = Some parameters })
        stream.Position <- 0L

        match (MessageReader stream).ReadMessage() with
        | Some(NotificationMessage recovered) ->
            Assert.Equal("initialized", recovered.Method)
            Assert.True(recovered.Params.Value["ok"].GetValue<bool>())
        | _ -> failwith "expected notification"

    [<Fact>]
    let ``server dispatches requests and notifications`` () =
        let input =
            Array.append
                (frame """{"jsonrpc":"2.0","id":1,"method":"add","params":{"a":1,"b":2}}""")
                (frame """{"jsonrpc":"2.0","method":"ping","params":{"seen":true}}""")

        use inStream = new MemoryStream(input)
        use outStream = new MemoryStream()
        let mutable notified = false

        Server(inStream, outStream)
            .OnRequest("add", fun _ parameters -> JsonValue.Create(parameters.Value["a"].GetValue<int>() + parameters.Value["b"].GetValue<int>()))
            .OnNotification("ping", fun parameters -> notified <- parameters.Value["seen"].GetValue<bool>())
            .Serve()

        Assert.True(notified)
        outStream.Position <- 0L
        match (MessageReader outStream).ReadMessage() with
        | Some(ResponseMessage response) -> Assert.Equal(3, response.Result.Value.GetValue<int>())
        | _ -> failwith "expected response"

    [<Fact>]
    let ``server handles errors`` () =
        let input =
            [| frame """{"jsonrpc":"2.0","id":1,"method":"unknown"}"""
               frame """{"jsonrpc":"2.0","id":2,"method":"bad"}"""
               frame """{"jsonrpc":"2.0","id":3,"method":"boom"}"""
               frame """{"jsonrpc":"2.0","method":"missing"}""" |]
            |> Array.concat

        use inStream = new MemoryStream(input)
        use outStream = new MemoryStream()

        Server(inStream, outStream)
            .OnRequest("bad", fun _ _ -> box { Code = JsonRpc.INVALID_PARAMS; Message = "Bad params"; Data = None })
            .OnRequest("boom", fun _ _ -> raise (InvalidOperationException("boom")))
            .Serve()

        outStream.Position <- 0L
        let reader = MessageReader outStream
        let readCode () =
            match reader.ReadMessage() with
            | Some(ResponseMessage response) -> response.Error.Value.Code
            | _ -> failwith "expected error response"

        Assert.Equal(JsonRpc.METHOD_NOT_FOUND, readCode())
        Assert.Equal(JsonRpc.INVALID_PARAMS, readCode())
        Assert.Equal(JsonRpc.INTERNAL_ERROR, readCode())
        Assert.Equal(None, reader.ReadMessage())

    [<Fact>]
    let ``server sends parse errors and ignores responses`` () =
        let input =
            Array.append
                (frame "NOT JSON")
                (frame """{"jsonrpc":"2.0","id":1,"result":42}""")

        use inStream = new MemoryStream(input)
        use outStream = new MemoryStream()

        Server(inStream, outStream).Serve()

        outStream.Position <- 0L
        match (MessageReader outStream).ReadMessage() with
        | Some(ResponseMessage response) -> Assert.Equal(JsonRpc.PARSE_ERROR, response.Error.Value.Code)
        | _ -> failwith "expected parse error"

        Assert.Equal(None, (MessageReader outStream).ReadMessage())
