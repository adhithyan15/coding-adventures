namespace CodingAdventures.ScytaleCipher;

public readonly record struct BruteForceResult(int Key, string Text);

public static class ScytaleCipher
{
    public static string Encrypt(string text, int key)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (text.Length == 0)
        {
            return string.Empty;
        }

        ValidateKey(text.Length, key);

        var rowCount = (text.Length + key - 1) / key;
        var paddedLength = rowCount * key;
        var padded = text.PadRight(paddedLength).ToCharArray();
        var result = new char[paddedLength];
        var output = 0;

        for (var column = 0; column < key; column++)
        {
            for (var row = 0; row < rowCount; row++)
            {
                result[output++] = padded[(row * key) + column];
            }
        }

        return new string(result);
    }

    public static string Decrypt(string text, int key)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (text.Length == 0)
        {
            return string.Empty;
        }

        ValidateKey(text.Length, key);

        var rowCount = (text.Length + key - 1) / key;
        var fullColumns = text.Length % key == 0 ? key : text.Length % key;
        var columnStarts = new int[key];
        var columnLengths = new int[key];
        var offset = 0;

        for (var column = 0; column < key; column++)
        {
            columnStarts[column] = offset;
            var columnLength = text.Length % key == 0 || column < fullColumns ? rowCount : rowCount - 1;
            columnLengths[column] = columnLength;
            offset += columnLength;
        }

        var chars = text.ToCharArray();
        var result = new List<char>(text.Length);
        for (var row = 0; row < rowCount; row++)
        {
            for (var column = 0; column < key; column++)
            {
                if (row < columnLengths[column])
                {
                    result.Add(chars[columnStarts[column] + row]);
                }
            }
        }

        return new string(result.ToArray()).TrimEnd(' ');
    }

    public static IReadOnlyList<BruteForceResult> BruteForce(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (text.Length < 4)
        {
            return [];
        }

        var maxKey = text.Length / 2;
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
