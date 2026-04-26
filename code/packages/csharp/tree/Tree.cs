using System.Text;

namespace CodingAdventures.Tree;

public enum TreeErrorKind
{
    NodeNotFound,
    DuplicateNode,
    RootRemoval,
}

public sealed class TreeException : InvalidOperationException
{
    public TreeException(TreeErrorKind kind, string? node)
        : base(CreateMessage(kind, node))
    {
        Kind = kind;
        Node = node;
    }

    public TreeErrorKind Kind { get; }

    public string? Node { get; }

    private static string CreateMessage(TreeErrorKind kind, string? node) =>
        kind switch
        {
            TreeErrorKind.NodeNotFound => $"Node not found in tree: {node}",
            TreeErrorKind.DuplicateNode => $"Node already exists in tree: {node}",
            TreeErrorKind.RootRemoval => "Cannot remove the root node.",
            _ => "Tree operation failed.",
        };
}

/// <summary>
/// A rooted tree with unique string node names.
/// </summary>
public sealed class Tree
{
    private readonly Dictionary<string, string?> _parents = new(StringComparer.Ordinal);
    private readonly Dictionary<string, SortedSet<string>> _children = new(StringComparer.Ordinal);

    public Tree(string root)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(root);
        Root = root;
        _parents[root] = null;
        _children[root] = [];
    }

    public string Root { get; }

    public int Size => _parents.Count;

    public Tree AddChild(string parent, string child)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(parent);
        ArgumentException.ThrowIfNullOrWhiteSpace(child);

        if (!HasNode(parent))
        {
            throw new TreeException(TreeErrorKind.NodeNotFound, parent);
        }

        if (HasNode(child))
        {
            throw new TreeException(TreeErrorKind.DuplicateNode, child);
        }

        _parents[child] = parent;
        _children[child] = [];
        _children[parent].Add(child);
        return this;
    }

    public Tree RemoveSubtree(string node)
    {
        EnsureNode(node);
        if (node == Root)
        {
            throw new TreeException(TreeErrorKind.RootRemoval, node);
        }

        var nodes = CollectSubtreeNodes(node);
        foreach (var current in nodes.AsEnumerable().Reverse())
        {
            foreach (var child in _children[current].ToArray())
            {
                _children[current].Remove(child);
            }

            if (_parents[current] is { } parent)
            {
                _children[parent].Remove(current);
            }

            _children.Remove(current);
            _parents.Remove(current);
        }

        return this;
    }

    public string? Parent(string node)
    {
        EnsureNode(node);
        return _parents[node];
    }

    public IReadOnlyList<string> Children(string node)
    {
        EnsureNode(node);
        return _children[node].ToArray();
    }

    public IReadOnlyList<string> Siblings(string node)
    {
        EnsureNode(node);
        var parent = _parents[node];
        return parent is null ? [] : _children[parent].Where(child => child != node).ToArray();
    }

    public bool IsLeaf(string node)
    {
        EnsureNode(node);
        return _children[node].Count == 0;
    }

    public bool IsRoot(string node)
    {
        EnsureNode(node);
        return node == Root;
    }

    public int Depth(string node)
    {
        EnsureNode(node);
        var depth = 0;
        var current = node;
        while (_parents[current] is { } parent)
        {
            depth++;
            current = parent;
        }

        return depth;
    }

    public int Height() => _parents.Keys.Select(Depth).DefaultIfEmpty(0).Max();

    public IReadOnlyList<string> Nodes() => _parents.Keys.Order(StringComparer.Ordinal).ToArray();

    public IReadOnlyList<string> Leaves() => _parents.Keys.Where(node => _children[node].Count == 0).Order(StringComparer.Ordinal).ToArray();

    public bool HasNode(string node) => _parents.ContainsKey(node);

    public IReadOnlyList<string> Preorder()
    {
        var result = new List<string>();
        VisitPreorder(Root, result);
        return result;
    }

    public IReadOnlyList<string> Postorder()
    {
        var result = new List<string>();
        VisitPostorder(Root, result);
        return result;
    }

    public IReadOnlyList<string> LevelOrder()
    {
        var result = new List<string>();
        var queue = new Queue<string>();
        queue.Enqueue(Root);
        while (queue.Count > 0)
        {
            var node = queue.Dequeue();
            result.Add(node);
            foreach (var child in _children[node])
            {
                queue.Enqueue(child);
            }
        }

        return result;
    }

    public IReadOnlyList<string> PathTo(string node)
    {
        EnsureNode(node);
        var path = new List<string>();
        var current = node;
        while (true)
        {
            path.Add(current);
            if (_parents[current] is not { } parent)
            {
                break;
            }

            current = parent;
        }

        path.Reverse();
        return path;
    }

    public string LowestCommonAncestor(string left, string right)
    {
        EnsureNode(left);
        EnsureNode(right);
        var leftPath = PathTo(left);
        var rightPath = PathTo(right);
        var limit = Math.Min(leftPath.Count, rightPath.Count);
        var ancestor = Root;
        for (var index = 0; index < limit && leftPath[index] == rightPath[index]; index++)
        {
            ancestor = leftPath[index];
        }

        return ancestor;
    }

    public string Lca(string left, string right) => LowestCommonAncestor(left, right);

    public Tree Subtree(string node)
    {
        EnsureNode(node);
        var tree = new Tree(node);
        CopyChildrenInto(node, tree);
        return tree;
    }

    public string ToAscii()
    {
        var builder = new StringBuilder();
        builder.AppendLine(Root);
        var children = _children[Root].ToArray();
        for (var index = 0; index < children.Length; index++)
        {
            AppendAscii(builder, children[index], string.Empty, index == children.Length - 1);
        }

        return builder.ToString().TrimEnd();
    }

    public override string ToString() => $"Tree(root={Root}, size={Size}, height={Height()})";

    private void EnsureNode(string node)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(node);
        if (!HasNode(node))
        {
            throw new TreeException(TreeErrorKind.NodeNotFound, node);
        }
    }

    private List<string> CollectSubtreeNodes(string node)
    {
        var result = new List<string>();
        var queue = new Queue<string>();
        queue.Enqueue(node);
        while (queue.Count > 0)
        {
            var current = queue.Dequeue();
            result.Add(current);
            foreach (var child in _children[current])
            {
                queue.Enqueue(child);
            }
        }

        return result;
    }

    private void VisitPreorder(string node, List<string> result)
    {
        result.Add(node);
        foreach (var child in _children[node])
        {
            VisitPreorder(child, result);
        }
    }

    private void VisitPostorder(string node, List<string> result)
    {
        foreach (var child in _children[node])
        {
            VisitPostorder(child, result);
        }

        result.Add(node);
    }

    private void CopyChildrenInto(string node, Tree target)
    {
        foreach (var child in _children[node])
        {
            target.AddChild(node, child);
            CopyChildrenInto(child, target);
        }
    }

    private void AppendAscii(StringBuilder builder, string node, string prefix, bool isLast)
    {
        builder.Append(prefix);
        builder.Append(isLast ? "`-- " : "|-- ");
        builder.AppendLine(node);

        var children = _children[node].ToArray();
        for (var index = 0; index < children.Length; index++)
        {
            AppendAscii(builder, children[index], prefix + (isLast ? "    " : "|   "), index == children.Length - 1);
        }
    }
}
