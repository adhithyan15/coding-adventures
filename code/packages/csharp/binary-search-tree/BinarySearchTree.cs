using System.Collections;

namespace CodingAdventures.BinarySearchTree;

public sealed class BstNode<T>
    where T : IComparable<T>
{
    public BstNode(T value, BstNode<T>? left = null, BstNode<T>? right = null, int? size = null)
    {
        Value = value;
        Left = left;
        Right = right;
        Size = size ?? 1 + BinarySearchTree<T>.NodeSize(left) + BinarySearchTree<T>.NodeSize(right);
    }

    public T Value { get; }

    public BstNode<T>? Left { get; }

    public BstNode<T>? Right { get; }

    public int Size { get; }
}

public sealed class BinarySearchTree<T> : IReadOnlyCollection<T>
    where T : IComparable<T>
{
    public BinarySearchTree(BstNode<T>? root = null)
    {
        Root = root;
    }

    public BstNode<T>? Root { get; }

    public int Count => Size();

    public static BinarySearchTree<T> Empty() => new();

    public static BinarySearchTree<T> FromSortedArray(IEnumerable<T> values)
    {
        ArgumentNullException.ThrowIfNull(values);
        var array = values.ToArray();
        return new BinarySearchTree<T>(BuildBalanced(array, 0, array.Length));
    }

    public BinarySearchTree<T> Insert(T value) => new(InsertNode(Root, value));

    public BinarySearchTree<T> Delete(T value) => new(DeleteNode(Root, value));

    public BstNode<T>? Search(T value) => SearchNode(Root, value);

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

    public bool IsValid() => Validate(Root, default, default, hasMinimum: false, hasMaximum: false) is not null;

    public int Height() => Height(Root);

    public int Size() => NodeSize(Root);

    public IEnumerator<T> GetEnumerator() => ToSortedArray().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString()
    {
        var root = Root is null ? "null" : Root.Value?.ToString();
        return $"BinarySearchTree(root={root}, size={Size()})";
    }

    public static BstNode<T>? SearchNode(BstNode<T>? root, T value)
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

    public static BstNode<T> InsertNode(BstNode<T>? root, T value)
    {
        if (root is null)
        {
            return new BstNode<T>(value);
        }

        var comparison = value.CompareTo(root.Value);
        if (comparison < 0)
        {
            return WithChildren(root, left: InsertNode(root.Left, value), replaceRight: false);
        }

        if (comparison > 0)
        {
            return WithChildren(root, right: InsertNode(root.Right, value), replaceLeft: false);
        }

        return root;
    }

    public static BstNode<T>? DeleteNode(BstNode<T>? root, T value)
    {
        if (root is null)
        {
            return null;
        }

        var comparison = value.CompareTo(root.Value);
        if (comparison < 0)
        {
            return WithChildren(root, left: DeleteNode(root.Left, value), replaceRight: false);
        }

        if (comparison > 0)
        {
            return WithChildren(root, right: DeleteNode(root.Right, value), replaceLeft: false);
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
        return new BstNode<T>(successor, root.Left, newRight);
    }

    public static T? MinValue(BstNode<T>? root)
    {
        var current = root;
        while (current?.Left is not null)
        {
            current = current.Left;
        }

        return current is null ? default : current.Value;
    }

    public static T? MaxValue(BstNode<T>? root)
    {
        var current = root;
        while (current?.Right is not null)
        {
            current = current.Right;
        }

        return current is null ? default : current.Value;
    }

    public static T? Predecessor(BstNode<T>? root, T value)
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

    public static T? Successor(BstNode<T>? root, T value)
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

    public static T? KthSmallest(BstNode<T>? root, int k)
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

    public static int Rank(BstNode<T>? root, T value)
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

    public static int Height(BstNode<T>? root)
    {
        return root is null ? -1 : 1 + Math.Max(Height(root.Left), Height(root.Right));
    }

    public static bool IsValid(BstNode<T>? root)
    {
        return Validate(root, default, default, hasMinimum: false, hasMaximum: false) is not null;
    }

    public static int NodeSize(BstNode<T>? root) => root?.Size ?? 0;

    private static void InOrder(BstNode<T>? root, List<T> output)
    {
        if (root is null)
        {
            return;
        }

        InOrder(root.Left, output);
        output.Add(root.Value);
        InOrder(root.Right, output);
    }

    private static BstNode<T> WithChildren(
        BstNode<T> root,
        BstNode<T>? left = null,
        BstNode<T>? right = null,
        bool replaceLeft = true,
        bool replaceRight = true)
    {
        return new BstNode<T>(
            root.Value,
            replaceLeft ? left : root.Left,
            replaceRight ? right : root.Right);
    }

    private static (BstNode<T>? Root, T Minimum) ExtractMin(BstNode<T> root)
    {
        if (root.Left is null)
        {
            return (root.Right, root.Value);
        }

        var (newLeft, minimum) = ExtractMin(root.Left);
        return (WithChildren(root, left: newLeft, right: root.Right), minimum);
    }

    private static BstNode<T>? BuildBalanced(T[] values, int start, int end)
    {
        if (start >= end)
        {
            return null;
        }

        var mid = start + (end - start) / 2;
        return new BstNode<T>(
            values[mid],
            BuildBalanced(values, start, mid),
            BuildBalanced(values, mid + 1, end));
    }

    private static (int Height, int Size)? Validate(
        BstNode<T>? root,
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

        var left = Validate(root.Left, minimum, root.Value, hasMinimum, hasMaximum: true);
        var right = Validate(root.Right, root.Value, maximum, hasMinimum: true, hasMaximum);
        if (left is null || right is null)
        {
            return null;
        }

        var height = 1 + Math.Max(left.Value.Height, right.Value.Height);
        var size = 1 + left.Value.Size + right.Value.Size;
        return root.Size == size ? (height, size) : null;
    }
}
