using System;
using System.Collections;
using System.Collections.Generic;
using System.Numerics;
using System.Text;

namespace CodingAdventures.Bitset;

// Bitset.cs -- Bitsets as densely packed boolean arrays
// =====================================================
//
// A normal bool array spends one byte per flag. A bitset spends one bit.
// That 8x reduction matters because the real payoff is not only memory but
// also bulk operations: a single 64-bit AND instruction combines 64 flags at
// once.
//
// This implementation follows the same LSB-first layout used by the Rust,
// Python, and TypeScript packages:
//
//   bit 0  -> least significant bit of word 0
//   bit 63 -> most significant bit of word 0
//   bit 64 -> least significant bit of word 1
//
// The bitset grows automatically when Set or Toggle reach beyond the current
// length. That makes it behave more like a dynamic array than a fixed bitmap.

/// <summary>
/// Raised when a binary-string constructor receives characters other than
/// <c>0</c> and <c>1</c>.
/// </summary>
public sealed class BitsetError : Exception
{
    /// <summary>
    /// Create an error that records the invalid binary input.
    /// </summary>
    public BitsetError(string input)
        : base($"Invalid binary string: \"{input}\".")
    {
        Input = input;
    }

    /// <summary>
    /// The offending input string.
    /// </summary>
    public string Input { get; }
}

/// <summary>
/// A compact boolean array packed into 64-bit words.
/// </summary>
public sealed class Bitset : IEquatable<Bitset>, IEnumerable<int>
{
    private const int BitsPerWord = 64;

    private readonly List<ulong> _words;
    private int _length;

    /// <summary>
    /// Create a zero-filled bitset with the requested logical length.
    /// </summary>
    public Bitset(int size)
    {
        if (size < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(size), "Bitset size cannot be negative.");
        }

        _length = size;
        _words = new List<ulong>(WordsNeeded(size));
        for (var i = 0; i < WordsNeeded(size); i++)
        {
            _words.Add(0);
        }
    }

    private Bitset(List<ulong> words, int length)
    {
        _words = words;
        _length = length;
        CleanTrailingBits();
    }

    /// <summary>
    /// Logical size: the number of addressable bits.
    /// </summary>
    public int Length => _length;

    /// <summary>
    /// Allocated size rounded to a multiple of 64.
    /// </summary>
    public int Capacity => _words.Count * BitsPerWord;

    /// <summary>
    /// Whether the bitset has zero logical length.
    /// </summary>
    public bool IsEmpty => _length == 0;

    /// <summary>
    /// Create a bitset from a non-negative integer using LSB-first ordering.
    /// </summary>
    public static Bitset FromInteger(UInt128 value)
    {
        if (value == UInt128.Zero)
        {
            return new Bitset(0);
        }

        var low = (ulong)value;
        var high = (ulong)(value >> 64);
        var length = high != 0
            ? 64 + (64 - BitOperations.LeadingZeroCount(high))
            : 64 - BitOperations.LeadingZeroCount(low);

        var words = new List<ulong> { low };
        if (high != 0)
        {
            words.Add(high);
        }

        return new Bitset(words, length);
    }

    /// <summary>
    /// Create a bitset from a string whose leftmost character is the highest bit.
    /// </summary>
    public static Bitset FromBinaryString(string value)
    {
        ArgumentNullException.ThrowIfNull(value);

        foreach (var ch in value)
        {
            if (ch != '0' && ch != '1')
            {
                throw new BitsetError(value);
            }
        }

        var bitset = new Bitset(value.Length);
        for (var bitIndex = 0; bitIndex < value.Length; bitIndex++)
        {
            var ch = value[value.Length - 1 - bitIndex];
            if (ch == '1')
            {
                bitset._words[WordIndex(bitIndex)] |= Bitmask(bitIndex);
            }
        }

        bitset.CleanTrailingBits();
        return bitset;
    }

    /// <summary>
    /// Set a bit to 1, growing the bitset if needed.
    /// </summary>
    public void Set(int index)
    {
        EnsureCapacity(index);
        _words[WordIndex(index)] |= Bitmask(index);
    }

    /// <summary>
    /// Set a bit to 0. Clearing beyond <see cref="Length"/> is a no-op.
    /// </summary>
    public void Clear(int index)
    {
        ValidateIndex(index);
        if (index >= _length)
        {
            return;
        }

        _words[WordIndex(index)] &= ~Bitmask(index);
    }

    /// <summary>
    /// Return whether a bit is set. Testing beyond <see cref="Length"/> is false.
    /// </summary>
    public bool Test(int index)
    {
        ValidateIndex(index);
        if (index >= _length)
        {
            return false;
        }

        return (_words[WordIndex(index)] & Bitmask(index)) != 0;
    }

    /// <summary>
    /// Flip a bit, growing the bitset if needed.
    /// </summary>
    public void Toggle(int index)
    {
        EnsureCapacity(index);
        _words[WordIndex(index)] ^= Bitmask(index);
        CleanTrailingBits();
    }

    /// <summary>
    /// Bitwise AND. The result length is the longer input length.
    /// </summary>
    public Bitset And(Bitset other) => BinaryOp(other, static (left, right) => left & right);

    /// <summary>
    /// Bitwise OR. The result length is the longer input length.
    /// </summary>
    public Bitset Or(Bitset other) => BinaryOp(other, static (left, right) => left | right);

    /// <summary>
    /// Bitwise XOR. The result length is the longer input length.
    /// </summary>
    public Bitset Xor(Bitset other) => BinaryOp(other, static (left, right) => left ^ right);

    /// <summary>
    /// Bitwise complement within the logical length of the bitset.
    /// </summary>
    public Bitset Not()
    {
        var relevantWords = WordsNeeded(_length);
        var words = new List<ulong>(relevantWords);
        for (var i = 0; i < relevantWords; i++)
        {
            words.Add(~GetWord(i));
        }

        return new Bitset(words, _length);
    }

    /// <summary>
    /// Set difference: keep bits set in this bitset that are not set in <paramref name="other"/>.
    /// </summary>
    public Bitset AndNot(Bitset other) => BinaryOp(other, static (left, right) => left & ~right);

    /// <summary>
    /// Count how many bits are set to 1.
    /// </summary>
    public int PopCount()
    {
        var count = 0;
        for (var i = 0; i < WordsNeeded(_length); i++)
        {
            count += BitOperations.PopCount(GetWord(i));
        }

        return count;
    }

    /// <summary>
    /// Return whether at least one bit is set.
    /// </summary>
    public bool Any()
    {
        for (var i = 0; i < WordsNeeded(_length); i++)
        {
            if (GetWord(i) != 0)
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// Return whether all bits within <see cref="Length"/> are set.
    /// Empty bitsets satisfy this vacuously.
    /// </summary>
    public bool All()
    {
        if (_length == 0)
        {
            return true;
        }

        var fullWords = _length / BitsPerWord;
        for (var i = 0; i < fullWords; i++)
        {
            if (GetWord(i) != ulong.MaxValue)
            {
                return false;
            }
        }

        var remainingBits = _length % BitsPerWord;
        if (remainingBits == 0)
        {
            return true;
        }

        return GetWord(fullWords) == TrailingMask(remainingBits);
    }

    /// <summary>
    /// Return whether no bits are set.
    /// </summary>
    public bool None() => !Any();

    /// <summary>
    /// Iterate over the indices of set bits in ascending order.
    /// </summary>
    public IEnumerable<int> IterSetBits()
    {
        var relevantWords = WordsNeeded(_length);
        for (var wordIndex = 0; wordIndex < relevantWords; wordIndex++)
        {
            var word = GetWord(wordIndex);
            while (word != 0)
            {
                var offset = BitOperations.TrailingZeroCount(word);
                yield return (wordIndex * BitsPerWord) + offset;
                word &= word - 1;
            }
        }
    }

    /// <summary>
    /// Convert to a 64-bit integer when the value fits in a single word.
    /// Returns <see langword="null"/> otherwise.
    /// </summary>
    public ulong? ToInteger()
    {
        var relevantWords = WordsNeeded(_length);
        if (relevantWords == 0)
        {
            return 0;
        }

        for (var i = 1; i < relevantWords; i++)
        {
            if (GetWord(i) != 0)
            {
                return null;
            }
        }

        return GetWord(0);
    }

    /// <summary>
    /// Convert to a conventional binary string with the highest bit on the left.
    /// </summary>
    public string ToBinaryString()
    {
        if (_length == 0)
        {
            return string.Empty;
        }

        var builder = new StringBuilder(_length);
        for (var i = _length - 1; i >= 0; i--)
        {
            builder.Append(Test(i) ? '1' : '0');
        }

        return builder.ToString();
    }

    /// <summary>
    /// Convenience alias for <see cref="Test"/>.
    /// </summary>
    public bool Contains(int index) => Test(index);

    /// <summary>
    /// Return a debug-friendly representation such as <c>Bitset(101)</c>.
    /// </summary>
    public override string ToString() => $"Bitset({ToBinaryString()})";

    /// <summary>
    /// Compare two bitsets by logical length and logical bits, ignoring spare capacity.
    /// </summary>
    public bool Equals(Bitset? other)
    {
        if (other is null || _length != other._length)
        {
            return false;
        }

        var relevantWords = WordsNeeded(_length);
        for (var i = 0; i < relevantWords; i++)
        {
            if (GetWord(i) != other.GetWord(i))
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>
    /// Compare this bitset to another object.
    /// </summary>
    public override bool Equals(object? obj) => obj is Bitset other && Equals(other);

    /// <summary>
    /// Compute a hash from the logical length and words inside that logical range.
    /// </summary>
    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(_length);
        for (var i = 0; i < WordsNeeded(_length); i++)
        {
            hash.Add(GetWord(i));
        }

        return hash.ToHashCode();
    }

    /// <summary>
    /// Enumerate the indices of all set bits.
    /// </summary>
    public IEnumerator<int> GetEnumerator() => IterSetBits().GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    /// <summary>
    /// Operator shorthand for <see cref="And"/>.
    /// </summary>
    public static Bitset operator &(Bitset left, Bitset right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.And(right);
    }

    /// <summary>
    /// Operator shorthand for <see cref="Or"/>.
    /// </summary>
    public static Bitset operator |(Bitset left, Bitset right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Or(right);
    }

    /// <summary>
    /// Operator shorthand for <see cref="Xor"/>.
    /// </summary>
    public static Bitset operator ^(Bitset left, Bitset right)
    {
        ArgumentNullException.ThrowIfNull(left);
        return left.Xor(right);
    }

    /// <summary>
    /// Operator shorthand for <see cref="Not"/>.
    /// </summary>
    public static Bitset operator ~(Bitset value)
    {
        ArgumentNullException.ThrowIfNull(value);
        return value.Not();
    }

    /// <summary>
    /// Equality operator based on logical length and logical bits.
    /// </summary>
    public static bool operator ==(Bitset? left, Bitset? right)
    {
        if (ReferenceEquals(left, right))
        {
            return true;
        }

        if (left is null || right is null)
        {
            return false;
        }

        return left.Equals(right);
    }

    /// <summary>
    /// Inequality operator based on logical length and logical bits.
    /// </summary>
    public static bool operator !=(Bitset? left, Bitset? right) => !(left == right);

    private Bitset BinaryOp(Bitset other, Func<ulong, ulong, ulong> operation)
    {
        ArgumentNullException.ThrowIfNull(other);

        var resultLength = Math.Max(_length, other._length);
        var relevantWords = WordsNeeded(resultLength);
        var words = new List<ulong>(relevantWords);
        for (var i = 0; i < relevantWords; i++)
        {
            words.Add(operation(GetWord(i), other.GetWord(i)));
        }

        return new Bitset(words, resultLength);
    }

    private ulong GetWord(int index) => index < _words.Count ? _words[index] : 0;

    private void EnsureCapacity(int index)
    {
        ValidateIndex(index);

        if (index >= Capacity)
        {
            var newCapacity = Capacity == 0 ? BitsPerWord : Capacity;
            while (index >= newCapacity)
            {
                newCapacity *= 2;
            }

            var targetWordCount = WordsNeeded(newCapacity);
            while (_words.Count < targetWordCount)
            {
                _words.Add(0);
            }
        }

        if (index + 1 > _length)
        {
            _length = index + 1;
        }
    }

    private void CleanTrailingBits()
    {
        var relevantWords = WordsNeeded(_length);

        for (var i = relevantWords; i < _words.Count; i++)
        {
            _words[i] = 0;
        }

        if (relevantWords == 0)
        {
            return;
        }

        var remainingBits = _length % BitsPerWord;
        if (remainingBits == 0)
        {
            return;
        }

        _words[relevantWords - 1] &= TrailingMask(remainingBits);
    }

    private static void ValidateIndex(int index)
    {
        if (index < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(index), "Bit indices cannot be negative.");
        }
    }

    private static int WordsNeeded(int bitCount) => bitCount == 0 ? 0 : ((bitCount - 1) / BitsPerWord) + 1;

    private static int WordIndex(int index) => index / BitsPerWord;

    private static ulong Bitmask(int index) => 1UL << (index % BitsPerWord);

    private static ulong TrailingMask(int bitCount) => bitCount == BitsPerWord ? ulong.MaxValue : (1UL << bitCount) - 1;
}
