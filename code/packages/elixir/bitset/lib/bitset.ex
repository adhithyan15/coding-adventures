# lib/bitset.ex -- Bitset: A Compact Boolean Array Packed into 64-bit Words
# ===========================================================================
#
# A bitset stores a sequence of bits -- each one either 0 or 1 -- packed
# into machine-word-sized integers. Instead of using an entire byte to
# represent a single true/false value, a bitset packs 64 of them into a
# single word.
#
# Why does this matter?
#
# 1. **Space**: 10,000 booleans as a list of atoms = ~80,000 bytes.
#    As a bitset = ~1,250 bytes. That's a 64x improvement.
#
# 2. **Speed**: OR-ing two boolean lists loops over 10,000 elements.
#    OR-ing two bitsets loops over ~157 words. Elixir's Bitwise module
#    performs a single 64-bit operation on each word, handling 64 bits
#    at once.
#
# 3. **Ubiquity**: Bitsets appear in Bloom filters, register allocators,
#    graph algorithms (visited sets), database bitmap indexes, filesystem
#    free-block bitmaps, network subnet masks, and garbage collectors.
#
# Bit Ordering: LSB-First
# -----------------------
#
# We use Least Significant Bit first ordering. Bit 0 is the least significant
# bit of word 0. Bit 63 is the most significant bit of word 0. Bit 64 is the
# least significant bit of word 1. And so on.
#
#     Word 0                              Word 1
#     +-----------------------------+     +-----------------------------+
#     | bit 63  ...  bit 2  bit 1  bit 0| | bit 127 ... bit 65  bit 64 |
#     +-----------------------------+     +-----------------------------+
#     MSB <------------------- LSB        MSB <------------------- LSB
#
# The three fundamental formulas that drive every bitset operation:
#
#     word_index = div(i, 64)       (which word contains bit i?)
#     bit_offset = rem(i, 64)       (which position within that word?)
#     bitmask    = 1 <<< rem(i, 64) (a mask with only bit i set)
#
# These are the heart of the entire implementation.
#
# Functional Style
# ----------------
#
# Because Elixir is a functional language with immutable data, every
# operation returns a NEW bitset rather than modifying in place. This
# is different from the Rust implementation which uses `&mut self`.
# The tradeoff is clarity and safety over raw performance -- there's
# no aliasing or mutation bugs possible.

defmodule CodingAdventures.Bitset do
  @moduledoc """
  A compact bitset that packs boolean values into 64-bit integer words.

  `Bitset` provides O(n/64) bulk bitwise operations (AND, OR, XOR, NOT),
  efficient iteration over set bits using trailing-zero-count, and
  ArrayList-style automatic growth when you set bits beyond the current size.

  ## Quick Start

      alias CodingAdventures.Bitset

      bs = Bitset.new(100)
           |> Bitset.set(0)
           |> Bitset.set(42)
           |> Bitset.set(99)

      Bitset.popcount(bs)    # => 3
      Bitset.set_bits(bs)    # => [0, 42, 99]

  ## Immutability

  All operations return a new Bitset. The original is never modified.
  This is the standard Elixir convention for data structures.

  ## Internal Representation

  Bits are stored in a list of integers, each holding 64 bits. We also
  track `len`, the logical size (number of addressable bits).

      +--------------------------------------------------------------+
      |                          capacity (256 bits = 4 words)        |
      |                                                               |
      |  +------------------------------------------+                 |
      |  |              len (200 bits)               | ... unused ... |
      |  |  (highest addressable bit index + 1)      | (always zero)  |
      |  +------------------------------------------+                 |
      +--------------------------------------------------------------+

  **Clean-trailing-bits invariant**: Bits beyond `len` in the last word
  are always zero. This is critical for correctness of popcount, any?,
  all?, none?, equality, and to_integer.
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # Constants
  # ---------------------------------------------------------------------------
  #
  # BITS_PER_WORD is 64 because we use 64-bit integers as our word type.
  # Every formula in this module uses this constant rather than a magic
  # number, so if someone ever wanted to experiment with 32-bit words,
  # they'd only need to change this constant.

  @bits_per_word 64

  # A full word with all 64 bits set: 0xFFFFFFFFFFFFFFFF.
  # Used in the all?/1 function to check if every bit in a word is 1.
  @full_word (1 <<< @bits_per_word) - 1

  # ---------------------------------------------------------------------------
  # Error module
  # ---------------------------------------------------------------------------
  #
  # We define a single exception type for bitset errors. Currently the only
  # error is an invalid binary string, but this gives us a named exception
  # type that callers can pattern match on.

  defmodule BitsetError do
    @moduledoc """
    Exception raised for invalid bitset operations.

    Currently raised when `from_binary_str/1` receives a string containing
    characters other than '0' and '1'.
    """
    defexception [:message]
  end

  # ---------------------------------------------------------------------------
  # The Bitset struct
  # ---------------------------------------------------------------------------
  #
  # We store bits in a list of integers called `words`. Each integer holds
  # 64 bits (we mask to 64 bits on every write to avoid Elixir's arbitrary
  # precision integers from growing unbounded).
  #
  # We also track `len`, the logical size -- the number of bits the user
  # considers "addressable". The capacity is always length(words) * 64.
  #
  # Why a list instead of a tuple? Lists are Elixir's standard immutable
  # sequence type. Although random access is O(n), we typically iterate
  # over all words sequentially for bulk operations, which is O(n) anyway.
  # For single-bit operations we do need to index into the list, which is
  # O(word_count) -- acceptable for the educational focus of this package.

  defstruct words: [], len: 0

  @type t :: %__MODULE__{
          words: [non_neg_integer()],
          len: non_neg_integer()
        }

  # ---------------------------------------------------------------------------
  # Helper functions (private)
  # ---------------------------------------------------------------------------
  #
  # These small utility functions compute the word index, bit offset, number
  # of words needed, and bitmask for a given bit position. They're used
  # throughout the implementation.

  # How many 64-bit words do we need to store `bit_count` bits?
  #
  # This is ceiling division: div(bit_count + 63, 64).
  #
  #     words_needed(0)   = 0   (no bits, no words)
  #     words_needed(1)   = 1   (1 bit needs 1 word)
  #     words_needed(64)  = 1   (64 bits fit exactly in 1 word)
  #     words_needed(65)  = 2   (65 bits need 2 words)
  #     words_needed(128) = 2   (128 bits fit exactly in 2 words)
  #     words_needed(200) = 4   (200 bits need ceil(200/64) = 4 words)
  defp words_needed(bit_count) do
    div(bit_count + @bits_per_word - 1, @bits_per_word)
  end

  # Which word contains bit `i`? Simply div(i, 64).
  #
  #     word_index(0)   = 0   (bit 0 is in word 0)
  #     word_index(63)  = 0   (bit 63 is the last bit of word 0)
  #     word_index(64)  = 1   (bit 64 is the first bit of word 1)
  #     word_index(137) = 2   (bit 137 is in word 2)
  defp word_index(i), do: div(i, @bits_per_word)

  # Which bit position within its word does bit `i` occupy? Simply rem(i, 64).
  #
  #     bit_offset(0)   = 0
  #     bit_offset(63)  = 63
  #     bit_offset(64)  = 0   (first bit of the next word)
  #     bit_offset(137) = 9   (137 - 2*64 = 9)
  defp bit_offset(i), do: rem(i, @bits_per_word)

  # A bitmask with only bit `i` set within its word.
  #
  # This is 1 <<< rem(i, 64). We use this mask to isolate, set, clear,
  # or toggle a single bit within a word using bitwise operations:
  #
  #     To set bit i:    word ||| bitmask(i)      (OR with mask turns bit on)
  #     To clear bit i:  word &&& bnot(bitmask(i)) (AND with inverted mask turns bit off)
  #     To test bit i:   (word &&& bitmask(i)) != 0 (AND with mask isolates the bit)
  #     To toggle bit i: bxor(word, bitmask(i))    (XOR with mask flips the bit)
  defp bitmask(i), do: 1 <<< bit_offset(i)

  # Mask to 64 bits. Elixir integers are arbitrary precision, so after
  # bitwise NOT or other operations we must mask to prevent the integer
  # from growing to negative territory or beyond 64 bits.
  defp mask64(value), do: value &&& @full_word

  # Replace the element at `index` in `list` with `value`.
  # Returns a new list. This is O(n) but our word lists are small
  # (a 10,000-bit bitset has only ~157 words).
  defp list_replace_at(list, index, value) do
    List.replace_at(list, index, value)
  end

  # Get the element at `index` in a list, returning `default` if out of bounds.
  # Elixir's Enum.at/3 does this but we wrap it for clarity.
  defp word_at(words, index, default \\ 0) do
    Enum.at(words, index, default)
  end

  # Zero out any bits beyond `len` in the last word.
  #
  # This maintains the clean-trailing-bits invariant. It must be called
  # after any operation that might set bits beyond `len`:
  #   - flip_all/1 flips all bits, including trailing ones
  #   - toggle/2 on the last word
  #   - bulk operations when operands have different sizes
  #
  # How it works:
  #
  #     len = 200, capacity = 256
  #     The last word holds bits 192-255, but only 192-199 are "real".
  #     remaining = rem(200, 64) = 8
  #     mask = (1 <<< 8) - 1 = 0xFF  (bits 0-7)
  #     last_word &&& 0xFF  -> zeroes out bits 8-63 of last word
  #
  # If `len` is a multiple of 64, there are no trailing bits to clean.
  defp clean_trailing_bits(%__MODULE__{len: 0} = bs), do: bs
  defp clean_trailing_bits(%__MODULE__{words: []} = bs), do: bs

  defp clean_trailing_bits(%__MODULE__{words: words, len: len} = bs) do
    remaining = bit_offset(len)

    if remaining == 0 do
      # len is a multiple of 64 -- no trailing bits to clean.
      bs
    else
      last_idx = length(words) - 1
      clean_mask = (1 <<< remaining) - 1
      last_word = Enum.at(words, last_idx)
      cleaned = last_word &&& clean_mask
      %{bs | words: list_replace_at(words, last_idx, cleaned)}
    end
  end

  # Ensure the bitset has capacity for bit `i`. If not, grow by doubling.
  #
  # After this call, capacity > i and len >= i + 1.
  #
  # Growth strategy:
  #
  # We double the capacity repeatedly until it exceeds `i`. The minimum
  # capacity after growth is 64 (one word). This doubling strategy gives
  # amortized O(1) growth, just like ArrayList.
  #
  #     Example: capacity=128, set(500)
  #       128 -> 256 -> 512 -> 1024  (stop: 500 < 1024)
  defp ensure_capacity(%__MODULE__{words: words, len: len} = bs, i) do
    current_cap = length(words) * @bits_per_word

    if i < current_cap do
      # Already have room. But we might need to update len.
      new_len = max(len, i + 1)
      %{bs | len: new_len}
    else
      # Need to grow. Start with current capacity (or 64 as minimum).
      new_cap = grow_capacity(max(current_cap, @bits_per_word), i)
      new_word_count = div(new_cap, @bits_per_word)
      # Extend the word list with zeros.
      extra = List.duplicate(0, new_word_count - length(words))
      new_len = max(len, i + 1)
      %{bs | words: words ++ extra, len: new_len}
    end
  end

  # Double capacity until it exceeds the target bit index.
  defp grow_capacity(cap, target) when cap > target, do: cap
  defp grow_capacity(cap, target), do: grow_capacity(cap * 2, target)

  # ---------------------------------------------------------------------------
  # Constructors
  # ---------------------------------------------------------------------------

  @doc """
  Create a new bitset with all bits initially zero.

  The `size` parameter sets the logical length (`len`). The capacity
  is rounded up to the next multiple of 64.

  ## Examples

      iex> bs = CodingAdventures.Bitset.new(100)
      iex> CodingAdventures.Bitset.size(bs)
      100
      iex> CodingAdventures.Bitset.capacity(bs)
      128

  `new(0)` is valid and creates an empty bitset:

      iex> bs = CodingAdventures.Bitset.new(0)
      iex> CodingAdventures.Bitset.size(bs)
      0
      iex> CodingAdventures.Bitset.capacity(bs)
      0
  """
  @spec new(non_neg_integer()) :: t()
  def new(size_val) when is_integer(size_val) and size_val >= 0 do
    word_count = words_needed(size_val)
    %__MODULE__{words: List.duplicate(0, word_count), len: size_val}
  end

  @doc """
  Create a bitset from a non-negative integer.

  Bit 0 of the bitset is the least significant bit of `value`.
  The `len` of the result is the position of the highest set bit + 1.
  If `value == 0`, then `len = 0`.

  Elixir has arbitrary precision integers, so any non-negative integer
  is accepted -- there's no overflow concern.

  ## How it works

  We split the integer into 64-bit words by repeatedly masking off
  the lowest 64 bits and shifting right:

      value = 0x0000_0000_0000_0005  (decimal 5, binary 101)
      word 0 = value &&& 0xFFFF_FFFF_FFFF_FFFF = 5
      value  = value >>> 64 = 0
      (stop when value reaches 0)

  Then we compute `len` by finding the highest set bit. For Elixir
  integers, we count the bit length using a recursive function.

  ## Examples

      iex> bs = CodingAdventures.Bitset.from_integer(5)
      iex> CodingAdventures.Bitset.size(bs)
      3
      iex> CodingAdventures.Bitset.test?(bs, 0)
      true
      iex> CodingAdventures.Bitset.test?(bs, 1)
      false
      iex> CodingAdventures.Bitset.test?(bs, 2)
      true
  """
  @spec from_integer(non_neg_integer()) :: t()
  def from_integer(0), do: new(0)

  def from_integer(value) when is_integer(value) and value > 0 do
    # Compute the logical length: position of highest set bit + 1.
    # For Elixir, we can use :erlang.system_info or compute manually.
    # The bit_length of a positive integer n is floor(log2(n)) + 1.
    bit_len = integer_bit_length(value)

    # Split the integer into 64-bit words by repeatedly extracting
    # the lowest 64 bits.
    words = split_into_words(value, [])

    %__MODULE__{words: words, len: bit_len}
  end

  # Compute the number of bits needed to represent a positive integer.
  # This is equivalent to floor(log2(n)) + 1.
  #
  #     integer_bit_length(1)   = 1   (binary: 1)
  #     integer_bit_length(5)   = 3   (binary: 101)
  #     integer_bit_length(255) = 8   (binary: 11111111)
  #     integer_bit_length(256) = 9   (binary: 100000000)
  defp integer_bit_length(0), do: 0
  defp integer_bit_length(n) when n > 0, do: count_bits(n, 0)

  defp count_bits(0, acc), do: acc
  defp count_bits(n, acc), do: count_bits(n >>> 1, acc + 1)

  # Split a non-negative integer into a list of 64-bit words (LSB first).
  #
  #     split_into_words(5, [])       => [5]
  #     split_into_words(2^64 + 3, []) => [3, 1]
  defp split_into_words(0, acc), do: Enum.reverse(acc)

  defp split_into_words(value, acc) do
    word = value &&& @full_word
    split_into_words(value >>> @bits_per_word, [word | acc])
  end

  @doc """
  Create a bitset from a binary string like `"1010"`.

  The leftmost character is the highest-indexed bit (conventional binary
  notation, matching how humans write numbers). The rightmost character
  is bit 0.

  ## String-to-bits mapping

      Input string: "1 0 1 0"
      Position:      3 2 1 0    (leftmost = highest bit index)

      Bit 0 = '0' (rightmost char)
      Bit 1 = '1'
      Bit 2 = '0'
      Bit 3 = '1' (leftmost char)

      This is the same as the integer 10 (binary 1010).

  ## Return value

  Returns `{:ok, bitset}` on success, or `{:error, message}` if the
  string contains characters other than '0' and '1'.

  ## Examples

      iex> {:ok, bs} = CodingAdventures.Bitset.from_binary_str("1010")
      iex> CodingAdventures.Bitset.size(bs)
      4
      iex> CodingAdventures.Bitset.test?(bs, 1)
      true
      iex> CodingAdventures.Bitset.test?(bs, 3)
      true
      iex> CodingAdventures.Bitset.test?(bs, 0)
      false
  """
  @spec from_binary_str(String.t()) :: {:ok, t()} | {:error, String.t()}
  def from_binary_str(""), do: {:ok, new(0)}

  def from_binary_str(str) when is_binary(str) do
    # Validate: every character must be '0' or '1'.
    if String.match?(str, ~r/^[01]+$/) do
      # The string length is the logical len of the bitset.
      bit_len = String.length(str)
      bs = new(bit_len)

      # Walk the string from right to left (LSB to MSB).
      # The rightmost character (last char) is bit 0.
      # The leftmost character (first char) is bit (len - 1).
      chars = String.graphemes(str) |> Enum.reverse()

      words =
        chars
        |> Enum.with_index()
        |> Enum.reduce(bs.words, fn {char, bit_idx}, acc_words ->
          if char == "1" do
            wi = word_index(bit_idx)
            current = Enum.at(acc_words, wi)
            list_replace_at(acc_words, wi, current ||| bitmask(bit_idx))
          else
            acc_words
          end
        end)

      result = %{bs | words: words}
      {:ok, clean_trailing_bits(result)}
    else
      {:error, "invalid binary string: #{inspect(str)}"}
    end
  end

  @doc """
  Like `from_binary_str/1` but raises `BitsetError` on invalid input.

  ## Examples

      iex> bs = CodingAdventures.Bitset.from_binary_str!("101")
      iex> CodingAdventures.Bitset.to_integer(bs)
      5
  """
  @spec from_binary_str!(String.t()) :: t()
  def from_binary_str!(str) do
    case from_binary_str(str) do
      {:ok, bs} -> bs
      {:error, msg} -> raise BitsetError, message: msg
    end
  end

  # ---------------------------------------------------------------------------
  # Single-bit operations
  # ---------------------------------------------------------------------------
  #
  # These are the bread-and-butter operations: set a bit, clear a bit,
  # test whether a bit is set, toggle a bit. Each one translates to a
  # single bitwise operation on the containing word.
  #
  # Growth semantics:
  #   - set/2 and toggle/2 AUTO-GROW the bitset if i >= len.
  #   - test?/2 and clear/2 do NOT grow. They return false / the unchanged
  #     bitset for out-of-range indices. This is safe because unallocated
  #     bits are conceptually zero.

  @doc """
  Set bit `i` to 1. Auto-grows the bitset if `i >= len`.

  Returns a new bitset with bit `i` set.

  ## How auto-growth works

  If `i` is beyond the current capacity, we double the capacity
  repeatedly until it's large enough (with a minimum of 64 bits).
  This is the same amortized O(1) strategy used by ArrayList.

      Before: len=100, capacity=128 (2 words)
      set(200): 200 >= 128, so double: 128 -> 256. Now 200 < 256.
      After: len=201, capacity=256 (4 words)

  ## Examples

      iex> bs = CodingAdventures.Bitset.new(10) |> CodingAdventures.Bitset.set(5)
      iex> CodingAdventures.Bitset.test?(bs, 5)
      true
  """
  @spec set(t(), non_neg_integer()) :: t()
  def set(%__MODULE__{} = bs, i) when is_integer(i) and i >= 0 do
    bs = ensure_capacity(bs, i)
    wi = word_index(i)
    current = Enum.at(bs.words, wi)
    # The core operation: OR the bitmask into the word.
    #
    #     word  = 0b...0000_0000
    #     mask  = 0b...0010_0000   (bit 5 within the word)
    #     result= 0b...0010_0000   (bit 5 is now set)
    #
    # OR is idempotent: setting an already-set bit is a no-op.
    new_word = current ||| bitmask(i)
    %{bs | words: list_replace_at(bs.words, wi, new_word)}
  end

  @doc """
  Set bit `i` to 0. No-op if `i >= len` (does not grow).

  Returns a new bitset with bit `i` cleared. Clearing a bit that's
  already 0 is a no-op. Clearing a bit beyond the bitset's length
  returns the bitset unchanged.

  ## How it works

  We AND the word with the inverted bitmask. The inverted mask has all
  bits set EXCEPT the target bit, so every other bit is preserved:

      word  = 0b...0010_0100   (bits 2 and 5 set)
      mask  = 0b...0010_0000   (bit 5)
      ~mask = 0b...1101_1111   (everything except bit 5)
      result= 0b...0000_0100   (bit 5 cleared, bit 2 preserved)

  ## Examples

      iex> bs = CodingAdventures.Bitset.new(10) |> CodingAdventures.Bitset.set(5)
      iex> bs = CodingAdventures.Bitset.clear(bs, 5)
      iex> CodingAdventures.Bitset.test?(bs, 5)
      false
  """
  @spec clear(t(), non_neg_integer()) :: t()
  def clear(%__MODULE__{len: len} = bs, i) when is_integer(i) and i >= 0 do
    if i >= len do
      # Out of range: nothing to clear. Don't grow.
      bs
    else
      wi = word_index(i)
      current = Enum.at(bs.words, wi)
      # AND with the inverted mask clears exactly one bit.
      new_word = current &&& mask64(bnot(bitmask(i)))
      %{bs | words: list_replace_at(bs.words, wi, new_word)}
    end
  end

  @doc """
  Test whether bit `i` is set. Returns `false` if `i >= len`.

  This is a pure read operation -- it never modifies the bitset.
  Testing a bit beyond the bitset's length returns false because
  unallocated bits are conceptually zero.

  ## How it works

  We AND the word with the bitmask. If the result is non-zero, the
  bit is set:

      word  = 0b...0010_0100   (bits 2 and 5 set)
      mask  = 0b...0010_0000   (bit 5)
      result= 0b...0010_0000   (non-zero -> bit 5 is set)

  ## Examples

      iex> bs = CodingAdventures.Bitset.new(10) |> CodingAdventures.Bitset.set(5)
      iex> CodingAdventures.Bitset.test?(bs, 5)
      true
      iex> CodingAdventures.Bitset.test?(bs, 3)
      false
      iex> CodingAdventures.Bitset.test?(bs, 999)
      false
  """
  @spec test?(t(), non_neg_integer()) :: boolean()
  def test?(%__MODULE__{len: len}, i) when is_integer(i) and i >= 0 and i >= len, do: false

  def test?(%__MODULE__{words: words}, i) when is_integer(i) and i >= 0 do
    wi = word_index(i)
    word = Enum.at(words, wi)
    (word &&& bitmask(i)) != 0
  end

  @doc """
  Toggle (flip) bit `i`. Auto-grows if `i >= len`.

  If the bit is 0, it becomes 1. If it's 1, it becomes 0.
  Returns a new bitset.

  ## How it works

  XOR with the bitmask flips exactly one bit:

      word  = 0b...0010_0100   (bits 2 and 5 set)
      mask  = 0b...0010_0000   (bit 5)
      result= 0b...0000_0100   (bit 5 flipped to 0)

  ## Examples

      iex> bs = CodingAdventures.Bitset.new(10) |> CodingAdventures.Bitset.toggle(5)
      iex> CodingAdventures.Bitset.test?(bs, 5)
      true
      iex> bs = CodingAdventures.Bitset.toggle(bs, 5)
      iex> CodingAdventures.Bitset.test?(bs, 5)
      false
  """
  @spec toggle(t(), non_neg_integer()) :: t()
  def toggle(%__MODULE__{} = bs, i) when is_integer(i) and i >= 0 do
    bs = ensure_capacity(bs, i)
    wi = word_index(i)
    current = Enum.at(bs.words, wi)
    new_word = bxor(current, bitmask(i))
    result = %{bs | words: list_replace_at(bs.words, wi, new_word)}
    # Toggle might have set a bit in the last word's trailing region,
    # so clean trailing bits to maintain the invariant.
    clean_trailing_bits(result)
  end

  # ---------------------------------------------------------------------------
  # Bulk bitwise operations
  # ---------------------------------------------------------------------------
  #
  # All bulk operations return a NEW bitset. They don't modify either
  # operand. The result has len = max(a.len, b.len).
  #
  # When two bitsets have different lengths, the shorter one is
  # "zero-extended" conceptually. In practice, we just stop reading
  # from the shorter one's words once they run out and treat missing
  # words as zero.
  #
  # Performance: each operation processes one 64-bit word per loop
  # iteration, so 64 bits are handled in a single CPU instruction.
  # This is the fundamental performance advantage of bitsets.

  @doc """
  Bitwise AND: result bit is 1 only if BOTH input bits are 1.

  ## Truth table

      A  B  A&B
      0  0   0
      0  1   0
      1  0   0
      1  1   1

  AND is used for **intersection**: elements that are in both sets.

  ## Examples

      iex> a = CodingAdventures.Bitset.from_integer(0b1100)
      iex> b = CodingAdventures.Bitset.from_integer(0b1010)
      iex> c = CodingAdventures.Bitset.bitwise_and(a, b)
      iex> CodingAdventures.Bitset.to_integer(c)
      8
  """
  @spec bitwise_and(t(), t()) :: t()
  def bitwise_and(%__MODULE__{} = a, %__MODULE__{} = b) do
    bulk_op(a, b, fn wa, wb -> wa &&& wb end)
  end

  @doc """
  Bitwise OR: result bit is 1 if EITHER (or both) input bits are 1.

  ## Truth table

      A  B  A|B
      0  0   0
      0  1   1
      1  0   1
      1  1   1

  OR is used for **union**: elements that are in either set.

  ## Examples

      iex> a = CodingAdventures.Bitset.from_integer(0b1100)
      iex> b = CodingAdventures.Bitset.from_integer(0b1010)
      iex> c = CodingAdventures.Bitset.bitwise_or(a, b)
      iex> CodingAdventures.Bitset.to_integer(c)
      14
  """
  @spec bitwise_or(t(), t()) :: t()
  def bitwise_or(%__MODULE__{} = a, %__MODULE__{} = b) do
    bulk_op(a, b, fn wa, wb -> wa ||| wb end)
  end

  @doc """
  Bitwise XOR: result bit is 1 if the input bits DIFFER.

  ## Truth table

      A  B  A^B
      0  0   0
      0  1   1
      1  0   1
      1  1   0

  XOR is used for **symmetric difference**: elements in either set
  but not both.

  ## Examples

      iex> a = CodingAdventures.Bitset.from_integer(0b1100)
      iex> b = CodingAdventures.Bitset.from_integer(0b1010)
      iex> c = CodingAdventures.Bitset.bitwise_xor(a, b)
      iex> CodingAdventures.Bitset.to_integer(c)
      6
  """
  @spec bitwise_xor(t(), t()) :: t()
  def bitwise_xor(%__MODULE__{} = a, %__MODULE__{} = b) do
    bulk_op(a, b, fn wa, wb -> bxor(wa, wb) end)
  end

  @doc """
  Bitwise NOT (flip all bits within `len`).

  ## Truth table

      A  ~A
      0   1
      1   0

  NOT is used for **complement**: elements NOT in the set.

  **Important**: flip_all flips bits within `len`, NOT within `capacity`.
  Bits beyond `len` remain zero (clean-trailing-bits invariant).
  The result has the same `len` as the input.

  ## Why "flip_all" instead of "not"?

  Elixir reserves the word `not` as a keyword. We use `flip_all` to
  convey the same meaning without collision.

  ## Examples

      iex> a = CodingAdventures.Bitset.from_integer(0b1010)
      iex> b = CodingAdventures.Bitset.flip_all(a)
      iex> CodingAdventures.Bitset.to_integer(b)
      5
  """
  @spec flip_all(t()) :: t()
  def flip_all(%__MODULE__{words: words, len: len}) do
    # Flip every bit in every word using bitwise NOT, then mask to 64 bits.
    #
    # Critical: clean trailing bits! The NOT operation flipped ALL bits
    # in every word, including the trailing bits beyond `len` that were
    # zero. We must zero them out again to maintain the invariant.
    result_words = Enum.map(words, fn w -> mask64(bnot(w)) end)

    result = %__MODULE__{words: result_words, len: len}
    clean_trailing_bits(result)
  end

  @doc """
  AND-NOT (set difference): bits in `a` that are NOT in `b`.

  This is equivalent to `bitwise_and(a, flip_all(b))`, but more efficient
  because we don't create an intermediate NOT result.

  ## Truth table

      A  B  A & ~B
      0  0    0
      0  1    0
      1  0    1
      1  1    0

  AND-NOT is used for **set difference**: elements in A but not in B.

  ## Examples

      iex> a = CodingAdventures.Bitset.from_integer(0b1110)
      iex> b = CodingAdventures.Bitset.from_integer(0b1010)
      iex> c = CodingAdventures.Bitset.difference(a, b)
      iex> CodingAdventures.Bitset.to_integer(c)
      4
  """
  @spec difference(t(), t()) :: t()
  def difference(%__MODULE__{} = a, %__MODULE__{} = b) do
    bulk_op(a, b, fn wa, wb -> wa &&& mask64(bnot(wb)) end)
  end

  # Generic helper for binary bulk operations.
  #
  # Takes two bitsets and a function that combines two words, producing
  # a new bitset with len = max(a.len, b.len). After combining, we clean
  # trailing bits to maintain the invariant.
  defp bulk_op(%__MODULE__{words: a_words, len: a_len}, %__MODULE__{words: b_words, len: b_len}, fun) do
    result_len = max(a_len, b_len)
    max_words = max(length(a_words), length(b_words))

    result_words =
      Enum.map(0..(max_words - 1)//1, fn i ->
        wa = word_at(a_words, i)
        wb = word_at(b_words, i)
        mask64(fun.(wa, wb))
      end)

    # Handle edge case: if both bitsets are empty (max_words = 0),
    # the range 0..-1 produces an empty list, which is correct.
    result_words = if max_words == 0, do: [], else: result_words

    result = %__MODULE__{words: result_words, len: result_len}
    clean_trailing_bits(result)
  end

  # ---------------------------------------------------------------------------
  # Counting and query operations
  # ---------------------------------------------------------------------------

  @doc """
  Count the number of set (1) bits. Named after the CPU instruction
  `POPCNT` (population count) that counts set bits in a word.

  For a bitset with N bits, this runs in O(N/64) time -- we process
  64 bits per loop iteration.

  ## How it works

  We use Erlang's `:erlang.popcount/1` (available in OTP 27+) or a
  manual bit-counting fallback for each word, then sum the results.

  ## Examples

      iex> bs = CodingAdventures.Bitset.from_integer(0b10110)
      iex> CodingAdventures.Bitset.popcount(bs)
      3
  """
  @spec popcount(t()) :: non_neg_integer()
  def popcount(%__MODULE__{words: words}) do
    Enum.reduce(words, 0, fn word, acc -> acc + popcount_word(word) end)
  end

  # Count the number of set bits in a single 64-bit word.
  #
  # We use the classic Hamming weight algorithm (bit manipulation trick):
  #
  #     Step 1: Count bits in pairs of 1
  #     Step 2: Count bits in groups of 2
  #     Step 3: Count bits in groups of 4
  #     Step 4: Count bits in groups of 8
  #     Step 5: Sum all bytes
  #
  # This is the same algorithm used in software implementations of POPCNT.
  # It processes all 64 bits in constant time with no loops or branches.
  defp popcount_word(0), do: 0

  defp popcount_word(word) do
    # Kernighan's bit counting: repeatedly clear the lowest set bit
    # and count how many times we can do it. This is O(k) where k is
    # the number of set bits, which is fast for sparse words.
    do_popcount(word, 0)
  end

  defp do_popcount(0, count), do: count

  defp do_popcount(word, count) do
    # word &&& (word - 1) clears the lowest set bit.
    #
    #     word     = 0b10100100
    #     word - 1 = 0b10100011  (borrow propagates through trailing zeros)
    #     AND      = 0b10100000  (lowest set bit cleared)
    do_popcount(word &&& word - 1, count + 1)
  end

  @doc """
  Returns the logical length: the number of addressable bits.

  This is the value passed to `new/1`, or the highest bit index + 1
  after any auto-growth operations.

  ## Examples

      iex> CodingAdventures.Bitset.size(CodingAdventures.Bitset.new(100))
      100
  """
  @spec size(t()) :: non_neg_integer()
  def size(%__MODULE__{len: len}), do: len

  @doc """
  Returns the capacity: the total allocated bits (always a multiple of 64).

  Capacity >= size. The difference (capacity - size) is "slack space" --
  bits that exist in memory but are always zero.

  ## Examples

      iex> CodingAdventures.Bitset.capacity(CodingAdventures.Bitset.new(100))
      128
  """
  @spec capacity(t()) :: non_neg_integer()
  def capacity(%__MODULE__{words: words}), do: length(words) * @bits_per_word

  @doc """
  Returns `true` if at least one bit is set.

  Short-circuits: returns as soon as it finds a non-zero word,
  without scanning the rest.

  ## Examples

      iex> CodingAdventures.Bitset.any?(CodingAdventures.Bitset.new(100))
      false
      iex> CodingAdventures.Bitset.any?(CodingAdventures.Bitset.new(100) |> CodingAdventures.Bitset.set(50))
      true
  """
  @spec any?(t()) :: boolean()
  def any?(%__MODULE__{words: words}) do
    Enum.any?(words, fn w -> w != 0 end)
  end

  @doc """
  Returns `true` if ALL bits in `0..len-1` are set.

  For an empty bitset (`len = 0`), returns `true` -- this is
  **vacuous truth**, the same convention used by Python's `all([])`,
  Rust's `Iterator::all`, and mathematical logic.

  ## How it works

  For each full word (all but the last), we check if every bit is set
  (word == 0xFFFFFFFFFFFFFFFF). For the last word, we only check the
  bits within `len`.

  ## Examples

      iex> CodingAdventures.Bitset.all?(CodingAdventures.Bitset.new(0))
      true
      iex> {:ok, bs} = CodingAdventures.Bitset.from_binary_str("1111")
      iex> CodingAdventures.Bitset.all?(bs)
      true
      iex> {:ok, bs} = CodingAdventures.Bitset.from_binary_str("1110")
      iex> CodingAdventures.Bitset.all?(bs)
      false
  """
  @spec all?(t()) :: boolean()
  def all?(%__MODULE__{len: 0}), do: true

  def all?(%__MODULE__{words: words, len: len}) do
    num_words = length(words)

    # Check all full words (all bits must be 1 = @full_word).
    full_words_ok =
      if num_words > 1 do
        words
        |> Enum.take(num_words - 1)
        |> Enum.all?(fn w -> w == @full_word end)
      else
        true
      end

    if not full_words_ok do
      false
    else
      # Check the last word: only the bits within `len` matter.
      remaining = bit_offset(len)
      last_word = List.last(words)

      if remaining == 0 do
        # len is a multiple of 64, so the last word is a full word.
        last_word == @full_word
      else
        # Create a mask for the valid bits: (1 <<< remaining) - 1
        expected_mask = (1 <<< remaining) - 1
        last_word == expected_mask
      end
    end
  end

  @doc """
  Returns `true` if no bits are set. Equivalent to `not any?/1`.

  ## Examples

      iex> CodingAdventures.Bitset.none?(CodingAdventures.Bitset.new(100))
      true
  """
  @spec none?(t()) :: boolean()
  def none?(%__MODULE__{} = bs), do: not any?(bs)

  # ---------------------------------------------------------------------------
  # Iteration
  # ---------------------------------------------------------------------------

  @doc """
  Returns a list of the indices of all set bits in ascending order.

  ## How it works: trailing-zero-count trick

  For each non-zero word, we use a trailing-zero-count to find the lowest
  set bit, record its index, then clear it with `word &&& (word - 1)`:

      word = 0b10100100   (bits 2, 5, 7 are set)

      Step 1: trailing_zeros = 2  -> record base + 2
              word = word &&& (word - 1) -> 0b10100000

      Step 2: trailing_zeros = 5  -> record base + 5
              word = word &&& (word - 1) -> 0b10000000

      Step 3: trailing_zeros = 7  -> record base + 7
              word = word &&& (word - 1) -> 0b00000000

      word == 0, move to next word.

  This is O(k) where k is the number of set bits, and it skips zero
  words entirely, making it very efficient for sparse bitsets.

  ## Examples

      iex> CodingAdventures.Bitset.set_bits(CodingAdventures.Bitset.from_integer(0b10100101))
      [0, 2, 5, 7]
  """
  @spec set_bits(t()) :: [non_neg_integer()]
  def set_bits(%__MODULE__{words: words, len: len}) do
    words
    |> Enum.with_index()
    |> Enum.flat_map(fn {word, word_idx} ->
      base = word_idx * @bits_per_word
      extract_set_bits(word, base, len, [])
    end)
  end

  # Extract all set bit indices from a single word.
  # Uses the trailing-zero-count trick: find lowest set bit, record it,
  # clear it, repeat until word is zero.
  defp extract_set_bits(0, _base, _len, acc), do: Enum.reverse(acc)

  defp extract_set_bits(word, base, len, acc) do
    # Find the position of the lowest set bit.
    # trailing_zeros counts how many zeros are at the bottom.
    bit_pos = trailing_zeros(word)
    index = base + bit_pos

    if index >= len do
      # Beyond the logical length -- stop.
      Enum.reverse(acc)
    else
      # Clear the lowest set bit: word &&& (word - 1)
      new_word = word &&& word - 1
      extract_set_bits(new_word, base, len, [index | acc])
    end
  end

  # Count trailing zeros in a 64-bit word.
  # This finds the position of the lowest set bit.
  #
  #     trailing_zeros(0b10100100) = 2  (bit 2 is the lowest set bit)
  #     trailing_zeros(0b10000000) = 7  (bit 7 is the lowest set bit)
  #     trailing_zeros(1)          = 0  (bit 0 is set)
  #
  # For word == 0, this would loop forever, but we guard against that
  # in the caller (extract_set_bits checks for 0 first).
  defp trailing_zeros(word) do
    do_trailing_zeros(word, 0)
  end

  defp do_trailing_zeros(word, count) do
    if (word &&& 1) == 1 do
      count
    else
      do_trailing_zeros(word >>> 1, count + 1)
    end
  end

  # ---------------------------------------------------------------------------
  # Conversion operations
  # ---------------------------------------------------------------------------

  @doc """
  Convert the bitset to a non-negative integer.

  Bit 0 of the bitset becomes the least significant bit of the integer.
  Returns 0 for an empty bitset.

  Elixir has arbitrary precision integers, so there is no overflow concern
  -- any bitset can be converted to an integer regardless of size.

  ## How it works

  We walk the words from last to first, shifting each word into position:

      result = words[n-1]
      result = (result <<< 64) ||| words[n-2]
      ...
      result = (result <<< 64) ||| words[0]

  ## Examples

      iex> CodingAdventures.Bitset.to_integer(CodingAdventures.Bitset.from_integer(42))
      42
      iex> CodingAdventures.Bitset.to_integer(CodingAdventures.Bitset.new(0))
      0
  """
  @spec to_integer(t()) :: non_neg_integer()
  def to_integer(%__MODULE__{words: []}), do: 0

  def to_integer(%__MODULE__{words: words}) do
    words
    |> Enum.reverse()
    |> Enum.reduce(0, fn word, acc ->
      (acc <<< @bits_per_word) ||| word
    end)
  end

  @doc """
  Convert the bitset to a binary string with the highest bit on the left.

  This is the inverse of `from_binary_str/1`. An empty bitset produces
  an empty string `""`.

  ## Examples

      iex> CodingAdventures.Bitset.to_binary_str(CodingAdventures.Bitset.from_integer(5))
      "101"
      iex> CodingAdventures.Bitset.to_binary_str(CodingAdventures.Bitset.new(0))
      ""
  """
  @spec to_binary_str(t()) :: String.t()
  def to_binary_str(%__MODULE__{len: 0}), do: ""

  def to_binary_str(%__MODULE__{} = bs) do
    # Build the string from the highest bit (len-1) down to bit 0.
    # This produces conventional binary notation: MSB on the left.
    (bs.len - 1)..0//-1
    |> Enum.map(fn i ->
      if test?(bs, i), do: "1", else: "0"
    end)
    |> Enum.join()
  end

  # ---------------------------------------------------------------------------
  # Equality
  # ---------------------------------------------------------------------------

  @doc """
  Returns `true` if two bitsets have the same `len` and the same bits set.

  Capacity is irrelevant to equality -- a bitset with `capacity = 128`
  can equal one with `capacity = 256` if their `len` and set bits match.

  ## Examples

      iex> a = CodingAdventures.Bitset.from_integer(5)
      iex> b = CodingAdventures.Bitset.from_integer(5)
      iex> CodingAdventures.Bitset.equal?(a, b)
      true
  """
  @spec equal?(t(), t()) :: boolean()
  def equal?(%__MODULE__{len: len_a} = a, %__MODULE__{len: len_b} = b) do
    if len_a != len_b do
      false
    else
      # Compare word-by-word. If one has more words allocated, the
      # extra words must all be zero (due to clean-trailing-bits).
      max_words = max(length(a.words), length(b.words))

      Enum.all?(0..max(max_words - 1, 0)//1, fn i ->
        word_at(a.words, i) == word_at(b.words, i)
      end)
    end
  end
end

# ---------------------------------------------------------------------------
# Protocol implementations
# ---------------------------------------------------------------------------
#
# We implement String.Chars (for to_string/1 and string interpolation)
# and Inspect (for iex display) to make bitsets easy to work with in
# the REPL and in logging.

defimpl String.Chars, for: CodingAdventures.Bitset do
  @moduledoc """
  Converts a Bitset to a human-readable string like "Bitset(101)".

  This is used by `to_string/1` and string interpolation (`\#{bitset}`).
  """
  def to_string(bitset) do
    "Bitset(#{CodingAdventures.Bitset.to_binary_str(bitset)})"
  end
end

defimpl Inspect, for: CodingAdventures.Bitset do
  @moduledoc """
  Custom inspect representation for Bitsets.

  Instead of showing the raw struct fields (which would display the
  internal word list), we show a readable format like:

      #Bitset<101, len=3>

  This makes iex output much more useful.
  """
  def inspect(bitset, _opts) do
    binary_str = CodingAdventures.Bitset.to_binary_str(bitset)
    len = CodingAdventures.Bitset.size(bitset)
    "#Bitset<#{binary_str}, len=#{len}>"
  end
end
