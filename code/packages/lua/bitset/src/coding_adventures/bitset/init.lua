-- coding_adventures.bitset
-- ============================================================================
--
-- A COMPACT BITSET PACKED INTO 64-BIT INTEGERS
--
-- A bitset (also called a bit array or bit vector) is a data structure that
-- compactly stores a sequence of boolean values — each bit is either 0 (false)
-- or 1 (true). Instead of using one full byte per boolean (as a plain Lua
-- table would), we pack 64 booleans into each 64-bit integer word.
--
-- Why 64 bits per word?
--   Lua 5.4 uses 64-bit signed integers natively. The bitwise operators
--   (&, |, ~, <<, >>) all operate on 64-bit integers, so 64 bits is our
--   natural "word size".
--
-- Memory layout:
--   words[1]  = bits  0 .. 63
--   words[2]  = bits 64 .. 127
--   words[3]  = bits 128 .. 191
--   ...
--
-- A bitset table looks like:
--   { words = {w1, w2, ...}, len = N }
-- where `len` is the logical capacity (number of addressable bit positions).
--
-- This module uses a FUNCTIONAL (immutable) style: every operation that would
-- mutate a bitset instead returns a NEW bitset, leaving the original unchanged.
-- This is safer, easier to reason about, and plays nicely with Lua's garbage
-- collector.
--
-- Lua integer notes:
--   Lua integers are 64-bit SIGNED. The bit patterns are the same as unsigned;
--   only the interpretation of the most-significant bit differs when printing.
--   All our bitwise operations are correct regardless of sign.
--
-- Usage:
--   local Bitset = require("coding_adventures.bitset")
--   local bs = Bitset.new(100)
--   bs = Bitset.set(bs, 42)
--   print(Bitset.test(bs, 42))   -- true
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Constants
-- ---------------------------------------------------------------------------

-- Each Lua integer holds 64 bits.
local BITS_PER_WORD = 64

-- ---------------------------------------------------------------------------
-- Internal helpers
-- ---------------------------------------------------------------------------

-- deep_copy_words: returns a shallow copy of the words array.
-- Since each element is an integer (value type), a shallow copy is sufficient.
local function deep_copy_words(words)
    local copy = {}
    for i = 1, #words do
        copy[i] = words[i]
    end
    return copy
end

-- word_count: how many words do we need to store `n` bits?
--   e.g. n=64  → 1 word
--        n=65  → 2 words
--        n=1   → 1 word
local function word_count(n)
    if n == 0 then return 0 end
    return math.floor((n - 1) / BITS_PER_WORD) + 1
end

-- ---------------------------------------------------------------------------
-- Constructor
-- ---------------------------------------------------------------------------

-- Bitset.new(len)
--   Create a new bitset capable of addressing bit indices 0 .. len-1.
--   All bits start as 0 (false).
--
--   Parameters:
--     len (integer) — the logical size of the bitset.
--
--   Returns:
--     A new bitset table: { words = {...}, len = len }
function M.new(len)
    assert(type(len) == "number" and len >= 0 and math.floor(len) == len,
        "Bitset.new: len must be a non-negative integer")
    local nw = word_count(len)
    local words = {}
    for i = 1, nw do
        words[i] = 0
    end
    return { words = words, len = len }
end

-- ---------------------------------------------------------------------------
-- Bit addressing helpers
-- ---------------------------------------------------------------------------

-- For bit index `i` (0-based):
--   word_idx = floor(i / 64) + 1   (1-based Lua table index)
--   bit_off  = i mod 64            (which bit within that word)
--   mask     = 1 << bit_off        (a word with only that bit set)
--
-- Example: bit 70
--   word_idx = floor(70/64) + 1 = 2
--   bit_off  = 70 mod 64 = 6
--   mask     = 1 << 6 = 64

local function word_idx(i)
    return math.floor(i / BITS_PER_WORD) + 1
end

local function bit_off(i)
    return i % BITS_PER_WORD
end

local function mask(i)
    return 1 << bit_off(i)
end

-- ---------------------------------------------------------------------------
-- Core operations
-- ---------------------------------------------------------------------------

-- Bitset.set(bs, i)
--   Return a new bitset with bit `i` set to 1.
--   If `i >= bs.len`, the bitset is grown to accommodate the new bit.
--
--   The key idea: to set bit `b` in word `w`, we do:
--     w = w | mask(b)
--   The OR sets the target bit without disturbing any other bit.
function M.set(bs, i)
    assert(type(i) == "number" and i >= 0 and math.floor(i) == i,
        "Bitset.set: i must be a non-negative integer")
    local new_len = bs.len
    local new_words = deep_copy_words(bs.words)
    -- Grow if needed
    if i >= new_len then
        new_len = i + 1
        local needed = word_count(new_len)
        while #new_words < needed do
            new_words[#new_words + 1] = 0
        end
    end
    local wi = word_idx(i)
    new_words[wi] = new_words[wi] | mask(i)
    return { words = new_words, len = new_len }
end

-- Bitset.clear(bs, i)
--   Return a new bitset with bit `i` set to 0.
--
--   To clear bit `b` in word `w`:
--     w = w & (~mask(b))
--   The NOT flips all bits of the mask so all bits are 1 except position b.
--   The AND then clears just that position.
--
--   In Lua 5.4, `~x` is bitwise NOT (returns a 64-bit integer with all bits
--   of x flipped). So ~mask(i) has a 0 exactly at position bit_off(i) and
--   1s everywhere else.
function M.clear(bs, i)
    assert(type(i) == "number" and i >= 0 and math.floor(i) == i,
        "Bitset.clear: i must be a non-negative integer")
    local new_words = deep_copy_words(bs.words)
    local wi = word_idx(i)
    if wi <= #new_words then
        new_words[wi] = new_words[wi] & (~mask(i))
    end
    return { words = new_words, len = bs.len }
end

-- Bitset.test(bs, i)
--   Return true if bit `i` is set, false otherwise.
--
--   To test bit `b` in word `w`:
--     (w & mask(b)) ~= 0
--   The AND isolates the target bit; if the result is non-zero, the bit is set.
function M.test(bs, i)
    assert(type(i) == "number" and i >= 0 and math.floor(i) == i,
        "Bitset.test: i must be a non-negative integer")
    local wi = word_idx(i)
    if wi > #bs.words then
        return false
    end
    return (bs.words[wi] & mask(i)) ~= 0
end

-- Bitset.size(bs)
--   Return the logical size (number of addressable bit positions).
function M.size(bs)
    return bs.len
end

-- ---------------------------------------------------------------------------
-- Popcount (population count — count of set bits)
-- ---------------------------------------------------------------------------

-- Bitset.popcount(bs)
--   Count the total number of bits that are set to 1 across all words.
--
--   We use Brian Kernighan's bit-counting trick for each word:
--
--     while w ~= 0 do
--       w = w & (w - 1)   -- clear the lowest set bit
--       count = count + 1
--     end
--
--   Why does `w & (w-1)` clear the lowest set bit?
--   Consider w = ...1000 (lowest set bit at position k).
--   Then w-1 = ...0111 (all bits below k become 1, bit k becomes 0).
--   So w & (w-1) = ...0000 (clears bit k, keeps higher bits intact).
--
--   This loop runs exactly once per set bit, which is efficient when the
--   bitset is sparse (few bits set).
--
--   NOTE: Lua 5.4 integers are 64-bit signed, so `w` can be negative when
--   the high bit (bit 63) is set. The arithmetic `w & (w-1)` still works
--   correctly as two's-complement arithmetic.
function M.popcount(bs)
    local total = 0
    for _, w in ipairs(bs.words) do
        -- We must handle the case where w is negative (bit 63 set).
        -- In Lua 5.4, integers are 64-bit signed. The loop terminates because
        -- each iteration clears one bit, and eventually w becomes 0.
        local ww = w
        while ww ~= 0 do
            ww = ww & (ww - 1)
            total = total + 1
        end
    end
    return total
end

-- ---------------------------------------------------------------------------
-- Set-bit iteration
-- ---------------------------------------------------------------------------

-- Bitset.set_bits(bs)
--   Return a list (array) of all bit indices that are currently set to 1,
--   in ascending order.
--
--   Example: if bits 0, 42, and 99 are set, returns {0, 42, 99}.
--
--   Algorithm: for each word, iterate over all 64 bit positions and collect
--   those that are set. (We could use Kernighan's trick here too, but the
--   straightforward scan is clearer for literate readers.)
function M.set_bits(bs)
    local result = {}
    for wi, w in ipairs(bs.words) do
        if w ~= 0 then
            for b = 0, BITS_PER_WORD - 1 do
                local global_idx = (wi - 1) * BITS_PER_WORD + b
                if global_idx < bs.len then
                    if (w & (1 << b)) ~= 0 then
                        result[#result + 1] = global_idx
                    end
                end
            end
        end
    end
    return result
end

-- ---------------------------------------------------------------------------
-- Bitwise operations between two bitsets
-- ---------------------------------------------------------------------------

-- Helper: ensure two bitsets have the same number of words for binary ops.
-- Returns two word arrays of equal length (padding with zeros on the right).
local function align_words(a, b)
    local na = #a.words
    local nb = #b.words
    local n = math.max(na, nb)
    local wa, wb = {}, {}
    for i = 1, n do
        wa[i] = a.words[i] or 0
        wb[i] = b.words[i] or 0
    end
    return wa, wb, n, math.max(a.len, b.len)
end

-- Bitset.bitwise_and(a, b)
--   Return a new bitset where bit i is set iff bit i is set in BOTH a and b.
--
--   Performed word-by-word: result.words[i] = a.words[i] & b.words[i]
function M.bitwise_and(a, b)
    local wa, wb, n, new_len = align_words(a, b)
    local new_words = {}
    for i = 1, n do
        new_words[i] = wa[i] & wb[i]
    end
    return { words = new_words, len = new_len }
end

-- Bitset.bitwise_or(a, b)
--   Return a new bitset where bit i is set iff bit i is set in a OR b (or both).
function M.bitwise_or(a, b)
    local wa, wb, n, new_len = align_words(a, b)
    local new_words = {}
    for i = 1, n do
        new_words[i] = wa[i] | wb[i]
    end
    return { words = new_words, len = new_len }
end

-- Bitset.bitwise_xor(a, b)
--   Return a new bitset where bit i is set iff bit i is set in EXACTLY ONE of a, b.
--   (eXclusive OR: set if different, clear if same)
function M.bitwise_xor(a, b)
    local wa, wb, n, new_len = align_words(a, b)
    local new_words = {}
    for i = 1, n do
        new_words[i] = wa[i] ~ wb[i]
    end
    return { words = new_words, len = new_len }
end

-- ---------------------------------------------------------------------------

return M
