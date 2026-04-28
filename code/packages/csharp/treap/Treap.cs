namespace CodingAdventures.Treap;

/// <summary>
/// Pure functional randomized treap for integer keys.
/// </summary>
public sealed class Treap
{
    private readonly Random _random;

    private Treap(Node? root, Random random)
    {
        Root = root;
        _random = random;
    }

    public Node? Root { get; }

    public bool IsEmpty => Root is null;

    public int Size => CountNodes(Root);

    public int Height => HeightOf(Root);

    public static Treap Empty() => new(null, new Random());

    public static Treap Empty(Random random)
    {
        ArgumentNullException.ThrowIfNull(random);
        return new Treap(null, random);
    }

    public static Treap WithSeed(int seed) => new(null, new Random(seed));

    public static Treap FromRoot(Node? root, Random? random = null) => new(root, random ?? new Random());

    public Treap Insert(int key) => Contains(key) ? this : InsertWithPriority(key, _random.NextDouble());

    public Treap InsertWithPriority(int key, double priority)
    {
        if (Contains(key))
        {
            return this;
        }

        var (left, right) = SplitStrict(Root, key);
        var singleton = new Node(key, priority);
        return new Treap(MergeNodes(MergeNodes(left, singleton), right), _random);
    }

    public Treap Delete(int key)
    {
        if (!Contains(key))
        {
            return this;
        }

        var (left, rest) = SplitStrict(Root, key);
        var (_, right) = SplitNode(rest, key);
        return new Treap(MergeNodes(left, right), _random);
    }

    public SplitResult Split(int key)
    {
        var (left, right) = SplitNode(Root, key);
        return new SplitResult(left, right);
    }

    public static Treap Merge(Treap left, Treap right)
    {
        ArgumentNullException.ThrowIfNull(left);
        ArgumentNullException.ThrowIfNull(right);
        return new Treap(MergeNodes(left.Root, right.Root), new Random());
    }

    public bool Contains(int key)
    {
        var node = Root;
        while (node is not null)
        {
            if (key < node.Key)
            {
                node = node.Left;
            }
            else if (key > node.Key)
            {
                node = node.Right;
            }
            else
            {
                return true;
            }
        }

        return false;
    }

    public int? Min()
    {
        var node = Root;
        if (node is null)
        {
            return null;
        }

        while (node.Left is not null)
        {
            node = node.Left;
        }

        return node.Key;
    }

    public int? Max()
    {
        var node = Root;
        if (node is null)
        {
            return null;
        }

        while (node.Right is not null)
        {
            node = node.Right;
        }

        return node.Key;
    }

    public int? Predecessor(int key)
    {
        int? best = null;
        var node = Root;
        while (node is not null)
        {
            if (key > node.Key)
            {
                best = node.Key;
                node = node.Right;
            }
            else
            {
                node = node.Left;
            }
        }

        return best;
    }

    public int? Successor(int key)
    {
        int? best = null;
        var node = Root;
        while (node is not null)
        {
            if (key < node.Key)
            {
                best = node.Key;
                node = node.Left;
            }
            else
            {
                node = node.Right;
            }
        }

        return best;
    }

    public int KthSmallest(int k)
    {
        var sorted = ToSortedList();
        if (k < 1 || k > sorted.Count)
        {
            throw new ArgumentOutOfRangeException(nameof(k), $"k={k} out of range; treap has {sorted.Count} elements.");
        }

        return sorted[k - 1];
    }

    public List<int> ToSortedList()
    {
        var result = new List<int>();
        InOrder(Root, result);
        return result;
    }

    public bool IsValidTreap() => CheckNode(Root, null, null, double.MaxValue);

    public override string ToString() => $"Treap(size={Size}, height={Height})";

    internal static (Node? Left, Node? Right) SplitNode(Node? node, int key)
    {
        if (node is null)
        {
            return (null, null);
        }

        if (node.Key <= key)
        {
            var (leftPart, rightPart) = SplitNode(node.Right, key);
            return (node with { Right = leftPart }, rightPart);
        }
        else
        {
            var (leftPart, rightPart) = SplitNode(node.Left, key);
            return (leftPart, node with { Left = rightPart });
        }
    }

    internal static (Node? Left, Node? Right) SplitStrict(Node? node, int key)
    {
        if (node is null)
        {
            return (null, null);
        }

        if (node.Key < key)
        {
            var (leftPart, rightPart) = SplitStrict(node.Right, key);
            return (node with { Right = leftPart }, rightPart);
        }
        else
        {
            var (leftPart, rightPart) = SplitStrict(node.Left, key);
            return (leftPart, node with { Left = rightPart });
        }
    }

    internal static Node? MergeNodes(Node? left, Node? right)
    {
        if (left is null)
        {
            return right;
        }

        if (right is null)
        {
            return left;
        }

        return left.Priority > right.Priority
            ? left with { Right = MergeNodes(left.Right, right) }
            : right with { Left = MergeNodes(left, right.Left) };
    }

    private static void InOrder(Node? node, List<int> result)
    {
        if (node is null)
        {
            return;
        }

        InOrder(node.Left, result);
        result.Add(node.Key);
        InOrder(node.Right, result);
    }

    private static bool CheckNode(Node? node, int? minKey, int? maxKey, double maxPriority)
    {
        if (node is null)
        {
            return true;
        }

        if ((minKey is not null && node.Key <= minKey) || (maxKey is not null && node.Key >= maxKey))
        {
            return false;
        }

        if (node.Priority > maxPriority)
        {
            return false;
        }

        return CheckNode(node.Left, minKey, node.Key, node.Priority)
            && CheckNode(node.Right, node.Key, maxKey, node.Priority);
    }

    private static int CountNodes(Node? node) => node is null ? 0 : 1 + CountNodes(node.Left) + CountNodes(node.Right);

    private static int HeightOf(Node? node) => node is null ? 0 : 1 + Math.Max(HeightOf(node.Left), HeightOf(node.Right));

    public sealed record Node(int Key, double Priority, Node? Left = null, Node? Right = null);

    public sealed record SplitResult(Node? Left, Node? Right);
}
