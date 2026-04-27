using System.Collections.Generic;
using System.Text;

namespace CodingAdventures.BinaryTree;

/// <summary>A node in a binary tree.</summary>
public sealed class BinaryTreeNode<T>
{
    public BinaryTreeNode(T value, BinaryTreeNode<T>? left = null, BinaryTreeNode<T>? right = null)
    {
        Value = value;
        Left = left;
        Right = right;
    }

    public T Value { get; }

    public BinaryTreeNode<T>? Left { get; }

    public BinaryTreeNode<T>? Right { get; }
}

/// <summary>A generic binary tree with traversal and shape helpers.</summary>
public sealed class BinaryTree<T>
{
    private readonly IEqualityComparer<T> _comparer;

    public BinaryTree(BinaryTreeNode<T>? root = null, IEqualityComparer<T>? comparer = null)
    {
        Root = root;
        _comparer = comparer ?? EqualityComparer<T>.Default;
    }

    public BinaryTreeNode<T>? Root { get; }

    public static BinaryTree<T> WithRoot(BinaryTreeNode<T>? root) => new(root);

    public static BinaryTree<T> Singleton(T value) => new(new BinaryTreeNode<T>(value));

    public static BinaryTree<T> FromLevelOrder(IReadOnlyList<T?> values)
    {
        return new BinaryTree<T>(BuildFromLevelOrder(values, 0));
    }

    public BinaryTreeNode<T>? Find(T value) => Find(Root, value, _comparer);

    public BinaryTreeNode<T>? LeftChild(T value) => Find(value)?.Left;

    public BinaryTreeNode<T>? RightChild(T value) => Find(value)?.Right;

    public bool IsFull() => IsFull(Root);

    public bool IsComplete() => IsComplete(Root);

    public bool IsPerfect() => IsPerfect(Root);

    public int Height() => Height(Root);

    public int Size() => Size(Root);

    public IReadOnlyList<T> Inorder()
    {
        var values = new List<T>();
        Inorder(Root, values);
        return values;
    }

    public IReadOnlyList<T> Preorder()
    {
        var values = new List<T>();
        Preorder(Root, values);
        return values;
    }

    public IReadOnlyList<T> Postorder()
    {
        var values = new List<T>();
        Postorder(Root, values);
        return values;
    }

    public IReadOnlyList<T> LevelOrder()
    {
        if (Root is null)
        {
            return [];
        }

        var values = new List<T>();
        var queue = new Queue<BinaryTreeNode<T>>();
        queue.Enqueue(Root);
        while (queue.Count > 0)
        {
            var node = queue.Dequeue();
            values.Add(node.Value);
            if (node.Left is not null)
            {
                queue.Enqueue(node.Left);
            }
            if (node.Right is not null)
            {
                queue.Enqueue(node.Right);
            }
        }

        return values;
    }

    public IReadOnlyList<T?> ToArray()
    {
        var height = Height();
        if (height < 0)
        {
            return [];
        }

        var values = new T?[(1 << (height + 1)) - 1];
        FillArray(Root, 0, values);
        return values;
    }

    public string ToAscii()
    {
        if (Root is null)
        {
            return string.Empty;
        }

        var builder = new StringBuilder();
        RenderAscii(Root, string.Empty, true, builder);
        return builder.ToString().TrimEnd('\r', '\n');
    }

    public override string ToString()
    {
        return $"BinaryTree(root={Root?.Value?.ToString() ?? "null"}, size={Size()})";
    }

    public static BinaryTreeNode<T>? Find(BinaryTreeNode<T>? root, T value, IEqualityComparer<T>? comparer = null)
    {
        comparer ??= EqualityComparer<T>.Default;
        if (root is null)
        {
            return null;
        }
        if (comparer.Equals(root.Value, value))
        {
            return root;
        }
        return Find(root.Left, value, comparer) ?? Find(root.Right, value, comparer);
    }

    public static bool IsFull(BinaryTreeNode<T>? root)
    {
        if (root is null)
        {
            return true;
        }
        if (root.Left is null && root.Right is null)
        {
            return true;
        }
        if (root.Left is null || root.Right is null)
        {
            return false;
        }
        return IsFull(root.Left) && IsFull(root.Right);
    }

    public static bool IsComplete(BinaryTreeNode<T>? root)
    {
        var queue = new Queue<BinaryTreeNode<T>?>();
        queue.Enqueue(root);
        var seenNull = false;
        while (queue.Count > 0)
        {
            var node = queue.Dequeue();
            if (node is null)
            {
                seenNull = true;
                continue;
            }
            if (seenNull)
            {
                return false;
            }
            queue.Enqueue(node.Left);
            queue.Enqueue(node.Right);
        }

        return true;
    }

    public static bool IsPerfect(BinaryTreeNode<T>? root)
    {
        var height = Height(root);
        return height < 0 ? Size(root) == 0 : Size(root) == (1 << (height + 1)) - 1;
    }

    public static int Height(BinaryTreeNode<T>? root)
    {
        return root is null ? -1 : 1 + Math.Max(Height(root.Left), Height(root.Right));
    }

    public static int Size(BinaryTreeNode<T>? root)
    {
        return root is null ? 0 : 1 + Size(root.Left) + Size(root.Right);
    }

    private static BinaryTreeNode<T>? BuildFromLevelOrder(IReadOnlyList<T?> values, int index)
    {
        if (index >= values.Count || values[index] is null)
        {
            return null;
        }

        return new BinaryTreeNode<T>(
            values[index]!,
            BuildFromLevelOrder(values, (2 * index) + 1),
            BuildFromLevelOrder(values, (2 * index) + 2));
    }

    private static void Inorder(BinaryTreeNode<T>? root, List<T> values)
    {
        if (root is null)
        {
            return;
        }
        Inorder(root.Left, values);
        values.Add(root.Value);
        Inorder(root.Right, values);
    }

    private static void Preorder(BinaryTreeNode<T>? root, List<T> values)
    {
        if (root is null)
        {
            return;
        }
        values.Add(root.Value);
        Preorder(root.Left, values);
        Preorder(root.Right, values);
    }

    private static void Postorder(BinaryTreeNode<T>? root, List<T> values)
    {
        if (root is null)
        {
            return;
        }
        Postorder(root.Left, values);
        Postorder(root.Right, values);
        values.Add(root.Value);
    }

    private static void FillArray(BinaryTreeNode<T>? root, int index, T?[] values)
    {
        if (root is null || index >= values.Length)
        {
            return;
        }
        values[index] = root.Value;
        FillArray(root.Left, (2 * index) + 1, values);
        FillArray(root.Right, (2 * index) + 2, values);
    }

    private static void RenderAscii(BinaryTreeNode<T> node, string prefix, bool isTail, StringBuilder builder)
    {
        builder.Append(prefix);
        builder.Append(isTail ? "`-- " : "|-- ");
        builder.AppendLine(node.Value?.ToString());

        var children = new List<BinaryTreeNode<T>>();
        if (node.Left is not null)
        {
            children.Add(node.Left);
        }
        if (node.Right is not null)
        {
            children.Add(node.Right);
        }

        var nextPrefix = prefix + (isTail ? "    " : "|   ");
        for (var index = 0; index < children.Count; index++)
        {
            RenderAscii(children[index], nextPrefix, index + 1 == children.Count, builder);
        }
    }
}
