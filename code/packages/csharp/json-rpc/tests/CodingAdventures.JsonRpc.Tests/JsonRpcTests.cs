using System.Text;
using System.Text.Json.Nodes;

namespace CodingAdventures.JsonRpc.Tests;

public sealed class JsonRpcTests
{
    [Fact]
    public void ParsesRequestsNotificationsAndResponses()
    {
        var request = Assert.IsType<Request>(JsonRpc.ParseMessage("""{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"x":1}}"""));
        Assert.Equal(1, request.Id!.GetValue<int>());
        Assert.Equal("initialize", request.Method);
        Assert.Equal(1, request.Params!["x"]!.GetValue<int>());

        var notification = Assert.IsType<Notification>(JsonRpc.ParseMessage("""{"jsonrpc":"2.0","method":"exit"}"""));
        Assert.Equal("exit", notification.Method);
        Assert.Null(notification.Params);

        var response = Assert.IsType<Response>(JsonRpc.ParseMessage("""{"jsonrpc":"2.0","id":"abc","result":{"ok":true}}"""));
        Assert.Equal("abc", response.Id!.GetValue<string>());
        Assert.True(response.Result!["ok"]!.GetValue<bool>());
    }

    [Fact]
    public void ParsesErrorResponsesAndReportsInvalidMessages()
    {
        var response = Assert.IsType<Response>(JsonRpc.ParseMessage("""{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found","data":"hover"}}"""));
        Assert.NotNull(response.Error);
        Assert.Equal(JsonRpc.MethodNotFound, response.Error!.Code);
        Assert.Equal("hover", response.Error.Data!.GetValue<string>());

        AssertRpcError(JsonRpc.ParseError, () => JsonRpc.ParseMessage("{not json}"));
        AssertRpcError(JsonRpc.InvalidRequest, () => JsonRpc.ParseMessage("""[{"jsonrpc":"2.0"}]"""));
        AssertRpcError(JsonRpc.InvalidRequest, () => JsonRpc.ParseMessage("""{"jsonrpc":"1.0","method":"x"}"""));
        AssertRpcError(JsonRpc.InvalidRequest, () => JsonRpc.ParseMessage("""{"jsonrpc":"2.0","id":null,"method":"x"}"""));
        AssertRpcError(JsonRpc.InvalidRequest, () => JsonRpc.ParseMessage("""{"jsonrpc":"2.0","id":1,"error":"bad"}"""));
        AssertRpcError(JsonRpc.InvalidRequest, () => JsonRpc.ParseMessage("""{"jsonrpc":"2.0","id":1,"error":{"code":"oops","message":"bad"}}"""));
    }

    [Fact]
    public void ConvertsMessagesToWireObjects()
    {
        var request = JsonRpc.MessageToNode(new Request(JsonValue.Create(1)!, "foo", new JsonObject { ["a"] = 1 }));
        Assert.Equal("2.0", request["jsonrpc"]!.GetValue<string>());
        Assert.Equal("foo", request["method"]!.GetValue<string>());
        Assert.Equal(1, request["params"]!["a"]!.GetValue<int>());

        var notification = JsonRpc.MessageToNode(new Notification("exit"));
        Assert.False(notification.ContainsKey("id"));
        Assert.False(notification.ContainsKey("params"));

        var error = JsonRpc.MessageToNode(new Response(JsonValue.Create("id")!, Error: new ResponseError(JsonRpc.InvalidParams, "Bad", JsonValue.Create("detail"))));
        Assert.Equal(JsonRpc.InvalidParams, error["error"]!["code"]!.GetValue<int>());
        Assert.Equal("detail", error["error"]!["data"]!.GetValue<string>());

        var success = JsonRpc.MessageToNode(new Response(JsonValue.Create(2)!, Result: null));
        Assert.True(success.ContainsKey("result"));
    }

    [Fact]
    public void ReaderHandlesFramingAndEof()
    {
        var raw1 = """{"jsonrpc":"2.0","id":1,"method":"foo"}""";
        var raw2 = """{"jsonrpc":"2.0","method":"bar"}""";
        using var stream = new MemoryStream(Frame(raw1).Concat(Frame(raw2)).ToArray());
        var reader = new MessageReader(stream);

        Assert.IsType<Request>(reader.ReadMessage());
        Assert.IsType<Notification>(reader.ReadMessage());
        Assert.Null(reader.ReadMessage());

        using var rawStream = new MemoryStream(Frame(raw1, "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n"));
        Assert.Equal(raw1, new MessageReader(rawStream).ReadRaw());
    }

    [Fact]
    public void ReaderReportsMalformedFrames()
    {
        AssertRpcError(JsonRpc.ParseError, () => new MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Type: text/plain\r\n\r\nhello"))).ReadMessage());
        AssertRpcError(JsonRpc.ParseError, () => new MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: nope\r\n\r\n{}"))).ReadMessage());
        AssertRpcError(JsonRpc.ParseError, () => new MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: -1\r\n\r\n"))).ReadMessage());
        AssertRpcError(JsonRpc.ParseError, () => new MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: 10\r\n\r\nshort"))).ReadMessage());
        AssertRpcError(JsonRpc.ParseError, () => new MessageReader(new MemoryStream(Encoding.ASCII.GetBytes("Content-Length: 1\r\n\r\n\xFF"))).ReadMessage());
    }

    [Fact]
    public void WriterProducesContentLengthFrames()
    {
        using var stream = new MemoryStream();
        var writer = new MessageWriter(stream);
        writer.WriteMessage(new Request(JsonValue.Create(1)!, "ping", new JsonObject { ["text"] = "日本語" }));

        var data = stream.ToArray();
        var separator = IndexOf(data, Encoding.ASCII.GetBytes("\r\n\r\n"));
        Assert.True(separator > 0);
        var header = Encoding.ASCII.GetString(data[..separator]);
        var payload = data[(separator + 4)..];
        var declared = int.Parse(header.Split(':')[1].Trim());

        Assert.Equal(declared, payload.Length);
        Assert.Contains("日本語", Encoding.UTF8.GetString(payload));
    }

    [Fact]
    public void RoundTripsMessagesThroughWriterAndReader()
    {
        using var stream = new MemoryStream();
        var writer = new MessageWriter(stream);
        writer.WriteMessage(new Notification("initialized", new JsonObject { ["ok"] = true }));
        stream.Position = 0;

        var recovered = Assert.IsType<Notification>(new MessageReader(stream).ReadMessage());
        Assert.Equal("initialized", recovered.Method);
        Assert.True(recovered.Params!["ok"]!.GetValue<bool>());
    }

    [Fact]
    public void ServerDispatchesRequestsAndNotifications()
    {
        var input = Frame("""{"jsonrpc":"2.0","id":1,"method":"add","params":{"a":1,"b":2}}""")
            .Concat(Frame("""{"jsonrpc":"2.0","method":"ping","params":{"seen":true}}"""))
            .ToArray();
        using var inStream = new MemoryStream(input);
        using var outStream = new MemoryStream();
        var notified = false;

        new Server(inStream, outStream)
            .OnRequest("add", (_, p) => JsonValue.Create(p!["a"]!.GetValue<int>() + p["b"]!.GetValue<int>()))
            .OnNotification("ping", p => notified = p!["seen"]!.GetValue<bool>())
            .Serve();

        Assert.True(notified);
        outStream.Position = 0;
        var response = Assert.IsType<Response>(new MessageReader(outStream).ReadMessage());
        Assert.Equal(3, response.Result!.GetValue<int>());
    }

    [Fact]
    public void ServerHandlesErrors()
    {
        var input = Frame("""{"jsonrpc":"2.0","id":1,"method":"unknown"}""")
            .Concat(Frame("""{"jsonrpc":"2.0","id":2,"method":"bad"}"""))
            .Concat(Frame("""{"jsonrpc":"2.0","id":3,"method":"boom"}"""))
            .Concat(Frame("""{"jsonrpc":"2.0","method":"missing"}"""))
            .ToArray();
        using var inStream = new MemoryStream(input);
        using var outStream = new MemoryStream();

        new Server(inStream, outStream)
            .OnRequest("bad", (_, _) => new ResponseError(JsonRpc.InvalidParams, "Bad params"))
            .OnRequest("boom", (_, _) => throw new InvalidOperationException("boom"))
            .Serve();

        outStream.Position = 0;
        var reader = new MessageReader(outStream);
        Assert.Equal(JsonRpc.MethodNotFound, Assert.IsType<Response>(reader.ReadMessage()).Error!.Code);
        Assert.Equal(JsonRpc.InvalidParams, Assert.IsType<Response>(reader.ReadMessage()).Error!.Code);
        Assert.Equal(JsonRpc.InternalError, Assert.IsType<Response>(reader.ReadMessage()).Error!.Code);
        Assert.Null(reader.ReadMessage());
    }

    [Fact]
    public void ServerSendsParseErrorsAndIgnoresResponses()
    {
        var input = Frame("NOT JSON")
            .Concat(Frame("""{"jsonrpc":"2.0","id":1,"result":42}"""))
            .ToArray();
        using var inStream = new MemoryStream(input);
        using var outStream = new MemoryStream();

        new Server(inStream, outStream).Serve();

        outStream.Position = 0;
        var response = Assert.IsType<Response>(new MessageReader(outStream).ReadMessage());
        Assert.Equal(JsonRpc.ParseError, response.Error!.Code);
        Assert.Null(new MessageReader(outStream).ReadMessage());
    }

    private static void AssertRpcError(int code, Action action)
    {
        var ex = Assert.Throws<JsonRpcException>(action);
        Assert.Equal(code, ex.Code);
    }

    private static byte[] Frame(string json, string extraHeader = "")
    {
        var payload = Encoding.UTF8.GetBytes(json);
        return Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n{extraHeader}\r\n")
            .Concat(payload)
            .ToArray();
    }

    private static int IndexOf(byte[] haystack, byte[] needle)
    {
        for (var i = 0; i <= haystack.Length - needle.Length; i++)
        {
            if (haystack.AsSpan(i, needle.Length).SequenceEqual(needle))
            {
                return i;
            }
        }

        return -1;
    }
}
