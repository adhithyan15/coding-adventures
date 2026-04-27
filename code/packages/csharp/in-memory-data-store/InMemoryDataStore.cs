using System.Text;
using CodingAdventures.InMemoryDataStoreEngine;
using CodingAdventures.InMemoryDataStoreProtocol;
using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStore;

public sealed class InMemoryDataStoreOptions
{
    public DataStoreEngine? Engine { get; init; }

    public Store? Store { get; init; }
}

public sealed class InMemoryDataStore
{
    private readonly DataStoreEngine _engine;
    private RespDecoder _decoder = new();

    public InMemoryDataStore(InMemoryDataStoreOptions? options = null)
    {
        options ??= new InMemoryDataStoreOptions();
        _engine = options.Engine ?? new DataStoreEngine(options.Store);
    }

    public Store Store => _engine.Store;

    public InMemoryDataStore RegisterModule(IDataStoreModule module)
    {
        _engine.RegisterModule(module);
        return this;
    }

    public void Reset(Store? store = null)
    {
        _engine.Reset(store ?? _engine.Store);
        _decoder = new RespDecoder();
    }

    public RespValue Execute(DataStoreCommand command) => _engine.Execute(command);

    public RespValue Execute(IReadOnlyList<string> parts) => _engine.Execute(parts);

    public RespValue ExecuteCommand(DataStoreCommand command) => _engine.Execute(command);

    public RespValue ExecuteParts(IReadOnlyList<string> parts) => _engine.Execute(parts);

    public RespValue? ExecuteFrame(RespValue frame)
    {
        if (frame is not RespArray array || array.Value is null)
        {
            return Resp.ErrorValue("ERR expected RESP array command");
        }

        if (array.Value.Count == 0)
        {
            return null;
        }

        var command = DataStoreProtocol.CommandFromResp(frame);
        return command is null ? Resp.ErrorValue("ERR expected RESP command array") : _engine.Execute(command);
    }

    public List<RespValue> Process(byte[] input)
    {
        _decoder.Feed(input);
        var responses = new List<RespValue>();
        while (_decoder.HasMessage())
        {
            var frame = _decoder.GetMessage();
            var response = ExecuteFrame(frame);
            if (response is not null)
            {
                responses.Add(response);
            }
        }

        return responses;
    }

    public List<RespValue> Process(string input) => Process(Encoding.UTF8.GetBytes(input));

    public byte[] Handle(byte[] input) => EncodeRespStream(Process(input));

    public byte[] Handle(string input) => EncodeRespStream(Process(input));

    public static byte[] EncodeRespStream(IEnumerable<RespValue> values) => ConcatBytes(values.Select(Resp.Encode));

    public static byte[] ConcatBytes(IEnumerable<byte[]> chunks)
    {
        var arrays = chunks.ToList();
        var length = arrays.Sum(chunk => chunk.Length);
        var result = new byte[length];
        var offset = 0;
        foreach (var chunk in arrays)
        {
            Buffer.BlockCopy(chunk, 0, result, offset, chunk.Length);
            offset += chunk.Length;
        }

        return result;
    }

    public static RespArray CommandToFrame(DataStoreCommand command) =>
        Resp.Array(DataStoreProtocol.CommandToParts(command).Select(Resp.BulkString).Cast<RespValue>().ToList());

    public static string FrameToResponseText(RespValue frame) =>
        frame switch
        {
            RespSimpleString simple => simple.Value,
            RespErrorValue error => error.Value,
            RespInteger integer => integer.Value.ToString(),
            RespBulkString bulk => bulk.Value ?? "(nil)",
            RespArray array => array.Value is null ? "(nil)" : $"[array:{array.Value.Count}]",
            _ => string.Empty
        };

    public static RespValue Ok() => Resp.SimpleString("OK");
}
