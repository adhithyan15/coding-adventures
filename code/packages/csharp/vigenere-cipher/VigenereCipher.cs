using System.Text;

namespace CodingAdventures.VigenereCipher;

/// <summary>
/// Result from ciphertext-only Vigenere analysis.
/// </summary>
public readonly record struct BreakResult(string Key, string Plaintext);

/// <summary>
/// Encrypt, decrypt, and analyze Vigenere ciphers.
/// </summary>
public static class VigenereCipher
{
    private static readonly double[] EnglishFrequenciesStorage =
    [
        0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015,
        0.06094, 0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749,
        0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056, 0.02758,
        0.00978, 0.02360, 0.00150, 0.01974, 0.00074,
    ];

    /// <summary>
    /// Expected English letter frequencies for A through Z.
    /// </summary>
    public static IReadOnlyList<double> EnglishFrequencies => EnglishFrequenciesStorage;

    private static bool IsAsciiLetter(char ch) => ch is >= 'A' and <= 'Z' or >= 'a' and <= 'z';

    private static string ValidateKey(string key)
    {
        ArgumentNullException.ThrowIfNull(key);

        if (key.Length == 0)
        {
            throw new ArgumentException("Key must not be empty.", nameof(key));
        }

        var normalized = key.ToUpperInvariant();
        foreach (var ch in normalized)
        {
            if (ch is < 'A' or > 'Z')
            {
                throw new ArgumentException("Key must contain only ASCII letters.", nameof(key));
            }
        }

        return normalized;
    }

    private static StringBuilder ExtractAlphaUpper(string text)
    {
        var letters = new StringBuilder(text.Length);
        foreach (var ch in text)
        {
            if (IsAsciiLetter(ch))
            {
                letters.Append(char.ToUpperInvariant(ch));
            }
        }

        return letters;
    }

    private static string Transform(string text, string key, int direction)
    {
        var normalizedKey = ValidateKey(key);
        var builder = new StringBuilder(text.Length);
        var keyIndex = 0;

        foreach (var ch in text)
        {
            if (ch is >= 'A' and <= 'Z')
            {
                var shift = direction * (normalizedKey[keyIndex % normalizedKey.Length] - 'A');
                builder.Append((char)('A' + (ch - 'A' + shift + 26) % 26));
                keyIndex++;
            }
            else if (ch is >= 'a' and <= 'z')
            {
                var shift = direction * (normalizedKey[keyIndex % normalizedKey.Length] - 'A');
                builder.Append((char)('a' + (ch - 'a' + shift + 26) % 26));
                keyIndex++;
            }
            else
            {
                builder.Append(ch);
            }
        }

        return builder.ToString();
    }

    /// <summary>
    /// Encrypt plaintext using the Vigenere cipher.
    /// </summary>
    public static string Encrypt(string plaintext, string key)
    {
        ArgumentNullException.ThrowIfNull(plaintext);
        return Transform(plaintext, key, 1);
    }

    /// <summary>
    /// Decrypt ciphertext using the Vigenere cipher.
    /// </summary>
    public static string Decrypt(string ciphertext, string key)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);
        return Transform(ciphertext, key, -1);
    }

    private static double IndexOfCoincidence(int[] counts, int total)
    {
        if (total < 2)
        {
            return 0.0;
        }

        var numerator = 0L;
        foreach (var count in counts)
        {
            numerator += (long)count * (count - 1);
        }

        return (double)numerator / ((long)total * (total - 1));
    }

    /// <summary>
    /// Estimate the key length by averaging the index of coincidence for
    /// candidate key-position groups.
    /// </summary>
    public static int FindKeyLength(string ciphertext, int maxLength = 20)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);

        var letters = ExtractAlphaUpper(ciphertext);
        if (letters.Length < 2)
        {
            return 1;
        }

        var limit = Math.Min(maxLength, letters.Length / 2);
        if (limit < 2)
        {
            return 1;
        }

        var averageIcs = new double[limit + 1];
        var bestAverageIc = 0.0;

        for (var length = 2; length <= limit; length++)
        {
            var totalIc = 0.0;
            var validGroups = 0;

            for (var group = 0; group < length; group++)
            {
                var counts = new int[26];
                var groupLength = 0;
                for (var position = group; position < letters.Length; position += length)
                {
                    counts[letters[position] - 'A']++;
                    groupLength++;
                }

                if (groupLength > 1)
                {
                    totalIc += IndexOfCoincidence(counts, groupLength);
                    validGroups++;
                }
            }

            if (validGroups > 0)
            {
                averageIcs[length] = totalIc / validGroups;
                bestAverageIc = Math.Max(bestAverageIc, averageIcs[length]);
            }
        }

        if (bestAverageIc <= 0.0)
        {
            return 1;
        }

        var threshold = bestAverageIc * 0.90;
        var candidates = new List<int>();
        for (var length = 2; length <= limit; length++)
        {
            if (averageIcs[length] >= threshold)
            {
                candidates.Add(length);
            }
        }

        foreach (var smaller in candidates.ToArray())
        {
            candidates.RemoveAll(candidate => candidate != smaller && candidate % smaller == 0);
        }

        return candidates.Count == 0 ? 1 : candidates[0];
    }

    private static double ChiSquared(int[] counts, int total)
    {
        if (total == 0)
        {
            return double.PositiveInfinity;
        }

        var sum = 0.0;
        for (var i = 0; i < 26; i++)
        {
            var expected = total * EnglishFrequenciesStorage[i];
            var difference = counts[i] - expected;
            sum += difference * difference / expected;
        }

        return sum;
    }

    /// <summary>
    /// Recover the uppercase key for a known key length using chi-squared
    /// frequency analysis.
    /// </summary>
    public static string FindKey(string ciphertext, int keyLength)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);
        if (keyLength <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(keyLength), "Key length must be positive.");
        }

        var letters = ExtractAlphaUpper(ciphertext);
        var key = new char[keyLength];

        for (var group = 0; group < keyLength; group++)
        {
            var groupLetters = new List<char>();
            for (var position = group; position < letters.Length; position += keyLength)
            {
                groupLetters.Add(letters[position]);
            }

            if (groupLetters.Count == 0)
            {
                key[group] = 'A';
                continue;
            }

            var bestShift = 0;
            var bestScore = double.PositiveInfinity;
            for (var shift = 0; shift < 26; shift++)
            {
                var counts = new int[26];
                foreach (var ch in groupLetters)
                {
                    var decrypted = (ch - 'A' + 26 - shift) % 26;
                    counts[decrypted]++;
                }

                var score = ChiSquared(counts, groupLetters.Count);
                if (score < bestScore)
                {
                    bestScore = score;
                    bestShift = shift;
                }
            }

            key[group] = (char)('A' + bestShift);
        }

        return MinimalPeriod(new string(key));
    }

    private static string MinimalPeriod(string key)
    {
        for (var period = 1; period <= key.Length / 2; period++)
        {
            if (key.Length % period != 0)
            {
                continue;
            }

            var repeated = true;
            for (var i = period; i < key.Length; i++)
            {
                if (key[i] != key[i % period])
                {
                    repeated = false;
                    break;
                }
            }

            if (repeated)
            {
                return key[..period];
            }
        }

        return key;
    }

    /// <summary>
    /// Estimate the key, then decrypt the ciphertext.
    /// </summary>
    public static BreakResult BreakCipher(string ciphertext)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);

        var keyLength = FindKeyLength(ciphertext);
        var key = FindKey(ciphertext, keyLength);
        return new BreakResult(key, Decrypt(ciphertext, key));
    }
}
