namespace CodingAdventures.TestBootstrap.CorpusJson;

/// <summary>Bounded text for messages built from untrusted fixture data (CWE-400).</summary>
public static class Clip
{
    /// <summary>Truncates <paramref name="s"/> to <paramref name="limit"/> chars, marking the cut with "...".</summary>
    public static string Apply(string s, int limit)
    {
        if (s.Length <= limit)
        {
            return s;
        }

        int cut = limit;

        // Never split a surrogate pair.
        if (cut > 0 && char.IsHighSurrogate(s[cut - 1]))
        {
            cut--;
        }

        return s.Substring(0, cut) + "...";
    }
}
