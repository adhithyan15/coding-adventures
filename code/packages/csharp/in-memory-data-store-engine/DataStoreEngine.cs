using CodingAdventures.InMemoryDataStoreProtocol;
using CodingAdventures.RespProtocol;
using Resp = CodingAdventures.RespProtocol.RespProtocol;

namespace CodingAdventures.InMemoryDataStoreEngine;

public delegate (Store Store, RespValue Response) CommandHandler(Store store, IReadOnlyList<string> args);

public interface IDataStoreModule
{
    void Register(DataStoreEngine engine);
}

public sealed partial class DataStoreEngine
{
    private static readonly HashSet<string> LazyExpireExempt =
    [
        "PING",
        "ECHO",
        "SELECT",
        "INFO",
        "DBSIZE",
        "FLUSHDB",
        "FLUSHALL",
        "KEYS"
    ];

    private readonly Dictionary<string, CommandHandler> _handlers = new(StringComparer.OrdinalIgnoreCase);
    private Store _storeState;

    public DataStoreEngine(Store? store = null)
    {
        _storeState = store ?? Store.Empty();
        InstallDefaultCommands();
    }

    public Store Store => _storeState;

    public DataStoreEngine RegisterCommand(string name, CommandHandler handler)
    {
        _handlers[name.ToUpperInvariant()] = handler;
        return this;
    }

    public DataStoreEngine RegisterModule(IDataStoreModule module)
    {
        module.Register(this);
        return this;
    }

    public RespValue Execute(DataStoreCommand command) => Execute([command.Name, .. command.Args]);

    public RespValue Execute(IReadOnlyList<string> parts)
    {
        if (parts.Count == 0)
        {
            return Resp.ErrorValue("ERR empty command");
        }

        var name = DataStoreProtocol.CommandName(parts);
        if (!_handlers.TryGetValue(name, out var handler))
        {
            return Resp.ErrorValue($"ERR unknown command '{name}'");
        }

        var shouldLazyExpire = !LazyExpireExempt.Contains(name) && parts.Count > 1;
        var inputStore = shouldLazyExpire ? _storeState.ExpireLazy(parts[1]) : _storeState;
        var (nextStore, response) = handler(inputStore, parts.Skip(1).ToList());
        _storeState = nextStore;
        return response;
    }

    public RespValue ExecuteCommand(DataStoreCommand command) => Execute(command);

    public RespValue ExecuteParts(IReadOnlyList<string> parts) => Execute(parts);

    public void Reset(Store store) => _storeState = store;

    public static bool IsMutatingCommand(string name) => name.ToUpperInvariant() switch
    {
        "SET" or "DEL" or "RENAME" or "INCR" or "DECR" or "INCRBY" or "DECRBY" or "APPEND" or
        "HSET" or "HDEL" or "LPUSH" or "RPUSH" or "LPOP" or "RPOP" or "SADD" or "SREM" or
        "ZADD" or "ZREM" or "PFADD" or "PFMERGE" or "EXPIRE" or "EXPIREAT" or "PERSIST" or
        "SELECT" or "FLUSHDB" or "FLUSHALL" => true,
        _ => false
    };

    partial void InstallDefaultCommands();
}
