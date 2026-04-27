using System.Text;

namespace CodingAdventures.Rope;

/// <summary>
/// Immutable rope for efficient text composition and slicing.
/// </summary>
public sealed class Rope
{
    private readonly RopeNode? _root;

    private Rope(RopeNode? root, int length)
    {
        _root = root;
        Length = length;
    }

    public int Length { get; }

    public int Count => Length;

    public bool IsEmpty => Length == 0;

    public static Rope Empty() => new(null, 0);

    public static Rope FromString(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        return text.Length == 0 ? Empty() : new Rope(new LeafNode(text), text.Length);
    }

    public static Rope RopeFromString(string text) => FromString(text);

    public static Rope Concat(Rope left, Rope right)
    {
        ArgumentNullException.ThrowIfNull(left);
        ArgumentNullException.ThrowIfNull(right);

        if (left._root is null)
        {
            return right;
        }

        if (right._root is null)
        {
            return left;
        }

        return new Rope(new InternalNode(left.Length, left._root, right._root), left.Length + right.Length);
    }

    public Rope Concat(Rope right) => Concat(this, right);

    public (Rope Left, Rope Right) Split(int index)
    {
        var chars = ToString().ToCharArray();
        var splitAt = Math.Clamp(index, 0, chars.Length);
        return (
            FromString(new string(chars[..splitAt])),
            FromString(new string(chars[splitAt..])));
    }

    public Rope Insert(int index, string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        var (left, right) = Split(index);
        return Concat(Concat(left, FromString(text)), right);
    }

    public Rope Delete(int start, int length)
    {
        if (length < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(length), "Length must be non-negative.");
        }

        var chars = ToString().ToCharArray();
        var safeStart = Math.Clamp(start, 0, chars.Length);
        var end = Math.Min(safeStart + length, chars.Length);
        return Concat(
            FromString(new string(chars[..safeStart])),
            FromString(new string(chars[end..])));
    }

    public char? Index(int index)
    {
        var text = ToString();
        return index >= 0 && index < text.Length ? text[index] : null;
    }

    public string Substring(int start, int end)
    {
        var chars = ToString().ToCharArray();
        var safeStart = Math.Clamp(start, 0, chars.Length);
        var safeEnd = Math.Clamp(end, 0, chars.Length);
        return safeStart >= safeEnd ? string.Empty : new string(chars[safeStart..safeEnd]);
    }

    public int Depth() => _root?.Depth() ?? 0;

    public bool IsBalanced() => _root?.IsBalanced() ?? true;

    public Rope Rebalance() => BuildBalanced(ToString());

    public override string ToString()
    {
        var builder = new StringBuilder(Length);
        _root?.AppendTo(builder);
        return builder.ToString();
    }

    private static Rope BuildBalanced(string text)
    {
        if (text.Length == 0)
        {
            return Empty();
        }

        if (text.Length == 1)
        {
            return FromString(text);
        }

        var midpoint = text.Length / 2;
        return Concat(BuildBalanced(text[..midpoint]), BuildBalanced(text[midpoint..]));
    }

    private abstract class RopeNode
    {
        public abstract int Depth();

        public abstract bool IsBalanced();

        public abstract void AppendTo(StringBuilder builder);
    }

    private sealed class LeafNode(string chunk) : RopeNode
    {
        public override int Depth() => 0;

        public override bool IsBalanced() => true;

        public override void AppendTo(StringBuilder builder) => builder.Append(chunk);
    }

    private sealed class InternalNode(int weight, RopeNode left, RopeNode right) : RopeNode
    {
        public int Weight => weight;

        public override int Depth() => 1 + Math.Max(left.Depth(), right.Depth());

        public override bool IsBalanced()
        {
            var leftDepth = left.Depth();
            var rightDepth = right.Depth();
            return Math.Abs(leftDepth - rightDepth) <= 1 && left.IsBalanced() && right.IsBalanced();
        }

        public override void AppendTo(StringBuilder builder)
        {
            left.AppendTo(builder);
            right.AppendTo(builder);
        }
    }
}
