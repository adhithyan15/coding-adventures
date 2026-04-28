using System.Collections;

namespace CodingAdventures.RadixTree;

/// <summary>
/// A compressed trie mapping string keys to values.
/// </summary>
public sealed class RadixTree<TValue> : IReadOnlyCollection<string>
{
    private readonly Node _root = new();
    private int _size;

    public RadixTree()
    {
    }

    public RadixTree(IEnumerable<KeyValuePair<string, TValue>> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);
        foreach (var entry in entries)
        {
            Insert(entry.Key, entry.Value);
        }
    }

    public int Count => _size;

    public int Size => _size;

    public bool IsEmpty => _size == 0;

    public void Insert(string key, TValue value)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (InsertRecursive(_root, key, value))
        {
            _size++;
        }
    }

    public void Put(string key, TValue value) => Insert(key, value);

    public TValue? Search(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        var node = _root;
        var remaining = key;

        while (remaining.Length > 0)
        {
            if (!node.Children.TryGetValue(FirstChar(remaining), out var edge))
            {
                return default;
            }

            var commonLength = CommonPrefixLength(remaining, edge.Label);
            if (commonLength < edge.Label.Length)
            {
                return default;
            }

            remaining = remaining[commonLength..];
            node = edge.Child;
        }

        return node.IsEnd ? node.Value : default;
    }

    public TValue? Get(string key) => Search(key);

    public bool ContainsKey(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return KeyExists(key);
    }

    public bool Delete(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        var result = DeleteRecursive(_root, key);
        if (result.Deleted)
        {
            _size--;
        }

        return result.Deleted;
    }

    public bool StartsWith(string prefix)
    {
        ArgumentNullException.ThrowIfNull(prefix);
        if (prefix.Length == 0)
        {
            return _size > 0;
        }

        var node = _root;
        var remaining = prefix;
        while (remaining.Length > 0)
        {
            if (!node.Children.TryGetValue(FirstChar(remaining), out var edge))
            {
                return false;
            }

            var commonLength = CommonPrefixLength(remaining, edge.Label);
            if (commonLength == remaining.Length)
            {
                return true;
            }

            if (commonLength < edge.Label.Length)
            {
                return false;
            }

            remaining = remaining[commonLength..];
            node = edge.Child;
        }

        return node.IsEnd || node.Children.Count > 0;
    }

    public List<string> WordsWithPrefix(string prefix)
    {
        ArgumentNullException.ThrowIfNull(prefix);
        if (prefix.Length == 0)
        {
            return Keys();
        }

        var node = _root;
        var remaining = prefix;
        var path = "";

        while (remaining.Length > 0)
        {
            if (!node.Children.TryGetValue(FirstChar(remaining), out var edge))
            {
                return [];
            }

            var commonLength = CommonPrefixLength(remaining, edge.Label);
            if (commonLength == remaining.Length)
            {
                if (commonLength == edge.Label.Length)
                {
                    path += edge.Label;
                    node = edge.Child;
                    remaining = "";
                }
                else
                {
                    var results = new List<string>();
                    CollectKeys(edge.Child, path + edge.Label, results);
                    return results;
                }
            }
            else if (commonLength < edge.Label.Length)
            {
                return [];
            }
            else
            {
                path += edge.Label;
                remaining = remaining[commonLength..];
                node = edge.Child;
            }
        }

        var matches = new List<string>();
        CollectKeys(node, path, matches);
        return matches;
    }

    public string? LongestPrefixMatch(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        var node = _root;
        var remaining = key;
        var consumed = 0;
        string? best = node.IsEnd ? "" : null;

        while (remaining.Length > 0)
        {
            if (!node.Children.TryGetValue(FirstChar(remaining), out var edge))
            {
                break;
            }

            var commonLength = CommonPrefixLength(remaining, edge.Label);
            if (commonLength < edge.Label.Length)
            {
                break;
            }

            consumed += commonLength;
            remaining = remaining[commonLength..];
            node = edge.Child;
            if (node.IsEnd)
            {
                best = key[..consumed];
            }
        }

        return best;
    }

    public List<string> Keys()
    {
        var results = new List<string>();
        CollectKeys(_root, "", results);
        return results;
    }

    public List<TValue> Values() => ToDictionary().Values.ToList();

    public SortedDictionary<string, TValue> ToDictionary()
    {
        var result = new SortedDictionary<string, TValue>(StringComparer.Ordinal);
        CollectValues(_root, "", result);
        return result;
    }

    public int NodeCount() => CountNodes(_root);

    public IEnumerator<string> GetEnumerator() => Keys().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString()
    {
        var preview = string.Join(", ", ToDictionary().Take(5).Select(pair => $"{pair.Key}={pair.Value}"));
        return $"RadixTree({_size} keys: [{preview}])";
    }

    private bool KeyExists(string key)
    {
        var node = _root;
        var remaining = key;

        while (remaining.Length > 0)
        {
            if (!node.Children.TryGetValue(FirstChar(remaining), out var edge))
            {
                return false;
            }

            var commonLength = CommonPrefixLength(remaining, edge.Label);
            if (commonLength < edge.Label.Length)
            {
                return false;
            }

            remaining = remaining[commonLength..];
            node = edge.Child;
        }

        return node.IsEnd;
    }

    private static bool InsertRecursive(Node node, string key, TValue value)
    {
        if (key.Length == 0)
        {
            var added = !node.IsEnd;
            node.IsEnd = true;
            node.Value = value;
            return added;
        }

        var first = FirstChar(key);
        if (!node.Children.TryGetValue(first, out var edge))
        {
            node.Children[first] = new Edge(key, Node.Leaf(value));
            return true;
        }

        var commonLength = CommonPrefixLength(key, edge.Label);
        if (commonLength == edge.Label.Length)
        {
            return InsertRecursive(edge.Child, key[commonLength..], value);
        }

        var common = edge.Label[..commonLength];
        var labelRest = edge.Label[commonLength..];
        var keyRest = key[commonLength..];
        var splitNode = new Node();
        splitNode.Children[FirstChar(labelRest)] = new Edge(labelRest, edge.Child);

        if (keyRest.Length == 0)
        {
            splitNode.IsEnd = true;
            splitNode.Value = value;
        }
        else
        {
            splitNode.Children[FirstChar(keyRest)] = new Edge(keyRest, Node.Leaf(value));
        }

        node.Children[first] = new Edge(common, splitNode);
        return true;
    }

    private static DeleteResult DeleteRecursive(Node node, string key)
    {
        if (key.Length == 0)
        {
            if (!node.IsEnd)
            {
                return new DeleteResult(false, false);
            }

            node.IsEnd = false;
            node.Value = default;
            return new DeleteResult(true, node.Children.Count == 1);
        }

        var first = FirstChar(key);
        if (!node.Children.TryGetValue(first, out var edge))
        {
            return new DeleteResult(false, false);
        }

        var commonLength = CommonPrefixLength(key, edge.Label);
        if (commonLength < edge.Label.Length)
        {
            return new DeleteResult(false, false);
        }

        var result = DeleteRecursive(edge.Child, key[commonLength..]);
        if (!result.Deleted)
        {
            return result;
        }

        if (result.ChildMergeable)
        {
            var grandchild = edge.Child.Children.First().Value;
            node.Children[first] = new Edge(edge.Label + grandchild.Label, grandchild.Child);
        }
        else if (!edge.Child.IsEnd && edge.Child.Children.Count == 0)
        {
            node.Children.Remove(first);
        }

        return new DeleteResult(true, !node.IsEnd && node.Children.Count == 1);
    }

    private static void CollectKeys(Node node, string current, List<string> results)
    {
        if (node.IsEnd)
        {
            results.Add(current);
        }

        foreach (var edge in node.Children.Values)
        {
            CollectKeys(edge.Child, current + edge.Label, results);
        }
    }

    private static void CollectValues(Node node, string current, SortedDictionary<string, TValue> result)
    {
        if (node.IsEnd)
        {
            result[current] = node.Value!;
        }

        foreach (var edge in node.Children.Values)
        {
            CollectValues(edge.Child, current + edge.Label, result);
        }
    }

    private static int CountNodes(Node node)
    {
        var count = 1;
        foreach (var edge in node.Children.Values)
        {
            count += CountNodes(edge.Child);
        }

        return count;
    }

    private static int CommonPrefixLength(string left, string right)
    {
        var index = 0;
        var limit = Math.Min(left.Length, right.Length);
        while (index < limit && left[index] == right[index])
        {
            index++;
        }

        return index;
    }

    private static char FirstChar(string value) => value[0];

    private sealed class Node
    {
        public bool IsEnd { get; set; }

        public TValue? Value { get; set; }

        public SortedDictionary<char, Edge> Children { get; } = new();

        public static Node Leaf(TValue value) => new()
        {
            IsEnd = true,
            Value = value,
        };
    }

    private sealed record Edge(string Label, Node Child);

    private readonly record struct DeleteResult(bool Deleted, bool ChildMergeable);
}
