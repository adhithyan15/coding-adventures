using CodingAdventures.HashMap;

namespace CodingAdventures.InMemoryDataStoreEngine;

public sealed class Database
{
    public Database(HashMap<string, Entry>? entries = null, CodingAdventures.Heap.MinHeap<(long ExpiresAt, string Key)>? ttlHeap = null)
    {
        Entries = entries ?? new HashMap<string, Entry>();
        TtlHeap = ttlHeap ?? DataStoreTypes.CreateExpiryHeap();
    }

    public HashMap<string, Entry> Entries { get; private set; }

    public CodingAdventures.Heap.MinHeap<(long ExpiresAt, string Key)> TtlHeap { get; private set; }

    public static Database Empty() => new();

    public static long CurrentTimeMs() => DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();

    public Database Clone() => new(Entries.Clone(), DataStoreTypes.CreateExpiryHeap(TtlHeap.ToArray()));

    public Entry? Get(string key)
    {
        if (!Entries.Has(key))
        {
            return null;
        }

        var entry = Entries.Get(key)!;
        if (entry.ExpiresAt is long expiresAt && CurrentTimeMs() >= expiresAt)
        {
            return null;
        }

        return entry;
    }

    public Database Set(string key, Entry entry)
    {
        var next = Clone();
        next.Entries = next.Entries.Set(key, DataStoreTypes.CloneEntry(entry));
        if (entry.ExpiresAt is long expiresAt)
        {
            next.TtlHeap.Push((expiresAt, key));
        }

        return next;
    }

    public Database Delete(string key)
    {
        var next = Clone();
        next.Entries = next.Entries.Delete(key);
        return next;
    }

    public bool Exists(string key) => Get(key) is not null;

    public EntryType? TypeOf(string key) => Get(key)?.EntryType;

    public List<string> Keys(string pattern) =>
        Entries.Keys()
            .Where(key => Get(key) is not null && GlobMatch(pattern, key))
            .OrderBy(key => key, StringComparer.Ordinal)
            .ToList();

    public int DbSize() => Entries.Keys().Count(key => Get(key) is not null);

    public Database ExpireLazy(string? key = null)
    {
        if (key is null || !Entries.Has(key))
        {
            return Clone();
        }

        var entry = Entries.Get(key)!;
        if (entry.ExpiresAt is null || CurrentTimeMs() < entry.ExpiresAt.Value)
        {
            return Clone();
        }

        return Delete(key);
    }

    public Database ActiveExpire()
    {
        var next = Clone();
        var now = CurrentTimeMs();
        while (!next.TtlHeap.IsEmpty())
        {
            var (expiresAt, key) = next.TtlHeap.Peek();
            if (expiresAt > now)
            {
                break;
            }

            next.TtlHeap.Pop();
            if (next.Entries.Has(key))
            {
                var current = next.Entries.Get(key)!;
                if (current.ExpiresAt == expiresAt)
                {
                    next.Entries = next.Entries.Delete(key);
                }
            }
        }

        return next;
    }

    public Database Clear() => Empty();

    private static bool GlobMatch(string pattern, string text)
    {
        var pi = 0;
        var ti = 0;
        var star = -1;
        var match = 0;

        while (ti < text.Length)
        {
            if (pi < pattern.Length && (pattern[pi] == '?' || pattern[pi] == text[ti]))
            {
                pi += 1;
                ti += 1;
            }
            else if (pi < pattern.Length && pattern[pi] == '*')
            {
                star = pi;
                match = ti;
                pi += 1;
            }
            else if (star != -1)
            {
                pi = star + 1;
                match += 1;
                ti = match;
            }
            else
            {
                return false;
            }
        }

        while (pi < pattern.Length && pattern[pi] == '*')
        {
            pi += 1;
        }

        return pi == pattern.Length;
    }
}

public sealed class Store
{
    public const int DefaultDbCount = 16;

    public Store(List<Database>? databases = null, int activeDb = 0)
    {
        Databases = databases ?? CreateDatabases(DefaultDbCount);
        ActiveDb = ClampDb(activeDb, Databases.Count);
    }

    public List<Database> Databases { get; private set; }

    public int ActiveDb { get; private set; }

    public static Store Empty(int dbCount = DefaultDbCount) => new(CreateDatabases(dbCount), 0);

    public Store Clone() => new(Databases.Select(database => database.Clone()).ToList(), ActiveDb);

    public Store WithActiveDb(int activeDb) => new(Databases.Select(database => database.Clone()).ToList(), ClampDb(activeDb, Databases.Count));

    public Store Select(int activeDb) => WithActiveDb(activeDb);

    public Database CurrentDb() => Databases[ActiveDb];

    public Entry? Get(string key) => CurrentDb().Get(key);

    public Store Set(string key, Entry entry)
    {
        var next = Clone();
        next.Databases[next.ActiveDb] = next.CurrentDb().Set(key, entry);
        return next;
    }

    public Store Delete(string key)
    {
        var next = Clone();
        next.Databases[next.ActiveDb] = next.CurrentDb().Delete(key);
        return next;
    }

    public bool Exists(string key) => Get(key) is not null;

    public List<string> Keys(string pattern) => CurrentDb().Keys(pattern);

    public EntryType? TypeOf(string key) => CurrentDb().TypeOf(key);

    public int DbSize() => CurrentDb().DbSize();

    public Store ExpireLazy(string? key = null)
    {
        var next = Clone();
        next.Databases[next.ActiveDb] = next.CurrentDb().ExpireLazy(key);
        return next;
    }

    public Store ActiveExpire()
    {
        var next = Clone();
        next.Databases[next.ActiveDb] = next.CurrentDb().ActiveExpire();
        return next;
    }

    public Store ActiveExpireAll()
    {
        var next = Clone();
        next.Databases = next.Databases.Select(database => database.ActiveExpire()).ToList();
        return next;
    }

    public Store FlushDb()
    {
        var next = Clone();
        next.Databases[next.ActiveDb] = Database.Empty();
        return next;
    }

    public Store FlushAll() => new(CreateDatabases(Databases.Count), ActiveDb);

    private static List<Database> CreateDatabases(int count) => Enumerable.Range(0, count).Select(_ => Database.Empty()).ToList();

    private static int ClampDb(int index, int length)
    {
        if (length <= 0)
        {
            return 0;
        }

        return Math.Min(Math.Max(0, index), length - 1);
    }
}
