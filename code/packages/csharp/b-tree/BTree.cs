namespace CodingAdventures.BTree;

/// <summary>
/// A self-balancing multi-way search tree mapping comparable keys to values.
/// </summary>
public sealed class BTree<TKey, TValue>
    where TKey : IComparable<TKey>
{
    private readonly int _minimumDegree;
    private Node _root;
    private int _count;

    public BTree(int minimumDegree = 2)
    {
        if (minimumDegree < 2)
        {
            throw new ArgumentOutOfRangeException(nameof(minimumDegree), "Minimum degree must be at least 2.");
        }

        _minimumDegree = minimumDegree;
        _root = new Node(isLeaf: true);
    }

    public int MinimumDegree => _minimumDegree;

    public int Count => _count;

    public bool IsEmpty => _count == 0;

    public void Insert(TKey key, TValue value)
    {
        ArgumentNullException.ThrowIfNull(key);

        if (_root.IsFull(_minimumDegree))
        {
            var newRoot = new Node(isLeaf: false);
            newRoot.Children.Add(_root);
            SplitChild(newRoot, 0);
            _root = newRoot;
        }

        if (InsertNonFull(_root, key, value))
        {
            _count++;
        }
    }

    public void Delete(TKey key)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (!Contains(key))
        {
            throw new KeyNotFoundException($"Key not found: {key}");
        }

        DeleteRecursive(_root, key);
        _count--;

        if (_root.Keys.Count == 0 && _root.Children.Count > 0)
        {
            _root = _root.Children[0];
        }
    }

    public TValue? Search(TKey key)
    {
        if (key is null)
        {
            return default;
        }

        return SearchRecursive(_root, key);
    }

    public bool Contains(TKey key)
    {
        if (key is null)
        {
            return false;
        }

        return ContainsRecursive(_root, key);
    }

    public TKey MinKey()
    {
        if (_count == 0)
        {
            throw new InvalidOperationException("Tree is empty.");
        }

        return MinNode(_root).Keys[0];
    }

    public TKey MaxKey()
    {
        if (_count == 0)
        {
            throw new InvalidOperationException("Tree is empty.");
        }

        var node = _root;
        while (!node.IsLeaf)
        {
            node = node.Children[^1];
        }

        return node.Keys[^1];
    }

    public List<KeyValuePair<TKey, TValue>> RangeQuery(TKey low, TKey high)
    {
        ArgumentNullException.ThrowIfNull(low);
        ArgumentNullException.ThrowIfNull(high);

        var result = new List<KeyValuePair<TKey, TValue>>();
        foreach (var entry in InOrder())
        {
            if (entry.Key.CompareTo(high) > 0)
            {
                break;
            }

            if (entry.Key.CompareTo(low) >= 0)
            {
                result.Add(entry);
            }
        }

        return result;
    }

    public List<KeyValuePair<TKey, TValue>> InOrder()
    {
        var result = new List<KeyValuePair<TKey, TValue>>(_count);
        CollectInOrder(_root, result);
        return result;
    }

    public int Height()
    {
        var node = _root;
        var height = 0;
        while (!node.IsLeaf)
        {
            node = node.Children[0];
            height++;
        }

        return height;
    }

    public bool IsValid()
    {
        if (_count == 0)
        {
            return _root.Keys.Count == 0 && _root.IsLeaf;
        }

        var leafDepth = -1;
        return Validate(_root, default!, hasMinKey: false, default!, hasMaxKey: false, 0, ref leafDepth, isRoot: true);
    }

    public override string ToString() => $"BTree(t={_minimumDegree}, size={_count}, height={Height()})";

    private void SplitChild(Node parent, int childIndex)
    {
        var child = parent.Children[childIndex];
        var right = new Node(child.IsLeaf);
        var mid = _minimumDegree - 1;

        parent.Keys.Insert(childIndex, child.Keys[mid]);
        parent.Values.Insert(childIndex, child.Values[mid]);
        parent.Children.Insert(childIndex + 1, right);

        right.Keys.AddRange(child.Keys.GetRange(mid + 1, child.Keys.Count - mid - 1));
        right.Values.AddRange(child.Values.GetRange(mid + 1, child.Values.Count - mid - 1));
        if (!child.IsLeaf)
        {
            right.Children.AddRange(child.Children.GetRange(_minimumDegree, child.Children.Count - _minimumDegree));
            child.Children.RemoveRange(_minimumDegree, child.Children.Count - _minimumDegree);
        }

        child.Keys.RemoveRange(mid, child.Keys.Count - mid);
        child.Values.RemoveRange(mid, child.Values.Count - mid);
    }

    private bool InsertNonFull(Node node, TKey key, TValue value)
    {
        var index = node.FindKeyIndex(key);
        if (index < node.Keys.Count && node.Keys[index].CompareTo(key) == 0)
        {
            node.Values[index] = value;
            return false;
        }

        if (node.IsLeaf)
        {
            node.Keys.Insert(index, key);
            node.Values.Insert(index, value);
            return true;
        }

        if (node.Children[index].IsFull(_minimumDegree))
        {
            SplitChild(node, index);
            var comparison = key.CompareTo(node.Keys[index]);
            if (comparison == 0)
            {
                node.Values[index] = value;
                return false;
            }

            if (comparison > 0)
            {
                index++;
            }
        }

        return InsertNonFull(node.Children[index], key, value);
    }

    private TValue? SearchRecursive(Node node, TKey key)
    {
        var index = node.FindKeyIndex(key);
        if (index < node.Keys.Count && node.Keys[index].CompareTo(key) == 0)
        {
            return node.Values[index];
        }

        return node.IsLeaf ? default : SearchRecursive(node.Children[index], key);
    }

    private bool ContainsRecursive(Node node, TKey key)
    {
        var index = node.FindKeyIndex(key);
        if (index < node.Keys.Count && node.Keys[index].CompareTo(key) == 0)
        {
            return true;
        }

        return !node.IsLeaf && ContainsRecursive(node.Children[index], key);
    }

    private Node MinNode(Node node)
    {
        while (!node.IsLeaf)
        {
            node = node.Children[0];
        }

        return node;
    }

    private Node MaxNode(Node node)
    {
        while (!node.IsLeaf)
        {
            node = node.Children[^1];
        }

        return node;
    }

    private void DeleteRecursive(Node node, TKey key)
    {
        var index = node.FindKeyIndex(key);
        var found = index < node.Keys.Count && node.Keys[index].CompareTo(key) == 0;

        if (found)
        {
            if (node.IsLeaf)
            {
                node.Keys.RemoveAt(index);
                node.Values.RemoveAt(index);
                return;
            }

            var leftChild = node.Children[index];
            var rightChild = node.Children[index + 1];
            if (leftChild.Keys.Count >= _minimumDegree)
            {
                var predecessor = MaxNode(leftChild);
                var predecessorKey = predecessor.Keys[^1];
                var predecessorValue = predecessor.Values[^1];
                node.Keys[index] = predecessorKey;
                node.Values[index] = predecessorValue;
                DeleteRecursive(leftChild, predecessorKey);
            }
            else if (rightChild.Keys.Count >= _minimumDegree)
            {
                var successor = MinNode(rightChild);
                var successorKey = successor.Keys[0];
                var successorValue = successor.Values[0];
                node.Keys[index] = successorKey;
                node.Values[index] = successorValue;
                DeleteRecursive(rightChild, successorKey);
            }
            else
            {
                var merged = MergeChildren(node, index);
                DeleteRecursive(merged, key);
            }

            return;
        }

        if (node.IsLeaf)
        {
            return;
        }

        var childIndex = EnsureMinKeys(node, index);
        DeleteRecursive(node.Children[childIndex], key);
    }

    private Node MergeChildren(Node parent, int leftIndex)
    {
        var left = parent.Children[leftIndex];
        var right = parent.Children[leftIndex + 1];

        left.Keys.Add(parent.Keys[leftIndex]);
        left.Values.Add(parent.Values[leftIndex]);
        parent.Keys.RemoveAt(leftIndex);
        parent.Values.RemoveAt(leftIndex);
        parent.Children.RemoveAt(leftIndex + 1);

        left.Keys.AddRange(right.Keys);
        left.Values.AddRange(right.Values);
        if (!left.IsLeaf)
        {
            left.Children.AddRange(right.Children);
        }

        return left;
    }

    private int EnsureMinKeys(Node parent, int childIndex)
    {
        var child = parent.Children[childIndex];
        if (child.Keys.Count >= _minimumDegree)
        {
            return childIndex;
        }

        if (childIndex > 0)
        {
            var leftSibling = parent.Children[childIndex - 1];
            if (leftSibling.Keys.Count >= _minimumDegree)
            {
                child.Keys.Insert(0, parent.Keys[childIndex - 1]);
                child.Values.Insert(0, parent.Values[childIndex - 1]);

                var last = leftSibling.Keys.Count - 1;
                parent.Keys[childIndex - 1] = leftSibling.Keys[last];
                parent.Values[childIndex - 1] = leftSibling.Values[last];
                leftSibling.Keys.RemoveAt(last);
                leftSibling.Values.RemoveAt(last);

                if (!leftSibling.IsLeaf)
                {
                    child.Children.Insert(0, leftSibling.Children[^1]);
                    leftSibling.Children.RemoveAt(leftSibling.Children.Count - 1);
                }

                return childIndex;
            }
        }

        if (childIndex < parent.Children.Count - 1)
        {
            var rightSibling = parent.Children[childIndex + 1];
            if (rightSibling.Keys.Count >= _minimumDegree)
            {
                child.Keys.Add(parent.Keys[childIndex]);
                child.Values.Add(parent.Values[childIndex]);

                parent.Keys[childIndex] = rightSibling.Keys[0];
                parent.Values[childIndex] = rightSibling.Values[0];
                rightSibling.Keys.RemoveAt(0);
                rightSibling.Values.RemoveAt(0);

                if (!rightSibling.IsLeaf)
                {
                    child.Children.Add(rightSibling.Children[0]);
                    rightSibling.Children.RemoveAt(0);
                }

                return childIndex;
            }
        }

        if (childIndex > 0)
        {
            MergeChildren(parent, childIndex - 1);
            return childIndex - 1;
        }

        MergeChildren(parent, childIndex);
        return childIndex;
    }

    private static void CollectInOrder(Node node, List<KeyValuePair<TKey, TValue>> result)
    {
        if (node.IsLeaf)
        {
            for (var i = 0; i < node.Keys.Count; i++)
            {
                result.Add(new KeyValuePair<TKey, TValue>(node.Keys[i], node.Values[i]));
            }

            return;
        }

        for (var i = 0; i < node.Keys.Count; i++)
        {
            CollectInOrder(node.Children[i], result);
            result.Add(new KeyValuePair<TKey, TValue>(node.Keys[i], node.Values[i]));
        }

        CollectInOrder(node.Children[^1], result);
    }

    private bool Validate(
        Node node,
        TKey minKey,
        bool hasMinKey,
        TKey maxKey,
        bool hasMaxKey,
        int depth,
        ref int leafDepth,
        bool isRoot)
    {
        var keyCount = node.Keys.Count;
        if (isRoot)
        {
            if (_count > 0 && keyCount < 1)
            {
                return false;
            }
        }
        else if (keyCount < _minimumDegree - 1 || keyCount > 2 * _minimumDegree - 1)
        {
            return false;
        }

        for (var i = 0; i < keyCount; i++)
        {
            var key = node.Keys[i];
            if (hasMinKey && key.CompareTo(minKey) <= 0)
            {
                return false;
            }

            if (hasMaxKey && key.CompareTo(maxKey) >= 0)
            {
                return false;
            }

            if (i > 0 && key.CompareTo(node.Keys[i - 1]) <= 0)
            {
                return false;
            }
        }

        if (node.IsLeaf)
        {
            if (node.Children.Count != 0)
            {
                return false;
            }

            if (leafDepth < 0)
            {
                leafDepth = depth;
            }

            return leafDepth == depth;
        }

        if (node.Children.Count != keyCount + 1)
        {
            return false;
        }

        for (var i = 0; i <= keyCount; i++)
        {
            var low = i > 0 ? node.Keys[i - 1] : minKey;
            var high = i < keyCount ? node.Keys[i] : maxKey;
            var hasLow = i > 0 || hasMinKey;
            var hasHigh = i < keyCount || hasMaxKey;
            if (!Validate(node.Children[i], low, hasLow, high, hasHigh, depth + 1, ref leafDepth, isRoot: false))
            {
                return false;
            }
        }

        return true;
    }

    private sealed class Node
    {
        public Node(bool isLeaf)
        {
            IsLeaf = isLeaf;
        }

        public bool IsLeaf { get; }

        public List<TKey> Keys { get; } = [];

        public List<TValue> Values { get; } = [];

        public List<Node> Children { get; } = [];

        public bool IsFull(int minimumDegree) => Keys.Count == 2 * minimumDegree - 1;

        public int FindKeyIndex(TKey key)
        {
            var low = 0;
            var high = Keys.Count;
            while (low < high)
            {
                var mid = (low + high) >>> 1;
                if (Keys[mid].CompareTo(key) < 0)
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
    }
}
