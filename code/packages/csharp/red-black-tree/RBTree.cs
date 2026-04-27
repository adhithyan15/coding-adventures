namespace CodingAdventures.RedBlackTree;

/// <summary>
/// Node color for a red-black tree.
/// </summary>
public enum Color
{
    /// <summary>Red node/link.</summary>
    Red,

    /// <summary>Black node/link.</summary>
    Black,
}

/// <summary>
/// A purely functional left-leaning red-black tree over integers.
/// </summary>
public sealed class RBTree
{
    /// <summary>
    /// Package version.
    /// </summary>
    public const string VERSION = "0.1.0";

    private readonly Node? _root;

    private RBTree(Node? root)
    {
        _root = root;
    }

    /// <summary>
    /// Immutable tree node.
    /// </summary>
    public sealed record Node(int Value, Color Color, Node? Left = null, Node? Right = null)
    {
        /// <summary>
        /// True when this node is red.
        /// </summary>
        public bool IsRed => Color == Color.Red;

        internal Node WithColor(Color color) => color == Color ? this : this with { Color = color };
    }

    /// <summary>
    /// Return an empty tree.
    /// </summary>
    public static RBTree Empty() => new(null);

    /// <summary>
    /// Return a new tree with value inserted. Duplicates are ignored.
    /// </summary>
    public RBTree Insert(int value)
    {
        var newRoot = InsertHelper(_root, value).WithColor(Color.Black);
        return new RBTree(newRoot);
    }

    /// <summary>
    /// Return a new tree with value removed. Missing values leave the tree unchanged.
    /// </summary>
    public RBTree Delete(int value)
    {
        if (!Contains(value))
        {
            return this;
        }

        var newRoot = _root is null ? null : DeleteHelper(_root, value);
        return new RBTree(newRoot?.WithColor(Color.Black));
    }

    /// <summary>
    /// Return true if value is present.
    /// </summary>
    public bool Contains(int value)
    {
        var node = _root;
        while (node is not null)
        {
            if (value < node.Value)
            {
                node = node.Left;
            }
            else if (value > node.Value)
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

    /// <summary>
    /// Return the minimum value, or null for an empty tree.
    /// </summary>
    public int? Min()
    {
        var node = _root;
        if (node is null)
        {
            return null;
        }

        while (node.Left is not null)
        {
            node = node.Left;
        }

        return node.Value;
    }

    /// <summary>
    /// Return the maximum value, or null for an empty tree.
    /// </summary>
    public int? Max()
    {
        var node = _root;
        if (node is null)
        {
            return null;
        }

        while (node.Right is not null)
        {
            node = node.Right;
        }

        return node.Value;
    }

    /// <summary>
    /// Return the largest value strictly less than value, or null.
    /// </summary>
    public int? Predecessor(int value)
    {
        int? best = null;
        var node = _root;
        while (node is not null)
        {
            if (value > node.Value)
            {
                best = node.Value;
                node = node.Right;
            }
            else
            {
                node = node.Left;
            }
        }

        return best;
    }

    /// <summary>
    /// Return the smallest value strictly greater than value, or null.
    /// </summary>
    public int? Successor(int value)
    {
        int? best = null;
        var node = _root;
        while (node is not null)
        {
            if (value < node.Value)
            {
                best = node.Value;
                node = node.Left;
            }
            else
            {
                node = node.Right;
            }
        }

        return best;
    }

    /// <summary>
    /// Return the 1-indexed kth smallest value.
    /// </summary>
    public int KthSmallest(int k)
    {
        var sorted = ToSortedList();
        if (k < 1 || k > sorted.Count)
        {
            throw new InvalidOperationException($"k={k} out of range; tree has {sorted.Count} elements");
        }

        return sorted[k - 1];
    }

    /// <summary>
    /// Return all values in ascending order.
    /// </summary>
    public IReadOnlyList<int> ToSortedList()
    {
        var result = new List<int>();
        InOrder(_root, result);
        return result;
    }

    /// <summary>
    /// Verify the red-black invariants.
    /// </summary>
    public bool IsValidRB()
    {
        if (_root is null)
        {
            return true;
        }

        return _root.Color == Color.Black && CheckNode(_root) != -1;
    }

    /// <summary>
    /// Return the root black-height, or 0 for an empty tree.
    /// </summary>
    public int BlackHeight() => BlackHeightHelper(_root);

    /// <summary>
    /// Return the number of values in the tree.
    /// </summary>
    public int Size() => SizeHelper(_root);

    /// <summary>
    /// Return the tree height, or 0 for an empty tree.
    /// </summary>
    public int Height() => HeightHelper(_root);

    /// <summary>
    /// True when the tree is empty.
    /// </summary>
    public bool IsEmpty => _root is null;

    /// <summary>
    /// Return the root node, or null when empty.
    /// </summary>
    public Node? Root => _root;

    /// <summary>
    /// Return the root node, or null when empty.
    /// </summary>
    public Node? GetRoot() => _root;

    /// <summary>
    /// Return tree metadata.
    /// </summary>
    public override string ToString() => $"RBTree{{size={Size()}, height={Height()}, blackHeight={BlackHeight()}}}";

    private static Node InsertHelper(Node? node, int value)
    {
        if (node is null)
        {
            return new Node(value, Color.Red);
        }

        if (value < node.Value)
        {
            return FixUp(node with { Left = InsertHelper(node.Left, value) });
        }

        if (value > node.Value)
        {
            return FixUp(node with { Right = InsertHelper(node.Right, value) });
        }

        return node;
    }

    private static Node? DeleteHelper(Node node, int value)
    {
        if (value < node.Value)
        {
            var current = node;
            if (!IsRed(current.Left) && !IsRed(current.Left?.Left))
            {
                current = MoveRedLeft(current);
            }

            var newLeft = current.Left is null ? null : DeleteHelper(current.Left, value);
            return FixUp(current with { Left = newLeft });
        }

        var n = node;
        if (IsRed(n.Left))
        {
            n = RotateRight(n);
        }

        if (value == n.Value && n.Right is null)
        {
            return null;
        }

        if (!IsRed(n.Right) && !IsRed(n.Right?.Left))
        {
            n = MoveRedRight(n);
        }

        if (value == n.Value)
        {
            var successor = MinValue(n.Right!);
            var newRight = DeleteMin(n.Right!);
            return FixUp(new Node(successor, n.Color, n.Left, newRight));
        }

        var right = n.Right is null ? null : DeleteHelper(n.Right, value);
        return FixUp(n with { Right = right });
    }

    private static Node? DeleteMin(Node node)
    {
        if (node.Left is null)
        {
            return null;
        }

        var current = node;
        if (!IsRed(current.Left) && !IsRed(current.Left?.Left))
        {
            current = MoveRedLeft(current);
        }

        return FixUp(current with { Left = DeleteMin(current.Left!) });
    }

    private static Node RotateLeft(Node node)
    {
        var right = node.Right!;
        return new Node(
            right.Value,
            node.Color,
            new Node(node.Value, Color.Red, node.Left, right.Left),
            right.Right);
    }

    private static Node RotateRight(Node node)
    {
        var left = node.Left!;
        return new Node(
            left.Value,
            node.Color,
            left.Left,
            new Node(node.Value, Color.Red, left.Right, node.Right));
    }

    private static Node FlipColors(Node node)
    {
        return new Node(
            node.Value,
            Toggle(node.Color),
            node.Left?.WithColor(Toggle(node.Left.Color)),
            node.Right?.WithColor(Toggle(node.Right.Color)));
    }

    private static Node FixUp(Node node)
    {
        var current = node;
        if (IsRed(current.Right) && !IsRed(current.Left))
        {
            current = RotateLeft(current);
        }

        if (IsRed(current.Left) && IsRed(current.Left?.Left))
        {
            current = RotateRight(current);
        }

        if (IsRed(current.Left) && IsRed(current.Right))
        {
            current = FlipColors(current);
        }

        return current;
    }

    private static Node MoveRedLeft(Node node)
    {
        var current = FlipColors(node);
        if (IsRed(current.Right?.Left))
        {
            current = current with { Right = RotateRight(current.Right!) };
            current = RotateLeft(current);
            current = FlipColors(current);
        }

        return current;
    }

    private static Node MoveRedRight(Node node)
    {
        var current = FlipColors(node);
        if (IsRed(current.Left?.Left))
        {
            current = RotateRight(current);
            current = FlipColors(current);
        }

        return current;
    }

    private static bool IsRed(Node? node) => node?.Color == Color.Red;

    private static Color Toggle(Color color) => color == Color.Red ? Color.Black : Color.Red;

    private static int MinValue(Node node)
    {
        var current = node;
        while (current.Left is not null)
        {
            current = current.Left;
        }

        return current.Value;
    }

    private static void InOrder(Node? node, List<int> values)
    {
        if (node is null)
        {
            return;
        }

        InOrder(node.Left, values);
        values.Add(node.Value);
        InOrder(node.Right, values);
    }

    private static int CheckNode(Node? node)
    {
        if (node is null)
        {
            return 1;
        }

        if (node.Color == Color.Red && (IsRed(node.Left) || IsRed(node.Right)))
        {
            return -1;
        }

        var leftBlackHeight = CheckNode(node.Left);
        var rightBlackHeight = CheckNode(node.Right);

        if (leftBlackHeight == -1 || rightBlackHeight == -1 || leftBlackHeight != rightBlackHeight)
        {
            return -1;
        }

        return leftBlackHeight + (node.Color == Color.Black ? 1 : 0);
    }

    private static int BlackHeightHelper(Node? node)
    {
        if (node is null)
        {
            return 0;
        }

        return BlackHeightHelper(node.Left) + (node.Color == Color.Black ? 1 : 0);
    }

    private static int SizeHelper(Node? node) =>
        node is null ? 0 : 1 + SizeHelper(node.Left) + SizeHelper(node.Right);

    private static int HeightHelper(Node? node) =>
        node is null ? 0 : 1 + Math.Max(HeightHelper(node.Left), HeightHelper(node.Right));
}
