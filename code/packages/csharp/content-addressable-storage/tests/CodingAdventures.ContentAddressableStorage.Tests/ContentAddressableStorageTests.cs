using System.Text;
using Sha1Algorithm = CodingAdventures.Sha1.Sha1;

namespace CodingAdventures.ContentAddressableStorage.Tests;

public sealed class ContentAddressableStorageTests
{
    [Fact]
    public void VersionIsStable()
    {
        Assert.Equal("0.1.0", ContentAddressableStoragePackage.Version);
    }

    [Fact]
    public void HexHelpersValidateAndRoundTrip()
    {
        var hex = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5";
        var key = Cas.HexToKey(hex);

        Assert.Equal(20, key.Length);
        Assert.Equal(hex, Cas.KeyToHex(key));
        Assert.Equal("0000000000000000000000000000000000000000", Cas.KeyToHex(new byte[20]));
        Assert.Throws<ArgumentException>(() => Cas.HexToKey("abc"));
        Assert.Throws<ArgumentException>(() => Cas.HexToKey(new string('z', 40)));
        Assert.Throws<ArgumentException>(() => Cas.KeyToHex([1, 2, 3]));
    }

    [Fact]
    public void DecodeHexPrefixPadsOddLengthOnRight()
    {
        Assert.Equal([0xa3, 0xf4], Cas.DecodeHexPrefix("a3f4"));
        Assert.Equal([0xa3, 0xf0], Cas.DecodeHexPrefix("a3f"));
        Assert.Equal([0xa0], Cas.DecodeHexPrefix("a"));
        Assert.Equal(Cas.HexToKey("a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"), Cas.DecodeHexPrefix("A3F4B2C1D0E9F8A7B6C5D4E3F2A1B0C9D8E7F6A5"));
        Assert.Throws<ArgumentException>(() => Cas.DecodeHexPrefix(""));
        Assert.Throws<ArgumentException>(() => Cas.DecodeHexPrefix("xyz"));
        Assert.Throws<ArgumentException>(() => Cas.DecodeHexPrefix(new string('a', 41)));
    }

    [Fact]
    public void MemoryStoreRoundTripsAndDeduplicates()
    {
        var cas = new ContentAddressableStore(new MemoryStore());
        var data = Encoding.UTF8.GetBytes("hello, world");

        var firstKey = cas.Put(data);
        var secondKey = cas.Put(data);

        Assert.Equal(firstKey, secondKey);
        Assert.Equal(Sha1Algorithm.Hash(data), firstKey);
        Assert.Equal(data, cas.Get(firstKey));
        Assert.True(cas.Exists(firstKey));
        Assert.False(cas.Exists(Sha1Algorithm.Hash("missing"u8.ToArray())));
    }

    [Fact]
    public void SupportsEmptyAndLargeBlobs()
    {
        var cas = new ContentAddressableStore(new MemoryStore());
        var emptyKey = cas.Put([]);
        var large = Enumerable.Repeat((byte)'x', 1024 * 1024).ToArray();
        var largeKey = cas.Put(large);

        Assert.Empty(cas.Get(emptyKey));
        Assert.Equal(large, cas.Get(largeKey));
    }

    [Fact]
    public void MissingAndCorruptedObjectsRaiseTypedErrors()
    {
        var store = new MemoryStore();
        var cas = new ContentAddressableStore(store);
        var key = cas.Put("original"u8.ToArray());
        store.Put(key, "tampered"u8.ToArray());

        var corrupted = Assert.Throws<CasCorruptedError>(() => cas.Get(key));
        Assert.Equal(key, corrupted.Key);
        Assert.Contains(Cas.KeyToHex(key), corrupted.Message);

        var missing = Sha1Algorithm.Hash("missing"u8.ToArray());
        var notFound = Assert.Throws<CasNotFoundError>(() => cas.Get(missing));
        Assert.Equal(missing, notFound.Key);
        Assert.IsAssignableFrom<CasError>(notFound);
    }

    [Fact]
    public void FindByPrefixResolvesUniqueMissingAmbiguousAndInvalidPrefixes()
    {
        var store = new MemoryStore();
        var key1 = Cas.HexToKey("abcd123400000000000000000000000000000000");
        var key2 = Cas.HexToKey("abcd1234ffffffffffffffffffffffffffffffff");
        store.Put(key1, []);
        store.Put(key2, []);
        var cas = new ContentAddressableStore(store);

        Assert.Equal(key1, cas.FindByPrefix(Cas.KeyToHex(key1)));
        Assert.Throws<CasAmbiguousPrefixError>(() => cas.FindByPrefix("abcd1234"));
        Assert.Throws<CasPrefixNotFoundError>(() => cas.FindByPrefix("0000"));
        var invalid = Assert.Throws<CasInvalidPrefixError>(() => cas.FindByPrefix("zzz"));
        Assert.Equal("zzz", invalid.Prefix);
    }

    [Fact]
    public void BackendErrorsAreWrapped()
    {
        var cas = new ContentAddressableStore(new FailingStore());

        Assert.Throws<CasStoreError>(() => cas.Put([1]));
        Assert.Throws<CasStoreError>(() => cas.Get(Sha1Algorithm.Hash([2])));
        Assert.Throws<CasStoreError>(() => cas.Exists(Sha1Algorithm.Hash([3])));
        Assert.Throws<CasStoreError>(() => cas.FindByPrefix("abcd"));
    }

    [Fact]
    public void StorePropertyReturnsWrappedStore()
    {
        var inner = new MemoryStore();
        var cas = new ContentAddressableStore(inner);

        Assert.Same(inner, cas.Store);
    }

    [Fact]
    public void LocalDiskStoreRoundTripsWithGitStyleLayout()
    {
        using var temp = new TemporaryDirectory();
        var store = new LocalDiskStore(temp.Path);
        var cas = new ContentAddressableStore(store);
        var data = "persisted to disk"u8.ToArray();

        var key = cas.Put(data);
        var hex = Cas.KeyToHex(key);
        var expectedPath = Path.Combine(temp.Path, hex[..2], hex[2..]);

        Assert.Equal(expectedPath, store.ObjectPath(key));
        Assert.True(File.Exists(expectedPath));
        Assert.Equal(data, cas.Get(key));
        Assert.True(cas.Exists(key));
        Assert.Equal(key, cas.FindByPrefix(hex[..10]));
        Assert.Empty(Directory.EnumerateFiles(temp.Path, "*.tmp", SearchOption.AllDirectories));
    }

    [Fact]
    public void LocalDiskStoreCreatesRootSkipsArtifactsAndHandlesEmptyPrefix()
    {
        using var temp = new TemporaryDirectory();
        var root = Path.Combine(temp.Path, "deep", "objects");
        var store = new LocalDiskStore(root);
        var key = Sha1Algorithm.Hash("real object"u8.ToArray());
        store.Put(key, "real object"u8.ToArray());
        var hex = Cas.KeyToHex(key);
        var bucket = Path.Combine(root, hex[..2]);

        File.WriteAllBytes(Path.Combine(bucket, "some-temp.tmp"), []);
        File.WriteAllBytes(Path.Combine(bucket, new string('z', 38)), []);
        Directory.CreateDirectory(Path.Combine(bucket, new string('a', 38)));

        Assert.True(Directory.Exists(root));
        Assert.Empty(store.KeysWithPrefix([]));
        Assert.Empty(store.KeysWithPrefix([0x00]));
        Assert.Contains(store.KeysWithPrefix([key[0]]), candidate => candidate.SequenceEqual(key));
    }

    [Fact]
    public void LocalDiskStoreSecondPutSkipsExistingFile()
    {
        using var temp = new TemporaryDirectory();
        var store = new LocalDiskStore(temp.Path);
        var data = "idempotent"u8.ToArray();
        var key = Sha1Algorithm.Hash(data);

        store.Put(key, data);
        var path = store.ObjectPath(key);
        var before = File.GetLastWriteTimeUtc(path);
        store.Put(key, data);

        Assert.Equal(before, File.GetLastWriteTimeUtc(path));
    }

    private sealed class MemoryStore : IBlobStore
    {
        private readonly Dictionary<string, byte[]> _data = [];

        public void Put(byte[] key, byte[] data)
        {
            _data[Cas.KeyToHex(key)] = data.ToArray();
        }

        public byte[] Get(byte[] key)
        {
            return _data.TryGetValue(Cas.KeyToHex(key), out var value)
                ? value.ToArray()
                : throw new FileNotFoundException(Cas.KeyToHex(key));
        }

        public bool Exists(byte[] key) => _data.ContainsKey(Cas.KeyToHex(key));

        public IReadOnlyList<byte[]> KeysWithPrefix(byte[] prefix) =>
            _data.Keys
                .Select(Cas.HexToKey)
                .Where(key => key.Take(prefix.Length).SequenceEqual(prefix))
                .ToList();
    }

    private sealed class FailingStore : IBlobStore
    {
        public void Put(byte[] key, byte[] data) => throw new IOException("put failed");

        public byte[] Get(byte[] key) => throw new IOException("get failed");

        public bool Exists(byte[] key) => throw new IOException("exists failed");

        public IReadOnlyList<byte[]> KeysWithPrefix(byte[] prefix) => throw new IOException("prefix failed");
    }

    private sealed class TemporaryDirectory : IDisposable
    {
        public TemporaryDirectory()
        {
            Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"ca-cas-{Guid.NewGuid():N}");
            Directory.CreateDirectory(Path);
        }

        public string Path { get; }

        public void Dispose()
        {
            if (Directory.Exists(Path))
            {
                Directory.Delete(Path, recursive: true);
            }
        }
    }
}
