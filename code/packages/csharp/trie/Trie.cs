using System.Text;

namespace CodingAdventures.Trie;

/// <summary>
/// Generic trie (prefix tree) mapping string keys to values.
/// </summary>
public sealed class Trie<T>
{
    private sealed class Node
    {
        public Dictionary<char, Node> Children { get; } = [];
        public bool IsEnd { get; set; }
        public T? Value { get; set; }
    }

    private readonly Node _root = new();
    private int _count;

    /// <summary>Number of keys stored in the trie.</summary>
    public int Count => _count;

    /// <summary>Number of keys stored in the trie.</summary>
    public int Size => _count;

    /// <summary>True when no keys are stored.</summary>
    public bool IsEmpty => _count == 0;

    /// <summary>Insert or update a key.</summary>
    public void Insert(string key, T? value)
    {
        ArgumentNullException.ThrowIfNull(key);

        var node = _root;
        foreach (var ch in key)
        {
            if (!node.Children.TryGetValue(ch, out var child))
            {
                child = new Node();
                node.Children.Add(ch, child);
            }

            node = child;
        }

        if (!node.IsEnd)
        {
            node.IsEnd = true;
            _count++;
        }

        node.Value = value;
    }

    /// <summary>Return the value for a key, or default when absent.</summary>
    public T? Get(string key)
    {
        var node = FindNode(key);
        return node is { IsEnd: true } ? node.Value : default;
    }

    /// <summary>Try to get the value for a key.</summary>
    public bool TryGetValue(string key, out T? value)
    {
        var node = FindNode(key);
        if (node is { IsEnd: true })
        {
            value = node.Value;
            return true;
        }

        value = default;
        return false;
    }

    /// <summary>Return true if the complete key exists.</summary>
    public bool Contains(string key)
    {
        var node = FindNode(key);
        return node is { IsEnd: true };
    }

    /// <summary>Return true if any key starts with the prefix.</summary>
    public bool StartsWith(string prefix) => FindNode(prefix) is not null;

    /// <summary>Delete a key if present.</summary>
    public bool Delete(string key)
    {
        var node = FindNode(key);
        if (node is not { IsEnd: true })
        {
            return false;
        }

        node.IsEnd = false;
        node.Value = default;
        _count--;
        return true;
    }

    /// <summary>Return all stored keys that start with prefix.</summary>
    public List<string> KeysWithPrefix(string prefix)
    {
        ArgumentNullException.ThrowIfNull(prefix);

        var results = new List<string>();
        var node = FindNode(prefix);
        if (node is not null)
        {
            CollectKeys(node, new StringBuilder(prefix), results);
        }

        return results;
    }

    /// <summary>Return all stored keys.</summary>
    public List<string> Keys() => KeysWithPrefix(string.Empty);

    private Node? FindNode(string? key)
    {
        if (key is null)
        {
            return null;
        }

        var node = _root;
        foreach (var ch in key)
        {
            if (!node.Children.TryGetValue(ch, out var child))
            {
                return null;
            }

            node = child;
        }

        return node;
    }

    private static void CollectKeys(Node node, StringBuilder prefix, List<string> results)
    {
        if (node.IsEnd)
        {
            results.Add(prefix.ToString());
        }

        foreach (var (ch, child) in node.Children)
        {
            prefix.Append(ch);
            CollectKeys(child, prefix, results);
            prefix.Length--;
        }
    }
}
