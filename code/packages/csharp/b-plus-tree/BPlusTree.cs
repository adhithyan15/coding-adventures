using System.Collections;

namespace CodingAdventures.BPlusTree;

/// <summary>
/// A B+ tree mapping comparable keys to values.
/// </summary>
public sealed class BPlusTree<TKey, TValue> : IEnumerable<KeyValuePair<TKey, TValue>>
    where TKey : notnull, IComparable<TKey>
{
    private readonly int _minimumDegree;
    private readonly SortedDictionary<TKey, TValue> _entries = [];
    private Node _root;
    private LeafNode _firstLeaf;

    public BPlusTree(int minimumDegree = 2)
    {
        if (minimumDegree < 2)
        {
            throw new ArgumentOutOfRangeException(nameof(minimumDegree), "Minimum degree must be at least 2.");
        }

        _minimumDegree = minimumDegree;
        _firstLeaf = new LeafNode();
        _root = _firstLeaf;
    }

    public int MinimumDegree => _minimumDegree;

    public int Count => _entries.Count;

    public int Size => Count;

    public bool IsEmpty => Count == 0;

    private int MaxKeys => 2 * _minimumDegree - 1;

    private int MaxChildren => 2 * _minimumDegree;

    public void Insert(TKey key, TValue value)
    {
        ArgumentNullException.ThrowIfNull(key);

        _entries[key] = value;
        Rebuild();
    }

    public void Delete(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);

        if (_entries.Remove(key))
        {
            Rebuild();
        }
    }

    public TValue? Search(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);

        var leaf = FindLeaf(key);
        var index = FindKeyIndex(leaf.Keys, key);
        return index < leaf.Keys.Count && leaf.Keys[index].CompareTo(key) == 0
            ? leaf.Values[index]
            : default;
    }

    public bool Contains(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);

        var leaf = FindLeaf(key);
        var index = FindKeyIndex(leaf.Keys, key);
        return index < leaf.Keys.Count && leaf.Keys[index].CompareTo(key) == 0;
    }

    public TKey MinKey()
    {
        if (Count == 0)
        {
            throw new InvalidOperationException("Tree is empty.");
        }

        return _firstLeaf.Keys[0];
    }

    public TKey MaxKey()
    {
        if (Count == 0)
        {
            throw new InvalidOperationException("Tree is empty.");
        }

        var node = _root;
        while (node is InternalNode internalNode)
        {
            node = internalNode.Children[^1];
        }

        var leaf = (LeafNode)node;
        return leaf.Keys[^1];
    }

    public List<KeyValuePair<TKey, TValue>> RangeScan(TKey low, TKey high)
    {
        ArgumentNullException.ThrowIfNull(low);
        ArgumentNullException.ThrowIfNull(high);

        if (low.CompareTo(high) > 0)
        {
            throw new ArgumentException("Low key must be less than or equal to high key.", nameof(low));
        }

        var result = new List<KeyValuePair<TKey, TValue>>();
        var leaf = FindLeaf(low);
        while (leaf is not null)
        {
            for (var index = 0; index < leaf.Keys.Count; index++)
            {
                var key = leaf.Keys[index];
                if (key.CompareTo(high) > 0)
                {
                    return result;
                }

                if (key.CompareTo(low) >= 0)
                {
                    result.Add(new KeyValuePair<TKey, TValue>(key, leaf.Values[index]));
                }
            }

            leaf = leaf.Next;
        }

        return result;
    }

    public List<KeyValuePair<TKey, TValue>> RangeQuery(TKey low, TKey high) => RangeScan(low, high);

    public List<KeyValuePair<TKey, TValue>> FullScan()
    {
        var result = new List<KeyValuePair<TKey, TValue>>(Count);
        var leaf = _firstLeaf;
        while (leaf is not null)
        {
            for (var index = 0; index < leaf.Keys.Count; index++)
            {
                result.Add(new KeyValuePair<TKey, TValue>(leaf.Keys[index], leaf.Values[index]));
            }

            leaf = leaf.Next;
        }

        return result;
    }

    public List<KeyValuePair<TKey, TValue>> InOrder() => FullScan();

    public int Height()
    {
        var height = 0;
        var node = _root;
        while (node is InternalNode internalNode)
        {
            height++;
            node = internalNode.Children[0];
        }

        return height;
    }

    public bool IsValid()
    {
        if (Count == 0)
        {
            return _root is LeafNode leaf && ReferenceEquals(leaf, _firstLeaf) && leaf.Keys.Count == 0 && leaf.Next is null;
        }

        var scan = FullScan();
        if (scan.Count != Count)
        {
            return false;
        }

        using var expected = _entries.GetEnumerator();
        TKey? previous = default;
        var hasPrevious = false;
        foreach (var entry in scan)
        {
            if (hasPrevious && previous!.CompareTo(entry.Key) >= 0)
            {
                return false;
            }

            if (!expected.MoveNext())
            {
                return false;
            }

            if (!EqualityComparer<TKey>.Default.Equals(expected.Current.Key, entry.Key)
                || !EqualityComparer<TValue>.Default.Equals(expected.Current.Value, entry.Value)
                || !Contains(entry.Key)
                || !EqualityComparer<TValue>.Default.Equals(Search(entry.Key), entry.Value))
            {
                return false;
            }

            previous = entry.Key;
            hasPrevious = true;
        }

        return !expected.MoveNext();
    }

    public IEnumerator<KeyValuePair<TKey, TValue>> GetEnumerator() => FullScan().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString() => $"BPlusTree(t={_minimumDegree}, size={Count}, height={Height()})";

    private void Rebuild()
    {
        if (_entries.Count == 0)
        {
            _firstLeaf = new LeafNode();
            _root = _firstLeaf;
            return;
        }

        var pairs = _entries.ToArray();
        var leaves = new List<Node>();
        LeafNode? previous = null;
        var offset = 0;

        foreach (var size in PartitionSizes(pairs.Length, _minimumDegree - 1, MaxKeys))
        {
            var leaf = new LeafNode();
            for (var index = offset; index < offset + size; index++)
            {
                leaf.Keys.Add(pairs[index].Key);
                leaf.Values.Add(pairs[index].Value);
            }

            if (previous is null)
            {
                _firstLeaf = leaf;
            }
            else
            {
                previous.Next = leaf;
            }

            previous = leaf;
            leaves.Add(leaf);
            offset += size;
        }

        _root = BuildLevel(leaves);
    }

    private Node BuildLevel(List<Node> children)
    {
        if (children.Count == 1)
        {
            return children[0];
        }

        if (children.Count <= MaxChildren)
        {
            return BuildInternal(children);
        }

        var parents = new List<Node>();
        var offset = 0;
        foreach (var size in PartitionSizes(children.Count, _minimumDegree, MaxChildren))
        {
            parents.Add(BuildInternal(children.GetRange(offset, size)));
            offset += size;
        }

        return BuildLevel(parents);
    }

    private InternalNode BuildInternal(IReadOnlyList<Node> children)
    {
        var node = new InternalNode();
        foreach (var child in children)
        {
            node.Children.Add(child);
        }

        for (var index = 1; index < children.Count; index++)
        {
            node.Keys.Add(GetFirstKey(children[index]));
        }

        return node;
    }

    private LeafNode FindLeaf(TKey key)
    {
        var node = _root;
        while (node is InternalNode internalNode)
        {
            var index = 0;
            while (index < internalNode.Keys.Count && key.CompareTo(internalNode.Keys[index]) >= 0)
            {
                index++;
            }

            node = internalNode.Children[index];
        }

        return (LeafNode)node;
    }

    private static TKey GetFirstKey(Node node)
    {
        while (node is InternalNode internalNode)
        {
            node = internalNode.Children[0];
        }

        var leaf = (LeafNode)node;
        return leaf.Keys[0];
    }

    private static int FindKeyIndex(IReadOnlyList<TKey> keys, TKey key)
    {
        var low = 0;
        var high = keys.Count;
        while (low < high)
        {
            var mid = (low + high) >>> 1;
            if (keys[mid].CompareTo(key) < 0)
            {
                low = mid + 1;
            }
            else
            {
                high = mid;
            }
        }

        return low;
    }

    private static List<int> PartitionSizes(int count, int minSize, int maxSize)
    {
        if (count <= maxSize)
        {
            return [count];
        }

        var groups = (count + maxSize - 1) / maxSize;
        var baseSize = count / groups;
        var remainder = count % groups;
        var sizes = new List<int>(groups);
        for (var index = 0; index < groups; index++)
        {
            var size = baseSize + (index < remainder ? 1 : 0);
            if (size < minSize || size > maxSize)
            {
                throw new InvalidOperationException("Unable to partition B+ tree nodes within degree constraints.");
            }

            sizes.Add(size);
        }

        return sizes;
    }

    private abstract class Node
    {
        public List<TKey> Keys { get; } = [];
    }

    private sealed class InternalNode : Node
    {
        public List<Node> Children { get; } = [];
    }

    private sealed class LeafNode : Node
    {
        public List<TValue> Values { get; } = [];

        public LeafNode? Next { get; set; }
    }
}
