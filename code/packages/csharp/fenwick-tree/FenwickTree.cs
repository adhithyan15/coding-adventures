using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace CodingAdventures.FenwickTree;

// FenwickTree.cs -- Prefix sums stored in a tree hidden inside an array
// =====================================================================
//
// A Fenwick tree, also called a Binary Indexed Tree, answers prefix-sum
// queries while still supporting point updates:
//
//   update(i, delta)     -- add delta to one element
//   prefixSum(i)         -- sum elements 1..i
//
// Both operations run in O(log n) time. The trick is that each array slot
// stores the total for a power-of-two-sized range, and the least-significant
// set bit of the index tells us the width of that range.
//
// Example:
//
//   index 12 = 1100b
//   lowbit(12) = 0100b = 4
//
// So tree[12] stores the sum of the last 4 values that end at 12.
// Following parent links by adding lowbit(index) walks upward to larger ranges.

/// <summary>
/// Base class for Fenwick-tree specific errors.
/// </summary>
public class FenwickError : Exception
{
    /// <summary>
    /// Create a new Fenwick-specific error.
    /// </summary>
    public FenwickError(string message)
        : base(message)
    {
    }
}

/// <summary>
/// Raised when an operation uses an index outside the legal range.
/// </summary>
public sealed class FenwickIndexOutOfRangeError : FenwickError
{
    /// <summary>
    /// Create an index-range error.
    /// </summary>
    public FenwickIndexOutOfRangeError(string message)
        : base(message)
    {
    }
}

/// <summary>
/// Raised when <see cref="FenwickTree.FindKth"/> is called on an empty tree.
/// </summary>
public sealed class FenwickEmptyTreeError : FenwickError
{
    /// <summary>
    /// Create an empty-tree error.
    /// </summary>
    public FenwickEmptyTreeError(string message)
        : base(message)
    {
    }
}

/// <summary>
/// Binary Indexed Tree for prefix sums with point updates.
/// </summary>
public sealed class FenwickTree
{
    private readonly int _length;
    private readonly double[] _bit;

    /// <summary>
    /// Create a zero-filled tree of the requested size.
    /// </summary>
    public FenwickTree(int length)
    {
        if (length < 0)
        {
            throw new FenwickError($"Size must be a non-negative integer, got {length}");
        }

        _length = length;
        _bit = new double[length + 1];
    }

    /// <summary>
    /// Build a tree from a list of values in O(n).
    /// </summary>
    public static FenwickTree FromList(IEnumerable<double> values)
    {
        ArgumentNullException.ThrowIfNull(values);

        var data = values.ToArray();
        var tree = new FenwickTree(data.Length);

        for (var index = 1; index <= tree._length; index++)
        {
            tree._bit[index] += data[index - 1];
            var parent = index + LowBit(index);
            if (parent <= tree._length)
            {
                tree._bit[parent] += tree._bit[index];
            }
        }

        return tree;
    }

    /// <summary>
    /// Number of values stored in the logical array.
    /// </summary>
    public int Length => _length;

    /// <summary>
    /// Whether the tree has zero elements.
    /// </summary>
    public bool IsEmpty => _length == 0;

    /// <summary>
    /// Add <paramref name="delta"/> to the value at <paramref name="index"/>.
    /// </summary>
    public void Update(int index, double delta)
    {
        CheckIndex(index);

        var current = index;
        while (current <= _length)
        {
            _bit[current] += delta;
            current += LowBit(current);
        }
    }

    /// <summary>
    /// Return the sum of elements in the inclusive range 1..index.
    /// </summary>
    public double PrefixSum(int index)
    {
        if (index < 0 || index > _length)
        {
            throw new FenwickIndexOutOfRangeError(
                $"prefixSum index {index} out of range [0, {_length}]");
        }

        var total = 0.0;
        var current = index;
        while (current > 0)
        {
            total += _bit[current];
            current -= LowBit(current);
        }

        return total;
    }

    /// <summary>
    /// Return the sum of the inclusive range left..right.
    /// </summary>
    public double RangeSum(int left, int right)
    {
        if (left > right)
        {
            throw new FenwickError($"left ({left}) must be <= right ({right})");
        }

        CheckIndex(left);
        CheckIndex(right);

        return left == 1
            ? PrefixSum(right)
            : PrefixSum(right) - PrefixSum(left - 1);
    }

    /// <summary>
    /// Return the exact value stored at one index.
    /// </summary>
    public double PointQuery(int index)
    {
        CheckIndex(index);
        return RangeSum(index, index);
    }

    /// <summary>
    /// Find the smallest index whose prefix sum is at least <paramref name="target"/>.
    /// </summary>
    public int FindKth(double target)
    {
        if (_length == 0)
        {
            throw new FenwickEmptyTreeError("findKth called on empty tree");
        }

        if (target <= 0.0)
        {
            throw new FenwickError($"k must be positive, got {FormatDouble(target)}");
        }

        var total = PrefixSum(_length);
        if (target > total)
        {
            throw new FenwickError(
                $"k ({FormatDouble(target)}) exceeds total sum of the tree ({FormatDouble(total)})");
        }

        var index = 0;
        var remaining = target;
        var step = HighestPowerOfTwoAtMost(_length);

        while (step > 0)
        {
            var nextIndex = index + step;
            if (nextIndex <= _length && _bit[nextIndex] < remaining)
            {
                index = nextIndex;
                remaining -= _bit[nextIndex];
            }

            step >>= 1;
        }

        return index + 1;
    }

    /// <inheritdoc />
    public override string ToString()
    {
        var rendered = string.Join(", ", _bit.Skip(1).Select(FormatDouble));
        return $"FenwickTree(n={_length}, bit=[{rendered}])";
    }

    private static int LowBit(int index) => index & -index;

    private static int HighestPowerOfTwoAtMost(int value)
    {
        var result = 1;
        while (result <= value / 2)
        {
            result <<= 1;
        }

        return result;
    }

    private static string FormatDouble(double value) =>
        value.ToString("G17", CultureInfo.InvariantCulture);

    private void CheckIndex(int index)
    {
        if (index < 1 || index > _length)
        {
            throw new FenwickIndexOutOfRangeError(
                $"Index {index} out of range [1, {_length}]");
        }
    }
}
