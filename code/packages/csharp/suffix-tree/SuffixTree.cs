namespace CodingAdventures.SuffixTree;

/// <summary>
/// Simple suffix tree facade with substring search helpers.
/// </summary>
public sealed class SuffixTree
{
    private readonly string _text;

    private SuffixTree(string text)
    {
        _text = text;
    }

    public string Text => _text;

    public static SuffixTree Build(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        return new SuffixTree(text);
    }

    public static SuffixTree BuildUkkonen(string text) => Build(text);

    public IReadOnlyList<int> Search(string pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        return SearchPositions(_text, pattern);
    }

    public int CountOccurrences(string pattern) => Search(pattern).Count;

    public string LongestRepeatedSubstring() => LongestRepeatedSubstringIn(_text);

    public IReadOnlyList<string> AllSuffixes() => AllSuffixesIn(_text);

    public int NodeCount() => 1 + _text.Length;

    public static IReadOnlyList<int> Search(SuffixTree tree, string pattern)
    {
        ArgumentNullException.ThrowIfNull(tree);
        return tree.Search(pattern);
    }

    public static int CountOccurrences(SuffixTree tree, string pattern)
    {
        ArgumentNullException.ThrowIfNull(tree);
        return tree.CountOccurrences(pattern);
    }

    public static string LongestRepeatedSubstring(SuffixTree tree)
    {
        ArgumentNullException.ThrowIfNull(tree);
        return tree.LongestRepeatedSubstring();
    }

    public static string LongestCommonSubstring(string left, string right)
    {
        ArgumentNullException.ThrowIfNull(left);
        ArgumentNullException.ThrowIfNull(right);

        if (left.Length == 0 || right.Length == 0)
        {
            return string.Empty;
        }

        var dp = new int[left.Length + 1, right.Length + 1];
        var bestLength = 0;
        var bestEnd = 0;
        for (var i = 1; i <= left.Length; i++)
        {
            for (var j = 1; j <= right.Length; j++)
            {
                if (left[i - 1] != right[j - 1])
                {
                    continue;
                }

                dp[i, j] = dp[i - 1, j - 1] + 1;
                if (dp[i, j] > bestLength)
                {
                    bestLength = dp[i, j];
                    bestEnd = i;
                }
            }
        }

        return left.Substring(bestEnd - bestLength, bestLength);
    }

    public static IReadOnlyList<string> AllSuffixes(SuffixTree tree)
    {
        ArgumentNullException.ThrowIfNull(tree);
        return tree.AllSuffixes();
    }

    public static int NodeCount(SuffixTree tree)
    {
        ArgumentNullException.ThrowIfNull(tree);
        return tree.NodeCount();
    }

    private static IReadOnlyList<int> SearchPositions(string text, string pattern)
    {
        if (pattern.Length == 0)
        {
            return Enumerable.Range(0, text.Length + 1).ToArray();
        }

        if (pattern.Length > text.Length)
        {
            return Array.Empty<int>();
        }

        var positions = new List<int>();
        for (var start = 0; start <= text.Length - pattern.Length; start++)
        {
            if (string.CompareOrdinal(text, start, pattern, 0, pattern.Length) == 0)
            {
                positions.Add(start);
            }
        }

        return positions;
    }

    private static string LongestRepeatedSubstringIn(string text)
    {
        var suffixes = AllSuffixesIn(text);
        var best = string.Empty;
        for (var i = 0; i < suffixes.Count; i++)
        {
            for (var j = i + 1; j < suffixes.Count; j++)
            {
                var prefix = CommonPrefix(suffixes[i], suffixes[j]);
                if (prefix.Length > best.Length)
                {
                    best = prefix;
                }
            }
        }

        return best;
    }

    private static string CommonPrefix(string left, string right)
    {
        var length = Math.Min(left.Length, right.Length);
        var index = 0;
        while (index < length && left[index] == right[index])
        {
            index++;
        }

        return left[..index];
    }

    private static IReadOnlyList<string> AllSuffixesIn(string text)
    {
        var suffixes = new List<string>(text.Length);
        for (var start = 0; start < text.Length; start++)
        {
            suffixes.Add(text[start..]);
        }

        return suffixes;
    }
}
