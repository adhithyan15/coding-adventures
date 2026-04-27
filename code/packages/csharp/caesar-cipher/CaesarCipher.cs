namespace CodingAdventures.CaesarCipher;

/// <summary>
/// One brute-force-friendly classical cipher plus the simplest statistical
/// attack against it.
/// </summary>
public readonly record struct BruteForceResult(int Shift, string Plaintext);

/// <summary>
/// Encrypt, decrypt, and attack Caesar ciphers.
/// </summary>
public static class CaesarCipher
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

    private static int NormalizeShift(int shift) => ((shift % 26) + 26) % 26;

    private static char ShiftChar(char ch, int normalizedShift)
    {
        if (ch is >= 'A' and <= 'Z')
        {
            var position = ch - 'A';
            return (char)('A' + (position + normalizedShift) % 26);
        }

        if (ch is >= 'a' and <= 'z')
        {
            var position = ch - 'a';
            return (char)('a' + (position + normalizedShift) % 26);
        }

        return ch;
    }

    /// <summary>
    /// Shift alphabetic characters forward by the requested amount.
    /// </summary>
    public static string Encrypt(string text, int shift)
    {
        ArgumentNullException.ThrowIfNull(text);

        var normalizedShift = NormalizeShift(shift);
        var chars = text.ToCharArray();
        for (var i = 0; i < chars.Length; i++)
        {
            chars[i] = ShiftChar(chars[i], normalizedShift);
        }

        return new string(chars);
    }

    /// <summary>
    /// Shift alphabetic characters backward by the requested amount.
    /// </summary>
    public static string Decrypt(string text, int shift)
    {
        ArgumentNullException.ThrowIfNull(text);
        return Encrypt(text, -shift);
    }

    /// <summary>
    /// Apply the special self-inverse shift-13 variant.
    /// </summary>
    public static string Rot13(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        return Encrypt(text, 13);
    }

    /// <summary>
    /// Try every non-trivial shift and return the resulting plaintexts.
    /// </summary>
    public static IReadOnlyList<BruteForceResult> BruteForce(string ciphertext)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);

        var results = new List<BruteForceResult>(25);
        for (var shift = 1; shift <= 25; shift++)
        {
            results.Add(new BruteForceResult(shift, Decrypt(ciphertext, shift)));
        }

        return results;
    }

    private static int[] LetterCounts(string text)
    {
        var counts = new int[26];
        foreach (var ch in text)
        {
            if (char.IsAsciiLetter(ch))
            {
                counts[char.ToUpperInvariant(ch) - 'A'] += 1;
            }
        }

        return counts;
    }

    private static double ChiSquared(string text)
    {
        var counts = LetterCounts(text);
        var total = counts.Sum();
        if (total == 0)
        {
            return double.MaxValue;
        }

        var totalAsDouble = (double)total;
        var sum = 0.0;
        for (var i = 0; i < 26; i++)
        {
            var expected = totalAsDouble * EnglishFrequenciesStorage[i];
            var difference = counts[i] - expected;
            sum += difference * difference / expected;
        }

        return sum;
    }

    /// <summary>
    /// Guess the most likely shift by comparing each candidate plaintext
    /// against English letter frequencies.
    /// </summary>
    public static (int Shift, string Plaintext) FrequencyAnalysis(string ciphertext)
    {
        ArgumentNullException.ThrowIfNull(ciphertext);

        var bestShift = 1;
        var bestPlaintext = Decrypt(ciphertext, 1);
        var bestScore = ChiSquared(bestPlaintext);

        for (var shift = 2; shift <= 25; shift++)
        {
            var candidate = Decrypt(ciphertext, shift);
            var score = ChiSquared(candidate);
            if (score < bestScore)
            {
                bestShift = shift;
                bestPlaintext = candidate;
                bestScore = score;
            }
        }

        return (bestShift, bestPlaintext);
    }
}
