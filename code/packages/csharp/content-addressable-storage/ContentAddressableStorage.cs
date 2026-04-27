using Sha1Algorithm = CodingAdventures.Sha1.Sha1;

namespace CodingAdventures.ContentAddressableStorage;

/// <summary>Package metadata for content-addressable storage.</summary>
public static class ContentAddressableStoragePackage
{
    /// <summary>The package version.</summary>
    public const string Version = "0.1.0";
}

/// <summary>Key conversion helpers for SHA-1 content-addressable storage.</summary>
public static class Cas
{
    /// <summary>The SHA-1 key length in bytes.</summary>
    public const int KeyLength = 20;

    /// <summary>Convert a 20-byte key to a lowercase 40-character hex string.</summary>
    public static string KeyToHex(byte[] key)
    {
        ValidateKey(key);
        return Convert.ToHexString(key).ToLowerInvariant();
    }

    /// <summary>Parse a 40-character hex string into a 20-byte key.</summary>
    public static byte[] HexToKey(string hex)
    {
        ArgumentNullException.ThrowIfNull(hex);
        if (hex.Length != KeyLength * 2)
        {
            throw new ArgumentException($"expected 40 hex chars, got {hex.Length}", nameof(hex));
        }

        try
        {
            return Convert.FromHexString(hex);
        }
        catch (FormatException exception)
        {
            throw new ArgumentException($"invalid hex string: {hex}", nameof(hex), exception);
        }
    }

    /// <summary>
    /// Decode a non-empty hexadecimal prefix. Odd-length prefixes are padded on the right.
    /// </summary>
    public static byte[] DecodeHexPrefix(string prefix)
    {
        ArgumentNullException.ThrowIfNull(prefix);
        if (prefix.Length == 0)
        {
            throw new ArgumentException("prefix cannot be empty", nameof(prefix));
        }

        if (prefix.Length > KeyLength * 2)
        {
            throw new ArgumentException($"prefix cannot be longer than 40 hex chars, got {prefix.Length}", nameof(prefix));
        }

        foreach (var ch in prefix)
        {
            if (!Uri.IsHexDigit(ch))
            {
                throw new ArgumentException($"invalid hex character: {ch}", nameof(prefix));
            }
        }

        var padded = prefix.Length % 2 == 0 ? prefix : prefix + "0";
        return Convert.FromHexString(padded);
    }

    internal static void ValidateKey(byte[] key)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (key.Length != KeyLength)
        {
            throw new ArgumentException($"key must be exactly 20 bytes, got {key.Length}", nameof(key));
        }
    }
}

/// <summary>Base class for all errors raised by this package.</summary>
public class CasError : Exception
{
    /// <summary>Create a CAS error.</summary>
    public CasError(string message) : base(message)
    {
    }

    /// <summary>Create a CAS error with an inner exception.</summary>
    public CasError(string message, Exception? innerException) : base(message, innerException)
    {
    }
}

/// <summary>The underlying blob store raised an unexpected exception.</summary>
public sealed class CasStoreError : CasError
{
    /// <summary>Create a store error wrapping the original exception.</summary>
    public CasStoreError(string message, Exception innerException) : base(message, innerException)
    {
    }
}

/// <summary>A requested object key does not exist in the store.</summary>
public sealed class CasNotFoundError : CasError
{
    /// <summary>Create a not-found error for a key.</summary>
    public CasNotFoundError(byte[] key) : this(key, null)
    {
    }

    internal CasNotFoundError(byte[] key, Exception? innerException) : base($"object not found: {Cas.KeyToHex(key)}", innerException)
    {
        Key = key.ToArray();
    }

    /// <summary>The missing key.</summary>
    public byte[] Key { get; }
}

/// <summary>Stored bytes do not hash to the requested key.</summary>
public sealed class CasCorruptedError : CasError
{
    /// <summary>Create a corruption error for a key.</summary>
    public CasCorruptedError(byte[] key) : base($"object corrupted: {Cas.KeyToHex(key)}")
    {
        Key = key.ToArray();
    }

    /// <summary>The requested key.</summary>
    public byte[] Key { get; }
}

/// <summary>An abbreviated key matched more than one object.</summary>
public sealed class CasAmbiguousPrefixError : CasError
{
    /// <summary>Create an ambiguous-prefix error.</summary>
    public CasAmbiguousPrefixError(string prefix) : base($"ambiguous prefix: {prefix}")
    {
        Prefix = prefix;
    }

    /// <summary>The unresolved prefix.</summary>
    public string Prefix { get; }
}

/// <summary>An abbreviated key matched no objects.</summary>
public sealed class CasPrefixNotFoundError : CasError
{
    /// <summary>Create a prefix-not-found error.</summary>
    public CasPrefixNotFoundError(string prefix) : base($"object not found for prefix: {prefix}")
    {
        Prefix = prefix;
    }

    /// <summary>The missing prefix.</summary>
    public string Prefix { get; }
}

/// <summary>An abbreviated key is empty, too long, or not valid hexadecimal.</summary>
public sealed class CasInvalidPrefixError : CasError
{
    /// <summary>Create an invalid-prefix error.</summary>
    public CasInvalidPrefixError(string prefix) : this(prefix, null)
    {
    }

    internal CasInvalidPrefixError(string prefix, Exception? innerException) : base($"invalid hex prefix: {prefix}", innerException)
    {
        Prefix = prefix;
    }

    /// <summary>The invalid prefix.</summary>
    public string Prefix { get; }
}

/// <summary>A pluggable raw blob backend for content-addressable storage.</summary>
public interface IBlobStore
{
    /// <summary>Persist data under a 20-byte SHA-1 key.</summary>
    void Put(byte[] key, byte[] data);

    /// <summary>Retrieve raw bytes by key.</summary>
    byte[] Get(byte[] key);

    /// <summary>Return whether a key exists.</summary>
    bool Exists(byte[] key);

    /// <summary>Return all keys whose first bytes match the prefix.</summary>
    IReadOnlyList<byte[]> KeysWithPrefix(byte[] prefix);
}

/// <summary>Hashes content, delegates storage, verifies reads, and resolves prefixes.</summary>
public sealed class ContentAddressableStore
{
    /// <summary>Create a CAS wrapper around a blob store.</summary>
    public ContentAddressableStore(IBlobStore store)
    {
        Store = store ?? throw new ArgumentNullException(nameof(store));
    }

    /// <summary>The wrapped blob store.</summary>
    public IBlobStore Store { get; }

    /// <summary>Hash and store data, returning its SHA-1 key.</summary>
    public byte[] Put(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var key = Sha1Algorithm.Hash(data);
        try
        {
            Store.Put(key, data);
        }
        catch (Exception exception)
        {
            throw new CasStoreError(exception.Message, exception);
        }

        return key;
    }

    /// <summary>Retrieve data by key and verify it hashes back to that key.</summary>
    public byte[] Get(byte[] key)
    {
        Cas.ValidateKey(key);
        byte[] data;
        try
        {
            data = Store.Get(key);
        }
        catch (FileNotFoundException exception)
        {
            throw new CasNotFoundError(key, exception);
        }
        catch (KeyNotFoundException exception)
        {
            throw new CasNotFoundError(key, exception);
        }
        catch (Exception exception)
        {
            throw new CasStoreError(exception.Message, exception);
        }

        if (!Sha1Algorithm.Hash(data).SequenceEqual(key))
        {
            throw new CasCorruptedError(key);
        }

        return data;
    }

    /// <summary>Return whether a key exists in the store.</summary>
    public bool Exists(byte[] key)
    {
        Cas.ValidateKey(key);
        try
        {
            return Store.Exists(key);
        }
        catch (Exception exception)
        {
            throw new CasStoreError(exception.Message, exception);
        }
    }

    /// <summary>Resolve an abbreviated hexadecimal key prefix to a unique full key.</summary>
    public byte[] FindByPrefix(string hexPrefix)
    {
        byte[] prefix;
        try
        {
            prefix = Cas.DecodeHexPrefix(hexPrefix);
        }
        catch (ArgumentException exception)
        {
            throw new CasInvalidPrefixError(hexPrefix, exception);
        }

        IReadOnlyList<byte[]> matches;
        try
        {
            matches = Store.KeysWithPrefix(prefix);
        }
        catch (Exception exception)
        {
            throw new CasStoreError(exception.Message, exception);
        }

        var ordered = matches
            .Select(key => key.ToArray())
            .OrderBy(Cas.KeyToHex, StringComparer.Ordinal)
            .ToList();

        return ordered.Count switch
        {
            0 => throw new CasPrefixNotFoundError(hexPrefix),
            1 => ordered[0],
            _ => throw new CasAmbiguousPrefixError(hexPrefix),
        };
    }
}

/// <summary>Filesystem-backed blob store using Git's two-character fanout layout.</summary>
public sealed class LocalDiskStore : IBlobStore
{
    private readonly string _root;

    /// <summary>Create a disk store rooted at a directory.</summary>
    public LocalDiskStore(string root)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(root);
        _root = root;
        Directory.CreateDirectory(_root);
    }

    /// <summary>Return the object path for a key.</summary>
    public string ObjectPath(byte[] key)
    {
        var hex = Cas.KeyToHex(key);
        return Path.Combine(_root, hex[..2], hex[2..]);
    }

    /// <inheritdoc />
    public void Put(byte[] key, byte[] data)
    {
        Cas.ValidateKey(key);
        ArgumentNullException.ThrowIfNull(data);
        var finalPath = ObjectPath(key);
        if (File.Exists(finalPath))
        {
            return;
        }

        var directory = Path.GetDirectoryName(finalPath)!;
        Directory.CreateDirectory(directory);
        var tempPath = Path.Combine(
            directory,
            $"{Path.GetFileName(finalPath)}.{Environment.ProcessId}.{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}.{Guid.NewGuid():N}.tmp");

        try
        {
            File.WriteAllBytes(tempPath, data);
            try
            {
                File.Move(tempPath, finalPath);
            }
            catch (IOException) when (File.Exists(finalPath))
            {
                File.Delete(tempPath);
            }
        }
        catch
        {
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }

            throw;
        }
    }

    /// <inheritdoc />
    public byte[] Get(byte[] key)
    {
        Cas.ValidateKey(key);
        return File.ReadAllBytes(ObjectPath(key));
    }

    /// <inheritdoc />
    public bool Exists(byte[] key)
    {
        Cas.ValidateKey(key);
        return File.Exists(ObjectPath(key));
    }

    /// <inheritdoc />
    public IReadOnlyList<byte[]> KeysWithPrefix(byte[] prefix)
    {
        ArgumentNullException.ThrowIfNull(prefix);
        if (prefix.Length == 0)
        {
            return [];
        }

        var bucket = Path.Combine(_root, prefix[0].ToString("x2"));
        if (!Directory.Exists(bucket))
        {
            return [];
        }

        var keys = new List<byte[]>();
        foreach (var path in Directory.EnumerateFiles(bucket))
        {
            var name = Path.GetFileName(path);
            if (name.Length != 38)
            {
                continue;
            }

            byte[] key;
            try
            {
                key = Cas.HexToKey(prefix[0].ToString("x2") + name);
            }
            catch (ArgumentException)
            {
                continue;
            }

            if (key.Take(prefix.Length).SequenceEqual(prefix))
            {
                keys.Add(key);
            }
        }

        return keys;
    }
}
