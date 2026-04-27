using System.Text;
using CodingAdventures.Heap;

namespace CodingAdventures.HuffmanTree;

public abstract record HuffmanNode(int Weight);

public sealed record HuffmanLeaf(int Symbol, int Weight) : HuffmanNode(Weight);

public sealed record HuffmanInternalNode(int Weight, HuffmanNode Left, HuffmanNode Right, int CreationOrder)
    : HuffmanNode(Weight);

internal readonly record struct NodePriority(int Weight, int NodeKindRank, int SymbolOrMax, int OrderOrMax);

internal readonly record struct HeapEntry(NodePriority Priority, HuffmanNode Node);

public sealed class HuffmanTree
{
    private readonly HuffmanNode _root;
    private readonly int _symbolCount;

    private HuffmanTree(HuffmanNode root, int symbolCount)
    {
        _root = root;
        _symbolCount = symbolCount;
    }

    public static HuffmanTree Build(IEnumerable<(int Symbol, int Frequency)> weights)
    {
        ArgumentNullException.ThrowIfNull(weights);

        var items = weights.ToList();
        if (items.Count == 0)
        {
            throw new ArgumentException("weights must not be empty", nameof(weights));
        }

        foreach (var (symbol, frequency) in items)
        {
            if (symbol < 0)
            {
                throw new ArgumentException($"symbol must be non-negative; got symbol={symbol}", nameof(weights));
            }

            if (frequency <= 0)
            {
                throw new ArgumentException(
                    $"frequency must be positive; got symbol={symbol}, freq={frequency}",
                    nameof(weights));
            }
        }

        var heap = new MinHeap<HeapEntry>((left, right) => ComparePriority(left.Priority, right.Priority));
        foreach (var (symbol, frequency) in items)
        {
            var leaf = new HuffmanLeaf(symbol, frequency);
            heap.Push(new HeapEntry(PriorityFor(leaf), leaf));
        }

        var creationOrder = 0;
        while (heap.Size > 1)
        {
            var left = heap.Pop().Node;
            var right = heap.Pop().Node;
            var internalNode = new HuffmanInternalNode(
                left.Weight + right.Weight,
                left,
                right,
                creationOrder);
            creationOrder++;
            heap.Push(new HeapEntry(PriorityFor(internalNode), internalNode));
        }

        return new HuffmanTree(heap.Pop().Node, items.Count);
    }

    public Dictionary<int, string> CodeTable()
    {
        var table = new Dictionary<int, string>();
        WalkCodes(_root, string.Empty, table);
        return table;
    }

    public string? CodeFor(int symbol) => FindCode(_root, symbol, string.Empty);

    public Dictionary<int, string> CanonicalCodeTable()
    {
        var lengths = new Dictionary<int, int>();
        CollectLengths(_root, 0, lengths);
        if (lengths.Count == 1)
        {
            return new Dictionary<int, string> { [lengths.Keys.Single()] = "0" };
        }

        var ordered = lengths
            .Select(pair => (Symbol: pair.Key, Length: pair.Value))
            .OrderBy(pair => pair.Length)
            .ThenBy(pair => pair.Symbol)
            .ToList();

        var codes = new Dictionary<int, string>();
        var codeValue = 0;
        var previousLength = ordered[0].Length;
        foreach (var (symbol, length) in ordered)
        {
            if (length > previousLength)
            {
                codeValue <<= length - previousLength;
            }

            codes[symbol] = Convert.ToString(codeValue, 2).PadLeft(length, '0');
            codeValue++;
            previousLength = length;
        }

        return codes;
    }

    public List<int> DecodeAll(string bits, int count)
    {
        ArgumentNullException.ThrowIfNull(bits);
        if (count < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(count), "count must be non-negative");
        }

        var output = new List<int>(count);
        var index = 0;
        var node = _root;
        var singleLeaf = _root is HuffmanLeaf;

        while (output.Count < count)
        {
            if (node is HuffmanLeaf leaf)
            {
                output.Add(leaf.Symbol);
                node = _root;
                if (singleLeaf && index < bits.Length)
                {
                    index++;
                }

                continue;
            }

            if (index >= bits.Length)
            {
                throw new InvalidOperationException(
                    $"Bit stream exhausted after {output.Count} symbols; expected {count}");
            }

            var bit = bits[index];
            index++;
            node = bit switch
            {
                '0' => ((HuffmanInternalNode)node).Left,
                '1' => ((HuffmanInternalNode)node).Right,
                _ => throw new InvalidOperationException("Bit stream must contain only '0' and '1'")
            };
        }

        return output;
    }

    public int Weight() => _root.Weight;

    public int Depth() => MaxDepth(_root, 0);

    public int SymbolCount() => _symbolCount;

    public List<(int Symbol, string Code)> Leaves()
    {
        var table = CodeTable();
        var leaves = new List<(int Symbol, string Code)>();
        CollectLeaves(_root, table, leaves);
        return leaves;
    }

    public bool IsValid()
    {
        var seen = new HashSet<int>();
        return Validate(_root, seen);
    }

    private static NodePriority PriorityFor(HuffmanNode node) =>
        node switch
        {
            HuffmanLeaf leaf => new NodePriority(leaf.Weight, 0, leaf.Symbol, int.MaxValue),
            HuffmanInternalNode internalNode => new NodePriority(
                internalNode.Weight,
                1,
                int.MaxValue,
                internalNode.CreationOrder),
            _ => throw new InvalidOperationException("Unknown Huffman node type")
        };

    private static int ComparePriority(NodePriority left, NodePriority right)
    {
        if (left.Weight != right.Weight)
        {
            return left.Weight.CompareTo(right.Weight);
        }

        if (left.NodeKindRank != right.NodeKindRank)
        {
            return left.NodeKindRank.CompareTo(right.NodeKindRank);
        }

        if (left.SymbolOrMax != right.SymbolOrMax)
        {
            return left.SymbolOrMax.CompareTo(right.SymbolOrMax);
        }

        return left.OrderOrMax.CompareTo(right.OrderOrMax);
    }

    private static void WalkCodes(HuffmanNode node, string prefix, IDictionary<int, string> table)
    {
        if (node is HuffmanLeaf leaf)
        {
            table[leaf.Symbol] = prefix.Length == 0 ? "0" : prefix;
            return;
        }

        var internalNode = (HuffmanInternalNode)node;
        WalkCodes(internalNode.Left, $"{prefix}0", table);
        WalkCodes(internalNode.Right, $"{prefix}1", table);
    }

    private static string? FindCode(HuffmanNode node, int symbol, string prefix)
    {
        if (node is HuffmanLeaf leaf)
        {
            return leaf.Symbol == symbol ? (prefix.Length == 0 ? "0" : prefix) : null;
        }

        var internalNode = (HuffmanInternalNode)node;
        return FindCode(internalNode.Left, symbol, $"{prefix}0")
            ?? FindCode(internalNode.Right, symbol, $"{prefix}1");
    }

    private static void CollectLengths(HuffmanNode node, int depth, IDictionary<int, int> lengths)
    {
        if (node is HuffmanLeaf leaf)
        {
            lengths[leaf.Symbol] = depth == 0 ? 1 : depth;
            return;
        }

        var internalNode = (HuffmanInternalNode)node;
        CollectLengths(internalNode.Left, depth + 1, lengths);
        CollectLengths(internalNode.Right, depth + 1, lengths);
    }

    private static int MaxDepth(HuffmanNode node, int depth) =>
        node switch
        {
            HuffmanLeaf => depth,
            HuffmanInternalNode internalNode => Math.Max(
                MaxDepth(internalNode.Left, depth + 1),
                MaxDepth(internalNode.Right, depth + 1)),
            _ => throw new InvalidOperationException("Unknown Huffman node type")
        };

    private static void CollectLeaves(
        HuffmanNode node,
        IReadOnlyDictionary<int, string> table,
        ICollection<(int Symbol, string Code)> output)
    {
        if (node is HuffmanLeaf leaf)
        {
            output.Add((leaf.Symbol, table[leaf.Symbol]));
            return;
        }

        var internalNode = (HuffmanInternalNode)node;
        CollectLeaves(internalNode.Left, table, output);
        CollectLeaves(internalNode.Right, table, output);
    }

    private static bool Validate(HuffmanNode node, ISet<int> seen)
    {
        if (node is HuffmanLeaf leaf)
        {
            return seen.Add(leaf.Symbol);
        }

        var internalNode = (HuffmanInternalNode)node;
        return internalNode.Weight == internalNode.Left.Weight + internalNode.Right.Weight
            && Validate(internalNode.Left, seen)
            && Validate(internalNode.Right, seen);
    }
}
