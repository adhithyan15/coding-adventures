namespace CodingAdventures.SegmentTree;

/// <summary>
/// A generic segment tree supporting range queries and point updates in O(log n).
/// </summary>
public sealed class SegmentTree<T>
{
    /// <summary>
    /// Package version.
    /// </summary>
    public const string VERSION = "0.1.0";

    private readonly T[] _tree;
    private readonly Func<T, T, T> _combine;
    private readonly T _identity;
    private readonly int _size;

    /// <summary>
    /// Build a segment tree over the provided values.
    /// </summary>
    public SegmentTree(IReadOnlyList<T> values, Func<T, T, T> combine, T identity)
    {
        ArgumentNullException.ThrowIfNull(values);
        ArgumentNullException.ThrowIfNull(combine);

        _size = values.Count;
        _combine = combine;
        _identity = identity;
        _tree = new T[Math.Max(4, 4 * _size)];
        Array.Fill(_tree, identity);

        if (_size > 0)
        {
            Build(values, node: 1, left: 0, right: _size - 1);
        }
    }

    /// <summary>
    /// Length of the source array.
    /// </summary>
    public int Size => _size;

    /// <summary>
    /// Length of the source array.
    /// </summary>
    public int Count => _size;

    /// <summary>
    /// True when the source array is empty.
    /// </summary>
    public bool IsEmpty => _size == 0;

    /// <summary>
    /// Return the aggregate over values[queryLeft..queryRight], inclusive.
    /// </summary>
    public T Query(int queryLeft, int queryRight)
    {
        if (queryLeft < 0 || queryRight >= _size || queryLeft > queryRight)
        {
            throw new ArgumentOutOfRangeException(
                nameof(queryLeft),
                $"Invalid query range [{queryLeft}, {queryRight}] for array of size {_size}.");
        }

        return QueryHelper(node: 1, left: 0, right: _size - 1, queryLeft, queryRight);
    }

    /// <summary>
    /// Replace values[index] and recompute affected ancestors.
    /// </summary>
    public void Update(int index, T value)
    {
        if (index < 0 || index >= _size)
        {
            throw new ArgumentOutOfRangeException(nameof(index), $"Index {index} out of range for array of size {_size}.");
        }

        UpdateHelper(node: 1, left: 0, right: _size - 1, index, value);
    }

    /// <summary>
    /// Reconstruct the current values from the leaf nodes.
    /// </summary>
    public IReadOnlyList<T> ToList()
    {
        var result = new List<T>(_size);
        if (_size > 0)
        {
            CollectLeaves(node: 1, left: 0, right: _size - 1, result);
        }

        return result;
    }

    /// <summary>
    /// Build a range-sum segment tree.
    /// </summary>
    public static SegmentTree<int> SumTree(IReadOnlyList<int> values) => new(values, static (a, b) => a + b, 0);

    /// <summary>
    /// Build a range-minimum segment tree.
    /// </summary>
    public static SegmentTree<int> MinTree(IReadOnlyList<int> values) => new(values, Math.Min, int.MaxValue);

    /// <summary>
    /// Build a range-maximum segment tree.
    /// </summary>
    public static SegmentTree<int> MaxTree(IReadOnlyList<int> values) => new(values, Math.Max, int.MinValue);

    /// <summary>
    /// Build a range-GCD segment tree.
    /// </summary>
    public static SegmentTree<int> GcdTree(IReadOnlyList<int> values) => new(values, Gcd, 0);

    /// <summary>
    /// Return a compact description of the tree metadata.
    /// </summary>
    public override string ToString() => $"SegmentTree{{n={_size}, identity={_identity}}}";

    private void Build(IReadOnlyList<T> values, int node, int left, int right)
    {
        if (left == right)
        {
            _tree[node] = values[left];
            return;
        }

        var mid = (left + right) / 2;
        Build(values, 2 * node, left, mid);
        Build(values, 2 * node + 1, mid + 1, right);
        _tree[node] = _combine(_tree[2 * node], _tree[2 * node + 1]);
    }

    private T QueryHelper(int node, int left, int right, int queryLeft, int queryRight)
    {
        if (right < queryLeft || left > queryRight)
        {
            return _identity;
        }

        if (queryLeft <= left && right <= queryRight)
        {
            return _tree[node];
        }

        var mid = (left + right) / 2;
        var leftResult = QueryHelper(2 * node, left, mid, queryLeft, queryRight);
        var rightResult = QueryHelper(2 * node + 1, mid + 1, right, queryLeft, queryRight);
        return _combine(leftResult, rightResult);
    }

    private void UpdateHelper(int node, int left, int right, int index, T value)
    {
        if (left == right)
        {
            _tree[node] = value;
            return;
        }

        var mid = (left + right) / 2;
        if (index <= mid)
        {
            UpdateHelper(2 * node, left, mid, index, value);
        }
        else
        {
            UpdateHelper(2 * node + 1, mid + 1, right, index, value);
        }

        _tree[node] = _combine(_tree[2 * node], _tree[2 * node + 1]);
    }

    private void CollectLeaves(int node, int left, int right, List<T> values)
    {
        if (left == right)
        {
            values.Add(_tree[node]);
            return;
        }

        var mid = (left + right) / 2;
        CollectLeaves(2 * node, left, mid, values);
        CollectLeaves(2 * node + 1, mid + 1, right, values);
    }

    private static int Gcd(int a, int b)
    {
        a = Math.Abs(a);
        b = Math.Abs(b);
        while (b != 0)
        {
            var next = a % b;
            a = b;
            b = next;
        }

        return a;
    }
}
