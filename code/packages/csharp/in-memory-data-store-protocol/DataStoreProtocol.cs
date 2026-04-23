using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStoreProtocol;

public sealed record DataStoreCommand(string Name, IReadOnlyList<string> Args);

public static class DataStoreProtocol
{
    public static string CommandName(IReadOnlyList<string> parts) => CommandFromParts(parts).Name;

    public static DataStoreCommand CommandFromParts(IReadOnlyList<string> parts)
    {
        ArgumentNullException.ThrowIfNull(parts);
        if (parts.Count == 0)
        {
            throw new InvalidOperationException("command frame cannot be empty");
        }

        return new DataStoreCommand(parts[0].Trim().ToUpperInvariant(), [.. parts.Skip(1)]);
    }

    public static List<string> CommandToParts(DataStoreCommand command)
    {
        ArgumentNullException.ThrowIfNull(command);
        return [command.Name, .. command.Args];
    }

    public static DataStoreCommand? CommandFromResp(RespValue value)
    {
        if (value is not RespArray array || array.Value is null || array.Value.Count == 0)
        {
            return null;
        }

        var parts = new List<string>();
        foreach (var element in array.Value)
        {
            var part = RespValueToString(element);
            if (part is null)
            {
                return null;
            }

            parts.Add(part);
        }

        return CommandFromParts(parts);
    }

    public static RespArray CommandToResp(DataStoreCommand command) =>
        Resp.Array(CommandToParts(command).Select(Resp.BulkString).Cast<RespValue>().ToList());

    public static RespValue CommandToRespValue(DataStoreCommand command) => CommandToResp(command);

    public static RespArray CommandFrameToResp(IReadOnlyList<string> parts) =>
        Resp.Array(parts.Select(Resp.BulkString).Cast<RespValue>().ToList());

    public static string? RespValueToString(RespValue value) =>
        value switch
        {
            RespSimpleString simple => simple.Value,
            RespErrorValue error => error.Value,
            RespInteger integer => integer.Value.ToString(),
            RespBulkString bulk => bulk.Value,
            RespArray => null,
            _ => null
        };
}
