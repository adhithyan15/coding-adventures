using System.Collections;

namespace CodingAdventures.HistoricalWeb;

/// <summary>The kind of item stored in a historical HTML arena.</summary>
public enum HtmlNodeKind
{
    Text,
    Element,
}

/// <summary>An immutable text or element record in the flat document arena.</summary>
public readonly record struct HtmlNode(
    HtmlNodeKind Kind,
    int ParentIndex,
    int FirstChildIndex,
    int NextSiblingIndex,
    string Tag,
    string Text,
    int FirstAttributeIndex,
    int AttributeCount);

/// <summary>A recognized attribute on an anchor; null means it had no value.</summary>
public readonly record struct HtmlAttribute(string Name, string? Value);

/// <summary>
/// An owned, immutable view of a document built by the November 1992 tag profile.
/// The arena uses indexes, so nested markup does not create recursive ownership.
/// </summary>
public sealed class HtmlDocument
{
    private readonly HtmlNode[] _nodes;
    private readonly HtmlAttribute[] _attributes;

    internal HtmlDocument(HtmlNode[] nodes, HtmlAttribute[] attributes, int rootCount)
    {
        _nodes = nodes;
        _attributes = attributes;
        Nodes = new ReadOnlyArrayView<HtmlNode>(_nodes);
        Attributes = new ReadOnlyArrayView<HtmlAttribute>(_attributes);
        RootCount = rootCount;
    }

    public IReadOnlyList<HtmlNode> Nodes { get; }

    public IReadOnlyList<HtmlAttribute> Attributes { get; }

    public int FirstRoot => _nodes.Length == 0 ? -1 : 0;

    public int RootCount { get; }

    public int NodeCount => _nodes.Length;

    public int AttributeCount => _attributes.Length;

    public HtmlNode NodeAt(int index)
    {
        if ((uint)index >= (uint)_nodes.Length)
        {
            throw new ArgumentOutOfRangeException(nameof(index));
        }

        return _nodes[index];
    }

    public HtmlAttribute AttributeAt(int index)
    {
        if ((uint)index >= (uint)_attributes.Length)
        {
            throw new ArgumentOutOfRangeException(nameof(index));
        }

        return _attributes[index];
    }

    /// <summary>
    /// A collection facade with only IReadOnlyList operations. Unlike general
    /// collection wrappers, it does not expose an underlying SyncRoot or array.
    /// </summary>
    private sealed class ReadOnlyArrayView<T>(T[] items) : IReadOnlyList<T>
    {
        public int Count => items.Length;

        public T this[int index] => items[index];

        public IEnumerator<T> GetEnumerator()
        {
            for (var i = 0; i < items.Length; i++)
            {
                yield return items[i];
            }
        }

        IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    }
}
