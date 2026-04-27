using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace CodingAdventures.JsonRpc;

public static class JsonRpc
{
    public const string Version = "0.1.0";
    public const int ParseError = -32700;
    public const int InvalidRequest = -32600;
    public const int MethodNotFound = -32601;
    public const int InvalidParams = -32602;
    public const int InternalError = -32603;

    internal static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    public static Message ParseMessage(string raw)
    {
        JsonNode? node;
        try
        {
            node = JsonNode.Parse(raw);
        }
        catch (JsonException ex)
        {
            throw new JsonRpcException(ParseError, $"Parse error: {ex.Message}");
        }

        if (node is not JsonObject obj)
        {
            throw new JsonRpcException(InvalidRequest, "Invalid Request: top-level value must be a JSON object");
        }

        if (obj["jsonrpc"]?.GetValue<string>() != "2.0")
        {
            throw new JsonRpcException(InvalidRequest, "Invalid Request: missing or wrong jsonrpc field");
        }

        if (obj.ContainsKey("result") || obj.ContainsKey("error"))
        {
            ResponseError? error = null;
            var errorNode = obj["error"];
            if (errorNode is not null)
            {
                if (errorNode is not JsonObject errorObject)
                {
                    throw new JsonRpcException(InvalidRequest, "Invalid Request: error must be a JSON object");
                }

                if (!TryGetInt(errorObject["code"], out var code))
                {
                    throw new JsonRpcException(InvalidRequest, "Invalid Request: error code must be an integer");
                }

                var message = TryGetString(errorObject["message"], out var text) ? text : string.Empty;
                error = new ResponseError(code, message, Clone(errorObject["data"]));
            }

            return new Response(Clone(obj["id"]), Clone(obj["result"]), error);
        }

        if (!TryGetString(obj["method"], out var method))
        {
            throw new JsonRpcException(InvalidRequest, "Invalid Request: method must be a string");
        }

        var parameters = Clone(obj["params"]);
        if (obj.ContainsKey("id"))
        {
            var id = obj["id"];
            if (id is null || !IsValidId(id))
            {
                throw new JsonRpcException(InvalidRequest, "Invalid Request: id must be a string or integer");
            }

            return new Request(Clone(id)!, method, parameters);
        }

        return new Notification(method, parameters);
    }

    public static JsonObject MessageToNode(Message message) =>
        message switch
        {
            Request request => RequestToNode(request),
            Response response => ResponseToNode(response),
            Notification notification => NotificationToNode(notification),
            _ => throw new ArgumentOutOfRangeException(nameof(message), "Unknown message type"),
        };

    public static JsonNode? ToJsonNode(object? value)
    {
        if (value is null)
        {
            return null;
        }

        return value switch
        {
            JsonNode node => node.DeepClone(),
            JsonElement element => JsonNode.Parse(element.GetRawText()),
            string text => JsonValue.Create(text),
            int number => JsonValue.Create(number),
            long number => JsonValue.Create(number),
            bool boolean => JsonValue.Create(boolean),
            double number => JsonValue.Create(number),
            ResponseError error => ResponseErrorToNode(error),
            _ => JsonSerializer.SerializeToNode(value, value.GetType(), JsonOptions),
        };
    }

    internal static JsonNode? Clone(JsonNode? node) => node?.DeepClone();

    internal static string ToJsonString(Message message) => MessageToNode(message).ToJsonString(JsonOptions);

    private static JsonObject RequestToNode(Request request)
    {
        var obj = BaseObject();
        obj["id"] = Clone(request.Id);
        obj["method"] = request.Method;
        if (request.Params is not null)
        {
            obj["params"] = Clone(request.Params);
        }

        return obj;
    }

    private static JsonObject ResponseToNode(Response response)
    {
        var obj = BaseObject();
        obj["id"] = Clone(response.Id);
        if (response.Error is not null)
        {
            obj["error"] = ResponseErrorToNode(response.Error);
        }
        else
        {
            obj["result"] = Clone(response.Result);
        }

        return obj;
    }

    private static JsonObject NotificationToNode(Notification notification)
    {
        var obj = BaseObject();
        obj["method"] = notification.Method;
        if (notification.Params is not null)
        {
            obj["params"] = Clone(notification.Params);
        }

        return obj;
    }

    private static JsonObject ResponseErrorToNode(ResponseError error)
    {
        var obj = new JsonObject
        {
            ["code"] = error.Code,
            ["message"] = error.Message,
        };
        if (error.Data is not null)
        {
            obj["data"] = Clone(error.Data);
        }

        return obj;
    }

    private static JsonObject BaseObject() => new() { ["jsonrpc"] = "2.0" };

    private static bool TryGetString(JsonNode? node, out string value)
    {
        value = string.Empty;
        return node is JsonValue jsonValue && jsonValue.TryGetValue(out value!);
    }

    private static bool TryGetInt(JsonNode? node, out int value)
    {
        value = default;
        return node is JsonValue jsonValue && jsonValue.TryGetValue(out value);
    }

    private static bool IsValidId(JsonNode node)
    {
        if (node is not JsonValue value)
        {
            return false;
        }

        return value.TryGetValue<string>(out _) || value.TryGetValue<int>(out _) || value.TryGetValue<long>(out _);
    }
}

public sealed class JsonRpcException : Exception
{
    public JsonRpcException(int code, string message)
        : base(message)
    {
        Code = code;
    }

    public int Code { get; }
}

public abstract record Message;

public sealed record ResponseError(int Code, string Message, JsonNode? Data = null);

public sealed record Request(JsonNode Id, string Method, JsonNode? Params = null) : Message;

public sealed record Response(JsonNode? Id, JsonNode? Result = null, ResponseError? Error = null) : Message;

public sealed record Notification(string Method, JsonNode? Params = null) : Message;

public sealed class MessageReader
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);
    private readonly Stream _stream;

    public MessageReader(Stream stream)
    {
        _stream = stream ?? throw new ArgumentNullException(nameof(stream));
    }

    public string? ReadRaw()
    {
        int? contentLength = null;

        while (true)
        {
            var lineBytes = ReadLine();
            if (lineBytes is null)
            {
                if (contentLength is null)
                {
                    return null;
                }

                throw new JsonRpcException(JsonRpc.ParseError, "Parse error: unexpected EOF in header block");
            }

            var line = Encoding.ASCII.GetString(lineBytes).TrimEnd('\r', '\n');
            if (line.Length == 0)
            {
                break;
            }

            var separator = line.IndexOf(':', StringComparison.Ordinal);
            if (separator < 0)
            {
                continue;
            }

            var name = line[..separator].Trim();
            var value = line[(separator + 1)..].Trim();
            if (!name.Equals("Content-Length", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (!int.TryParse(value, out var parsed))
            {
                throw new JsonRpcException(JsonRpc.ParseError, $"Parse error: invalid Content-Length value: {value}");
            }

            contentLength = parsed;
        }

        if (contentLength is null)
        {
            throw new JsonRpcException(JsonRpc.ParseError, "Parse error: no Content-Length header found");
        }

        if (contentLength < 0)
        {
            throw new JsonRpcException(JsonRpc.ParseError, $"Parse error: Content-Length must be non-negative, got {contentLength}");
        }

        var payload = ReadExactly(contentLength.Value);
        try
        {
            return StrictUtf8.GetString(payload);
        }
        catch (DecoderFallbackException ex)
        {
            throw new JsonRpcException(JsonRpc.ParseError, $"Parse error: payload is not valid UTF-8: {ex.Message}");
        }
    }

    public Message? ReadMessage()
    {
        var raw = ReadRaw();
        return raw is null ? null : JsonRpc.ParseMessage(raw);
    }

    private byte[]? ReadLine()
    {
        var bytes = new List<byte>();
        while (true)
        {
            var value = _stream.ReadByte();
            if (value < 0)
            {
                return bytes.Count == 0 ? null : bytes.ToArray();
            }

            bytes.Add((byte)value);
            if (value == '\n')
            {
                return bytes.ToArray();
            }
        }
    }

    private byte[] ReadExactly(int count)
    {
        var buffer = new byte[count];
        var offset = 0;
        while (offset < count)
        {
            var read = _stream.Read(buffer, offset, count - offset);
            if (read == 0)
            {
                throw new JsonRpcException(JsonRpc.ParseError, $"Parse error: expected {count} bytes but stream ended after {offset}");
            }

            offset += read;
        }

        return buffer;
    }
}

public sealed class MessageWriter
{
    private readonly Stream _stream;

    public MessageWriter(Stream stream)
    {
        _stream = stream ?? throw new ArgumentNullException(nameof(stream));
    }

    public void WriteRaw(string json)
    {
        ArgumentNullException.ThrowIfNull(json);

        var payload = Encoding.UTF8.GetBytes(json);
        var header = Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n\r\n");
        _stream.Write(header);
        _stream.Write(payload);
        _stream.Flush();
    }

    public void WriteMessage(Message message)
    {
        ArgumentNullException.ThrowIfNull(message);
        WriteRaw(JsonRpc.ToJsonString(message));
    }
}

public delegate object? RequestHandler(JsonNode id, JsonNode? parameters);

public delegate void NotificationHandler(JsonNode? parameters);

public sealed class Server
{
    private readonly MessageReader _reader;
    private readonly MessageWriter _writer;
    private readonly Dictionary<string, RequestHandler> _requestHandlers = [];
    private readonly Dictionary<string, NotificationHandler> _notificationHandlers = [];

    public Server(Stream input, Stream output)
    {
        _reader = new MessageReader(input);
        _writer = new MessageWriter(output);
    }

    public Server OnRequest(string method, RequestHandler handler)
    {
        ArgumentNullException.ThrowIfNull(method);
        ArgumentNullException.ThrowIfNull(handler);
        _requestHandlers[method] = handler;
        return this;
    }

    public Server OnNotification(string method, NotificationHandler handler)
    {
        ArgumentNullException.ThrowIfNull(method);
        ArgumentNullException.ThrowIfNull(handler);
        _notificationHandlers[method] = handler;
        return this;
    }

    public void Serve()
    {
        while (true)
        {
            Message? message;
            try
            {
                message = _reader.ReadMessage();
            }
            catch (JsonRpcException ex)
            {
                SendError(null, ex.Code, ex.Message);
                continue;
            }

            if (message is null)
            {
                break;
            }

            Dispatch(message);
        }
    }

    private void Dispatch(Message message)
    {
        switch (message)
        {
            case Request request:
                HandleRequest(request);
                break;
            case Notification notification:
                HandleNotification(notification);
                break;
        }
    }

    private void HandleRequest(Request request)
    {
        if (!_requestHandlers.TryGetValue(request.Method, out var handler))
        {
            SendError(request.Id, JsonRpc.MethodNotFound, "Method not found");
            return;
        }

        object? result;
        try
        {
            result = handler(request.Id, request.Params);
        }
        catch (Exception ex)
        {
            SendError(request.Id, JsonRpc.InternalError, $"Internal error: {ex.Message}");
            return;
        }

        var response = result is ResponseError error
            ? new Response(JsonRpc.Clone(request.Id), Error: error)
            : new Response(JsonRpc.Clone(request.Id), Result: JsonRpc.ToJsonNode(result));

        _writer.WriteMessage(response);
    }

    private void HandleNotification(Notification notification)
    {
        if (!_notificationHandlers.TryGetValue(notification.Method, out var handler))
        {
            return;
        }

        try
        {
            handler(notification.Params);
        }
        catch
        {
            // JSON-RPC notifications never receive error responses.
        }
    }

    private void SendError(JsonNode? id, int code, string message)
    {
        _writer.WriteMessage(new Response(JsonRpc.Clone(id), Error: new ResponseError(code, message)));
    }
}
