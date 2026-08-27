using System.Text;

namespace CodingAdventures.ScytaleCipher;

public readonly record struct BruteForceResult(int Key, string Text);

public static class ScytaleCipher
{
    public const int MaxBruteForceTextLength = 4096;

    public static string Encrypt(string text, int key)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (text.Length == 0)
        {
            return string.Empty;
        }

        var scalars = text.EnumerateRunes().ToArray();
        ValidateKey(scalars.Length, key);

        var rowCount = (scalars.Length + key - 1) / key;
        var paddedLength = rowCount * key;
        var padded = new Rune[paddedLength];
        scalars.CopyTo(padded, 0);
        Array.Fill(padded, new Rune(' '), scalars.Length, paddedLength - scalars.Length);
        var result = new StringBuilder(paddedLength);

        for (var column = 0; column < key; column++)
        {
            for (var row = 0; row < rowCount; row++)
            {
                result.Append(padded[(row * key) + column]);
            }
        }

        return result.ToString();
    }

    public static string Decrypt(string text, int key)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (text.Length == 0)
        {
            return string.Empty;
        }

        var scalars = text.EnumerateRunes().ToArray();
        ValidateKey(scalars.Length, key);

        var rowCount = (scalars.Length + key - 1) / key;
        var fullColumns = scalars.Length % key == 0 ? key : scalars.Length % key;
        var columnStarts = new int[key];
        var columnLengths = new int[key];
        var offset = 0;

        for (var column = 0; column < key; column++)
        {
            columnStarts[column] = offset;
            var columnLength = scalars.Length % key == 0 || column < fullColumns ? rowCount : rowCount - 1;
            columnLengths[column] = columnLength;
            offset += columnLength;
        }

        var result = new List<Rune>(scalars.Length);
        for (var row = 0; row < rowCount; row++)
        {
            for (var column = 0; column < key; column++)
            {
                if (row < columnLengths[column])
                {
                    result.Add(scalars[columnStarts[column] + row]);
                }
            }
        }

        while (result.Count > 0 && result[^1].Value == 0x20)
        {
            result.RemoveAt(result.Count - 1);
        }

        var plaintext = new StringBuilder(result.Count);
        foreach (var scalar in result)
        {
            plaintext.Append(scalar);
        }
        return plaintext.ToString();
    }

    public static IReadOnlyList<BruteForceResult> BruteForce(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        var scalarLength = text.EnumerateRunes().Count();
        if (scalarLength > MaxBruteForceTextLength)
        {
            throw new ArgumentOutOfRangeException(nameof(text), "scytale-brute-force-limit");
        }
        if (scalarLength < 4)
        {
            return [];
        }

        var maxKey = scalarLength / 2;
        var results = new List<BruteForceResult>(maxKey - 1);
        for (var key = 2; key <= maxKey; key++)
        {
            results.Add(new BruteForceResult(key, Decrypt(text, key)));
        }

        return results;
    }

    private static void ValidateKey(int textLength, int key)
    {
        if (key < 2)
        {
            throw new ArgumentOutOfRangeException(nameof(key), key, "Key must be >= 2.");
        }

        if (key > textLength)
        {
            throw new ArgumentOutOfRangeException(nameof(key), key, "Key must be <= text length.");
        }
    }
}
