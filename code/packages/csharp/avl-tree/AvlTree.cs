using System.Collections;

namespace CodingAdventures.AvlTree;

public sealed class AvlNode<T>
    where T : IComparable<T>
{
    public AvlNode(
        T value,
        AvlNode<T>? left = null,
        AvlNode<T>? right = null,
        int? height = null,
        int? size = null)
    {
        Value = value;
        Left = left;
        Right = right;
        Height = height ?? 1 + Math.Max(AvlTree<T>.NodeHeight(left), AvlTree<T>.NodeHeight(right));
        Size = size ?? 1 + AvlTree<T>.NodeSize(left) + AvlTree<T>.NodeSize(right);
    }

    public T Value { get; }

    public AvlNode<T>? Left { get; }

    public AvlNode<T>? Right { get; }

    public int Height { get; }

    public int Size { get; }
}

public sealed class AvlTree<T> : IReadOnlyCollection<T>
    where T : IComparable<T>
{
    public AvlTree(AvlNode<T>? root = null)
    {
        Root = root;
    }

    public AvlNode<T>? Root { get; }

    public int Count => Size();

    public static AvlTree<T> Empty() => new();

    public static AvlTree<T> FromValues(IEnumerable<T> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        var tree = Empty();
        foreach (var value in values)
        {
            tree = tree.Insert(value);
        }

        return tree;
    }

    public AvlTree<T> Insert(T value) => new(InsertNode(Root, value));

    public AvlTree<T> Delete(T value) => new(DeleteNode(Root, value));

    public AvlNode<T>? Search(T value) => SearchNode(Root, value);

    public bool Contains(T value) => Search(value) is not null;

    public T? MinValue() => MinValue(Root);

    public T? MaxValue() => MaxValue(Root);

    public T? Predecessor(T value) => Predecessor(Root, value);

    public T? Successor(T value) => Successor(Root, value);

    public T? KthSmallest(int k) => KthSmallest(Root, k);

    public int Rank(T value) => Rank(Root, value);

    public List<T> ToSortedArray()
    {
        var output = new List<T>();
        InOrder(Root, output);
        return output;
    }

    public bool IsValidBst() => IsValidBst(Root);

    public bool IsValidAvl() => ValidateAvl(Root, default, default, hasMinimum: false, hasMaximum: false) is not null;

    public int BalanceFactor(AvlNode<T>? node) => BalanceFactorNode(node);

    public int Height() => NodeHeight(Root);

    public int Size() => NodeSize(Root);

    public IEnumerator<T> GetEnumerator() => ToSortedArray().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString()
    {
        var root = Root is null ? "null" : Root.Value?.ToString();
        return $"AvlTree(root={root}, size={Size()}, height={Height()})";
    }

    public static AvlNode<T>? SearchNode(AvlNode<T>? root, T value)
    {
        var current = root;
        while (current is not null)
        {
            var comparison = value.CompareTo(current.Value);
            if (comparison < 0)
            {
                current = current.Left;
            }
            else if (comparison > 0)
            {
                current = current.Right;
            }
            else
            {
                return current;
            }
        }

        return null;
    }

    public static AvlNode<T> InsertNode(AvlNode<T>? root, T value)
    {
        if (root is null)
        {
            return new AvlNode<T>(value);
        }

        var comparison = value.CompareTo(root.Value);
        if (comparison < 0)
        {
            return Rebalance(new AvlNode<T>(root.Value, InsertNode(root.Left, value), root.Right));
        }

        if (comparison > 0)
        {
            return Rebalance(new AvlNode<T>(root.Value, root.Left, InsertNode(root.Right, value)));
        }

        return root;
    }

    public static AvlNode<T>? DeleteNode(AvlNode<T>? root, T value)
    {
        if (root is null)
        {
            return null;
        }

        var comparison = value.CompareTo(root.Value);
        if (comparison < 0)
        {
            return Rebalance(new AvlNode<T>(root.Value, DeleteNode(root.Left, value), root.Right));
        }

        if (comparison > 0)
        {
            return Rebalance(new AvlNode<T>(root.Value, root.Left, DeleteNode(root.Right, value)));
        }

        if (root.Left is null)
        {
            return root.Right;
        }

        if (root.Right is null)
        {
            return root.Left;
        }

        var (newRight, successor) = ExtractMin(root.Right);
        return Rebalance(new AvlNode<T>(successor, root.Left, newRight));
    }

    public static T? MinValue(AvlNode<T>? root)
    {
        var current = root;
        while (current?.Left is not null)
        {
            current = current.Left;
        }

        return current is null ? default : current.Value;
    }

    public static T? MaxValue(AvlNode<T>? root)
    {
        var current = root;
        while (current?.Right is not null)
        {
            current = current.Right;
        }

        return current is null ? default : current.Value;
    }

    public static T? Predecessor(AvlNode<T>? root, T value)
    {
        var current = root;
        var best = default(T);
        var hasBest = false;

        while (current is not null)
        {
            if (value.CompareTo(current.Value) <= 0)
            {
                current = current.Left;
            }
            else
            {
                best = current.Value;
                hasBest = true;
                current = current.Right;
            }
        }

        return hasBest ? best : default;
    }

    public static T? Successor(AvlNode<T>? root, T value)
    {
        var current = root;
        var best = default(T);
        var hasBest = false;

        while (current is not null)
        {
            if (value.CompareTo(current.Value) >= 0)
            {
                current = current.Right;
            }
            else
            {
                best = current.Value;
                hasBest = true;
                current = current.Left;
            }
        }

        return hasBest ? best : default;
    }

    public static T? KthSmallest(AvlNode<T>? root, int k)
    {
        if (root is null || k <= 0)
        {
            return default;
        }

        var leftSize = NodeSize(root.Left);
        if (k == leftSize + 1)
        {
            return root.Value;
        }

        return k <= leftSize
            ? KthSmallest(root.Left, k)
            : KthSmallest(root.Right, k - leftSize - 1);
    }

    public static int Rank(AvlNode<T>? root, T value)
    {
        if (root is null)
        {
            return 0;
        }

        var comparison = value.CompareTo(root.Value);
        if (comparison < 0)
        {
            return Rank(root.Left, value);
        }

        if (comparison > 0)
        {
            return NodeSize(root.Left) + 1 + Rank(root.Right, value);
        }

        return NodeSize(root.Left);
    }

    public static bool IsValidBst(AvlNode<T>? root)
    {
        return ValidateBst(root, default, default, hasMinimum: false, hasMaximum: false);
    }

    public static bool IsValidAvl(AvlNode<T>? root)
    {
        return ValidateAvl(root, default, default, hasMinimum: false, hasMaximum: false) is not null;
    }

    public static int BalanceFactorNode(AvlNode<T>? node)
    {
        return node is null ? 0 : NodeHeight(node.Left) - NodeHeight(node.Right);
    }

    public static int NodeHeight(AvlNode<T>? root) => root?.Height ?? -1;

    public static int NodeSize(AvlNode<T>? root) => root?.Size ?? 0;

    private static AvlNode<T> Rebalance(AvlNode<T> node)
    {
        var balance = BalanceFactorNode(node);

        if (balance > 1)
        {
            var left = node.Left;
            if (left is not null && BalanceFactorNode(left) < 0)
            {
                left = RotateLeft(left);
            }

            return RotateRight(new AvlNode<T>(node.Value, left, node.Right));
        }

        if (balance < -1)
        {
            var right = node.Right;
            if (right is not null && BalanceFactorNode(right) > 0)
            {
                right = RotateRight(right);
            }

            return RotateLeft(new AvlNode<T>(node.Value, node.Left, right));
        }

        return node;
    }

    private static AvlNode<T> RotateLeft(AvlNode<T> root)
    {
        var right = root.Right;
        if (right is null)
        {
            return root;
        }

        var newLeft = new AvlNode<T>(root.Value, root.Left, right.Left);
        return new AvlNode<T>(right.Value, newLeft, right.Right);
    }

    private static AvlNode<T> RotateRight(AvlNode<T> root)
    {
        var left = root.Left;
        if (left is null)
        {
            return root;
        }

        var newRight = new AvlNode<T>(root.Value, left.Right, root.Right);
        return new AvlNode<T>(left.Value, left.Left, newRight);
    }

    private static (AvlNode<T>? Root, T Minimum) ExtractMin(AvlNode<T> root)
    {
        if (root.Left is null)
        {
            return (root.Right, root.Value);
        }

        var (newLeft, minimum) = ExtractMin(root.Left);
        return (Rebalance(new AvlNode<T>(root.Value, newLeft, root.Right)), minimum);
    }

    private static void InOrder(AvlNode<T>? root, List<T> output)
    {
        if (root is null)
        {
            return;
        }

        InOrder(root.Left, output);
        output.Add(root.Value);
        InOrder(root.Right, output);
    }

    private static bool ValidateBst(
        AvlNode<T>? root,
        T? minimum,
        T? maximum,
        bool hasMinimum,
        bool hasMaximum)
    {
        if (root is null)
        {
            return true;
        }

        if (hasMinimum && root.Value.CompareTo(minimum!) <= 0)
        {
            return false;
        }

        if (hasMaximum && root.Value.CompareTo(maximum!) >= 0)
        {
            return false;
        }

        return ValidateBst(root.Left, minimum, root.Value, hasMinimum, hasMaximum: true)
            && ValidateBst(root.Right, root.Value, maximum, hasMinimum: true, hasMaximum);
    }

    private static (int Height, int Size)? ValidateAvl(
        AvlNode<T>? root,
        T? minimum,
        T? maximum,
        bool hasMinimum,
        bool hasMaximum)
    {
        if (root is null)
        {
            return (-1, 0);
        }

        if (hasMinimum && root.Value.CompareTo(minimum!) <= 0)
        {
            return null;
        }

        if (hasMaximum && root.Value.CompareTo(maximum!) >= 0)
        {
            return null;
        }

        var left = ValidateAvl(root.Left, minimum, root.Value, hasMinimum, hasMaximum: true);
        var right = ValidateAvl(root.Right, root.Value, maximum, hasMinimum: true, hasMaximum);
        if (left is null || right is null)
        {
            return null;
        }

        var height = 1 + Math.Max(left.Value.Height, right.Value.Height);
        var size = 1 + left.Value.Size + right.Value.Size;
        if (root.Height != height || root.Size != size || Math.Abs(left.Value.Height - right.Value.Height) > 1)
        {
            return null;
        }

        return (height, size);
    }
}
