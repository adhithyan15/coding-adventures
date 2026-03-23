// lib.rs -- Bitset: A Compact Boolean Array Packed into 64-bit Words
// ==================================================================
//
// A bitset stores a sequence of bits -- each one either 0 or 1 -- packed
// into machine-word-sized integers (`u64`). Instead of using an entire byte
// to represent a single true/false value, a bitset packs 64 of them into
// a single word.
//
// Why does this matter?
//
// 1. **Space**: 10,000 booleans as `Vec<bool>` = 10,000 bytes.
//    As a bitset = ~1,250 bytes. That's an 8x improvement (64x vs Python).
//
// 2. **Speed**: AND-ing two boolean arrays loops over 10,000 elements.
//    AND-ing two bitsets loops over ~157 words. The CPU performs a single
//    64-bit AND instruction on each word, operating on 64 bits at once.
//
// 3. **Ubiquity**: Bitsets appear in Bloom filters, register allocators,
//    graph algorithms (visited sets), database bitmap indexes, filesystem
//    free-block bitmaps, network subnet masks, and garbage collectors.
//
// Bit Ordering: LSB-First
// -----------------------
//
// We use Least Significant Bit first ordering. Bit 0 is the least significant
// bit of word 0. Bit 63 is the most significant bit of word 0. Bit 64 is the
// least significant bit of word 1. And so on.
//
//     Word 0                              Word 1
//     ┌─────────────────────────────┐     ┌─────────────────────────────┐
//     │ bit 63  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64 │
//     └─────────────────────────────┘     └─────────────────────────────┘
//     MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
//
// The three fundamental formulas that drive every bitset operation:
//
//     word_index = i / 64       (which word contains bit i?)
//     bit_offset = i % 64       (which position within that word?)
//     bitmask    = 1u64 << (i % 64)  (a mask with only bit i set)
//
// These are the heart of the entire implementation.

use std::fmt;
use std::ops::{BitAnd, BitOr, BitXor, Not};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
//
// BITS_PER_WORD is 64 because we use u64 as our word type. Every formula in
// this module uses this constant rather than a magic number, so if someone
// ever wanted to experiment with u32 words (32 bits), they'd only need to
// change this constant and the word type.

/// Number of bits stored in each word of the bitset.
const BITS_PER_WORD: usize = 64;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------
//
// We have exactly one error variant: an invalid binary string was passed to
// `from_binary_str`. This keeps the error type minimal and focused.
// We implement Display and std::error::Error manually to avoid external
// dependencies -- this package has zero deps.

/// Errors that can occur when constructing a bitset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitsetError {
    /// The input string contained characters other than '0' and '1'.
    ///
    /// The `String` payload contains the offending input so error messages
    /// can show exactly what was wrong.
    InvalidBinaryString(String),
}

impl fmt::Display for BitsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitsetError::InvalidBinaryString(s) => {
                write!(f, "invalid binary string: {:?}", s)
            }
        }
    }
}

impl std::error::Error for BitsetError {}

// ---------------------------------------------------------------------------
// The Bitset struct
// ---------------------------------------------------------------------------
//
// Internal Representation
// ~~~~~~~~~~~~~~~~~~~~~~~
//
// We store bits in a Vec<u64> called `words`. Each u64 holds 64 bits.
// We also track `len`, the logical size -- the number of bits the user
// considers "addressable". The capacity is always words.len() * 64.
//
//     ┌──────────────────────────────────────────────────────────────────┐
//     │                          capacity (256 bits = 4 words)           │
//     │                                                                  │
//     │  ┌──────────────────────────────────────────┐                    │
//     │  │              len (200 bits)                │ ··· unused ····  │
//     │  │  (highest addressable bit index + 1)       │ (always zero)   │
//     │  └──────────────────────────────────────────┘                    │
//     └──────────────────────────────────────────────────────────────────┘
//
// **Clean-trailing-bits invariant**: Bits beyond `len` in the last word are
// always zero. This is critical for correctness of popcount, any, all, none,
// equality, and to_integer. Every operation that modifies the last word must
// clean trailing bits afterwards.

/// A compact bitset that packs boolean values into 64-bit words.
///
/// `Bitset` provides O(n/64) bulk bitwise operations (AND, OR, XOR, NOT),
/// efficient iteration over set bits using trailing-zero-count, and
/// ArrayList-style automatic growth when you set bits beyond the current size.
///
/// # Examples
///
/// ```
/// use bitset::Bitset;
///
/// // Create a bitset and set some bits
/// let mut bs = Bitset::new(100);
/// bs.set(0);
/// bs.set(42);
/// bs.set(99);
/// assert_eq!(bs.popcount(), 3);
///
/// // Iterate over set bits
/// let bits: Vec<usize> = bs.iter_set_bits().collect();
/// assert_eq!(bits, vec![0, 42, 99]);
///
/// // Bulk operations return new bitsets
/// let mut other = Bitset::new(100);
/// other.set(42);
/// other.set(50);
/// let intersection = bs.and(&other);
/// assert_eq!(intersection.popcount(), 1); // only bit 42
/// ```
#[derive(Debug, Clone)]
pub struct Bitset {
    /// The packed bit storage. Each u64 holds 64 bits.
    /// words[0] holds bits 0-63, words[1] holds bits 64-127, etc.
    words: Vec<u64>,

    /// The logical size: the number of bits the user considers addressable.
    /// Bits 0 through len-1 are "real". Bits from len to capacity-1 exist
    /// in memory but are always zero (the clean-trailing-bits invariant).
    len: usize,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------
//
// These small utility functions compute the word index, bit offset, and
// number of words needed for a given bit count. They're used throughout
// the implementation.

/// How many u64 words do we need to store `bit_count` bits?
///
/// This is ceiling division: (bit_count + 63) / 64.
///
/// ```text
/// words_needed(0)   = 0   (no bits, no words)
/// words_needed(1)   = 1   (1 bit needs 1 word)
/// words_needed(64)  = 1   (64 bits fit exactly in 1 word)
/// words_needed(65)  = 2   (65 bits need 2 words)
/// words_needed(128) = 2   (128 bits fit exactly in 2 words)
/// words_needed(200) = 4   (200 bits need ceil(200/64) = 4 words)
/// ```
fn words_needed(bit_count: usize) -> usize {
    // Integer ceiling division without overflow risk:
    // (bit_count + BITS_PER_WORD - 1) / BITS_PER_WORD
    (bit_count + BITS_PER_WORD - 1) / BITS_PER_WORD
}

/// Which word contains bit `i`? Simply i / 64.
///
/// ```text
/// word_index(0)   = 0   (bit 0 is in word 0)
/// word_index(63)  = 0   (bit 63 is the last bit of word 0)
/// word_index(64)  = 1   (bit 64 is the first bit of word 1)
/// word_index(137) = 2   (bit 137 is in word 2)
/// ```
fn word_index(i: usize) -> usize {
    i / BITS_PER_WORD
}

/// Which bit position within its word does bit `i` occupy? Simply i % 64.
///
/// ```text
/// bit_offset(0)   = 0
/// bit_offset(63)  = 63
/// bit_offset(64)  = 0   (first bit of the next word)
/// bit_offset(137) = 9   (137 - 2*64 = 9)
/// ```
fn bit_offset(i: usize) -> usize {
    i % BITS_PER_WORD
}

/// A bitmask with only bit `i` set within its word.
///
/// This is `1u64 << (i % 64)`. We use this mask to isolate, set, clear,
/// or toggle a single bit within a word using bitwise operations:
///
/// ```text
/// To set bit i:    word |= bitmask(i)     (OR with mask turns bit on)
/// To clear bit i:  word &= !bitmask(i)    (AND with inverted mask turns bit off)
/// To test bit i:   (word & bitmask(i)) != 0  (AND with mask isolates the bit)
/// To toggle bit i: word ^= bitmask(i)     (XOR with mask flips the bit)
/// ```
fn bitmask(i: usize) -> u64 {
    1u64 << bit_offset(i)
}

impl Bitset {
    // ------------------------------------------------------------------
    // Constructors
    // ------------------------------------------------------------------

    /// Create a new bitset with all bits initially zero.
    ///
    /// The `size` parameter sets the logical length (`len`). The capacity
    /// is rounded up to the next multiple of 64.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::new(100);
    /// assert_eq!(bs.len(), 100);
    /// assert_eq!(bs.capacity(), 128);  // 2 words * 64 bits/word
    /// assert_eq!(bs.popcount(), 0);    // all bits start as zero
    /// ```
    ///
    /// `new(0)` is valid and creates an empty bitset:
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::new(0);
    /// assert_eq!(bs.len(), 0);
    /// assert_eq!(bs.capacity(), 0);
    /// ```
    pub fn new(size: usize) -> Self {
        Bitset {
            words: vec![0u64; words_needed(size)],
            len: size,
        }
    }

    /// Create a bitset from a non-negative integer.
    ///
    /// Bit 0 of the bitset is the least significant bit of `value`.
    /// The `len` of the result is the position of the highest set bit + 1.
    /// If `value == 0`, then `len = 0`.
    ///
    /// We accept `u128` to support values larger than a single u64 word.
    ///
    /// # How it works
    ///
    /// We split the u128 into its low 64 bits and high 64 bits:
    ///
    /// ```text
    /// value = 0x0000_0000_0000_0005  (decimal 5, binary 101)
    /// low   = 5   (bits 0-63)
    /// high  = 0   (bits 64-127)
    /// ```
    ///
    /// Then we figure out `len` by finding the highest set bit. For a u128,
    /// that's `128 - leading_zeros`. If the value is 0, len is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::from_integer(5);  // binary: 101
    /// assert_eq!(bs.len(), 3);           // highest bit is position 2
    /// assert!(bs.test(0));               // bit 0 = 1
    /// assert!(!bs.test(1));              // bit 1 = 0
    /// assert!(bs.test(2));               // bit 2 = 1
    /// ```
    pub fn from_integer(value: u128) -> Self {
        // Special case: zero produces an empty bitset.
        if value == 0 {
            return Bitset::new(0);
        }

        // Split the u128 into two u64 halves.
        //
        //     u128 value:  [  high 64 bits  |  low 64 bits  ]
        //                  bits 127-64        bits 63-0
        let low = value as u64; // truncates to lower 64 bits
        let high = (value >> 64) as u64; // shifts upper 64 bits down

        // The logical length is the position of the highest set bit + 1.
        // For a u128, this is 128 - leading_zeros.
        let len = (128 - value.leading_zeros()) as usize;

        let mut words = vec![low];
        if high != 0 {
            words.push(high);
        }

        Bitset { words, len }
    }

    /// Create a bitset from a binary string like `"1010"`.
    ///
    /// The leftmost character is the highest-indexed bit (conventional binary
    /// notation, matching how humans write numbers). The rightmost character
    /// is bit 0.
    ///
    /// # String-to-bits mapping
    ///
    /// ```text
    /// Input string: "1 0 1 0"
    /// Position:      3 2 1 0    (leftmost = highest bit index)
    ///
    /// Bit 0 = '0' (rightmost char)
    /// Bit 1 = '1'
    /// Bit 2 = '0'
    /// Bit 3 = '1' (leftmost char)
    ///
    /// This is the same as the integer 10 (binary 1010).
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `BitsetError::InvalidBinaryString` if the string contains
    /// any character other than '0' or '1'.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::from_binary_str("1010").unwrap();
    /// assert_eq!(bs.len(), 4);
    /// assert!(bs.test(1));   // bit 1 = '1'
    /// assert!(bs.test(3));   // bit 3 = '1'
    /// assert!(!bs.test(0));  // bit 0 = '0'
    /// ```
    pub fn from_binary_str(s: &str) -> Result<Self, BitsetError> {
        // Validate: every character must be '0' or '1'.
        if !s.chars().all(|c| c == '0' || c == '1') {
            return Err(BitsetError::InvalidBinaryString(s.to_string()));
        }

        // Empty string produces an empty bitset.
        if s.is_empty() {
            return Ok(Bitset::new(0));
        }

        // The string length is the logical len of the bitset.
        let len = s.len();
        let mut bs = Bitset::new(len);

        // Walk the string from right to left (LSB to MSB).
        // The rightmost character (index s.len()-1) is bit 0.
        // The leftmost character (index 0) is bit s.len()-1.
        for (char_idx, ch) in s.chars().rev().enumerate() {
            if ch == '1' {
                // char_idx is the bit index (0 = rightmost = LSB).
                let wi = word_index(char_idx);
                bs.words[wi] |= bitmask(char_idx);
            }
        }

        // Clean trailing bits. The from_binary_str might set bits beyond len
        // if len is not a multiple of 64, but since we only set bits within
        // 0..len, this is already clean. Still, let's be defensive.
        bs.clean_trailing_bits();

        Ok(bs)
    }

    // ------------------------------------------------------------------
    // Single-bit operations
    // ------------------------------------------------------------------
    //
    // These are the bread-and-butter operations: set a bit, clear a bit,
    // test whether a bit is set, toggle a bit. Each one translates to a
    // single bitwise operation on the containing word.
    //
    // Growth semantics:
    //   - set(i) and toggle(i) AUTO-GROW the bitset if i >= len.
    //   - test(i) and clear(i) do NOT grow. They return false / do nothing
    //     for out-of-range indices. This is safe because unallocated bits
    //     are conceptually zero.

    /// Set bit `i` to 1. Auto-grows the bitset if `i >= len`.
    ///
    /// # How auto-growth works
    ///
    /// If `i` is beyond the current capacity, we double the capacity
    /// repeatedly until it's large enough (with a minimum of 64 bits).
    /// This is the same amortized O(1) strategy used by `Vec`, `ArrayList`,
    /// and Python's `list`.
    ///
    /// ```text
    /// Before: len=100, capacity=128 (2 words)
    /// set(200): 200 >= 128, so double: 128 -> 256. Now 200 < 256.
    /// After: len=201, capacity=256 (4 words)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let mut bs = Bitset::new(10);
    /// bs.set(5);
    /// assert!(bs.test(5));
    ///
    /// // Auto-growth:
    /// bs.set(100);  // grows from len=10 to len=101
    /// assert_eq!(bs.len(), 101);
    /// assert!(bs.test(100));
    /// ```
    pub fn set(&mut self, i: usize) {
        self.ensure_capacity(i);
        // The core operation: OR the bitmask into the word.
        //
        //     words[2] = 0b...0000_0000
        //     mask     = 0b...0010_0000   (bit 5 within the word)
        //     result   = 0b...0010_0000   (bit 5 is now set)
        //
        // OR is idempotent: setting an already-set bit is a no-op.
        self.words[word_index(i)] |= bitmask(i);
    }

    /// Set bit `i` to 0. No-op if `i >= len` (does not grow).
    ///
    /// Clearing a bit that's already 0 is a no-op. Clearing a bit beyond
    /// the bitset's length is also a no-op -- there's nothing to clear,
    /// because unallocated bits are conceptually zero.
    ///
    /// # How it works
    ///
    /// We AND the word with the inverted bitmask. The inverted mask has all
    /// bits set EXCEPT the target bit, so every other bit is preserved:
    ///
    /// ```text
    /// words[2] = 0b...0010_0100   (bits 2 and 5 set)
    /// mask     = 0b...0010_0000   (bit 5)
    /// !mask    = 0b...1101_1111   (everything except bit 5)
    /// result   = 0b...0000_0100   (bit 5 cleared, bit 2 preserved)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let mut bs = Bitset::new(10);
    /// bs.set(5);
    /// assert!(bs.test(5));
    /// bs.clear(5);
    /// assert!(!bs.test(5));
    ///
    /// // Clearing beyond len is a no-op:
    /// bs.clear(999);  // no panic, no growth
    /// assert_eq!(bs.len(), 10);
    /// ```
    pub fn clear(&mut self, i: usize) {
        if i >= self.len {
            return; // out of range: nothing to clear
        }
        self.words[word_index(i)] &= !bitmask(i);
    }

    /// Test whether bit `i` is set. Returns `false` if `i >= len`.
    ///
    /// This is a pure read operation -- it never modifies the bitset.
    /// Testing a bit beyond the bitset's length returns false because
    /// unallocated bits are conceptually zero.
    ///
    /// # How it works
    ///
    /// We AND the word with the bitmask. If the result is non-zero, the
    /// bit is set:
    ///
    /// ```text
    /// words[2] = 0b...0010_0100   (bits 2 and 5 set)
    /// mask     = 0b...0010_0000   (bit 5)
    /// result   = 0b...0010_0000   (non-zero -> bit 5 is set)
    ///
    /// mask     = 0b...0000_1000   (bit 3)
    /// result   = 0b...0000_0000   (zero -> bit 3 is not set)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let mut bs = Bitset::new(10);
    /// bs.set(5);
    /// assert!(bs.test(5));
    /// assert!(!bs.test(3));
    /// assert!(!bs.test(999));  // beyond len → false
    /// ```
    pub fn test(&self, i: usize) -> bool {
        if i >= self.len {
            return false; // out of range: conceptually zero
        }
        (self.words[word_index(i)] & bitmask(i)) != 0
    }

    /// Toggle (flip) bit `i`. Auto-grows if `i >= len`.
    ///
    /// If the bit is 0, it becomes 1. If it's 1, it becomes 0.
    ///
    /// # How it works
    ///
    /// XOR with the bitmask flips exactly one bit:
    ///
    /// ```text
    /// words[2] = 0b...0010_0100   (bits 2 and 5 set)
    /// mask     = 0b...0010_0000   (bit 5)
    /// result   = 0b...0000_0100   (bit 5 flipped to 0)
    ///
    /// words[2] = 0b...0000_0100   (only bit 2 set)
    /// mask     = 0b...0010_0000   (bit 5)
    /// result   = 0b...0010_0100   (bit 5 flipped to 1)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let mut bs = Bitset::new(10);
    /// bs.toggle(5);       // 0 → 1
    /// assert!(bs.test(5));
    /// bs.toggle(5);       // 1 → 0
    /// assert!(!bs.test(5));
    /// ```
    pub fn toggle(&mut self, i: usize) {
        self.ensure_capacity(i);
        self.words[word_index(i)] ^= bitmask(i);

        // Toggle might have set a bit in the last word's trailing region.
        // For example, if len=5 and we toggle bit 7 (which grew the bitset
        // to len=8, capacity=64), the trailing bits above bit 7 are fine.
        // But if len was already 64 and we toggle bit 3, we need to make
        // sure we haven't accidentally set trailing bits. In practice,
        // ensure_capacity already handles growth, and toggling a bit within
        // len doesn't create trailing-bit issues. But after growth, the
        // new len might not be a multiple of 64, so we clean just in case.
        self.clean_trailing_bits();
    }

    // ------------------------------------------------------------------
    // Bulk bitwise operations
    // ------------------------------------------------------------------
    //
    // All bulk operations return a NEW bitset. They don't modify either
    // operand. The result has len = max(a.len, b.len).
    //
    // When two bitsets have different lengths, the shorter one is
    // "zero-extended" conceptually. In practice, we just stop reading
    // from the shorter one's words once they run out and treat missing
    // words as zero.
    //
    // Performance: each operation processes one 64-bit word per loop
    // iteration, so 64 bits are handled in a single CPU instruction.
    // This is the fundamental performance advantage of bitsets.

    /// Bitwise AND: result bit is 1 only if BOTH input bits are 1.
    ///
    /// # Truth table
    ///
    /// ```text
    /// A  B  A&B
    /// 0  0   0
    /// 0  1   0
    /// 1  0   0
    /// 1  1   1
    /// ```
    ///
    /// AND is used for **intersection**: elements that are in both sets.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let a = Bitset::from_integer(0b1100);  // bits 2,3
    /// let b = Bitset::from_integer(0b1010);  // bits 1,3
    /// let c = a.and(&b);
    /// assert_eq!(c.to_integer(), Some(0b1000));  // only bit 3
    /// ```
    pub fn and(&self, other: &Bitset) -> Bitset {
        let result_len = self.len.max(other.len);
        let max_words = self.words.len().max(other.words.len());
        let mut result_words = Vec::with_capacity(max_words);

        for i in 0..max_words {
            // If one bitset is shorter, its missing words are zero.
            // AND with zero produces zero, which is correct.
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            result_words.push(a & b);
        }

        let mut result = Bitset {
            words: result_words,
            len: result_len,
        };
        result.clean_trailing_bits();
        result
    }

    /// Bitwise OR: result bit is 1 if EITHER (or both) input bits are 1.
    ///
    /// # Truth table
    ///
    /// ```text
    /// A  B  A|B
    /// 0  0   0
    /// 0  1   1
    /// 1  0   1
    /// 1  1   1
    /// ```
    ///
    /// OR is used for **union**: elements that are in either set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let a = Bitset::from_integer(0b1100);  // bits 2,3
    /// let b = Bitset::from_integer(0b1010);  // bits 1,3
    /// let c = a.or(&b);
    /// assert_eq!(c.to_integer(), Some(0b1110));  // bits 1,2,3
    /// ```
    pub fn or(&self, other: &Bitset) -> Bitset {
        let result_len = self.len.max(other.len);
        let max_words = self.words.len().max(other.words.len());
        let mut result_words = Vec::with_capacity(max_words);

        for i in 0..max_words {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            result_words.push(a | b);
        }

        let mut result = Bitset {
            words: result_words,
            len: result_len,
        };
        result.clean_trailing_bits();
        result
    }

    /// Bitwise XOR: result bit is 1 if the input bits DIFFER.
    ///
    /// # Truth table
    ///
    /// ```text
    /// A  B  A^B
    /// 0  0   0
    /// 0  1   1
    /// 1  0   1
    /// 1  1   0
    /// ```
    ///
    /// XOR is used for **symmetric difference**: elements in either set
    /// but not both.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let a = Bitset::from_integer(0b1100);  // bits 2,3
    /// let b = Bitset::from_integer(0b1010);  // bits 1,3
    /// let c = a.xor(&b);
    /// assert_eq!(c.to_integer(), Some(0b0110));  // bits 1,2
    /// ```
    pub fn xor(&self, other: &Bitset) -> Bitset {
        let result_len = self.len.max(other.len);
        let max_words = self.words.len().max(other.words.len());
        let mut result_words = Vec::with_capacity(max_words);

        for i in 0..max_words {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            result_words.push(a ^ b);
        }

        let mut result = Bitset {
            words: result_words,
            len: result_len,
        };
        result.clean_trailing_bits();
        result
    }

    /// Bitwise NOT: flip every bit within `len`.
    ///
    /// # Truth table
    ///
    /// ```text
    /// A  ~A
    /// 0   1
    /// 1   0
    /// ```
    ///
    /// NOT is used for **complement**: elements NOT in the set.
    ///
    /// **Important**: NOT flips bits within `len`, NOT within `capacity`.
    /// Bits beyond `len` remain zero (clean-trailing-bits invariant).
    /// The result has the same `len` as the input.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let a = Bitset::from_integer(0b1010);  // len=4, bits 1,3 set
    /// let b = a.not();
    /// assert_eq!(b.to_integer(), Some(0b0101));  // len=4, bits 0,2 set
    /// ```
    pub fn not(&self) -> Bitset {
        let result_words: Vec<u64> = self.words.iter().map(|&w| !w).collect();

        // Critical: clean trailing bits! The NOT operation flipped ALL bits
        // in every word, including the trailing bits beyond `len` that were
        // zero. We must zero them out again to maintain the invariant.
        //
        //     Before NOT: word[3] = 0b00000000_XXXXXXXX  (trailing bits are 0)
        //     After  NOT: word[3] = 0b11111111_xxxxxxxx  (trailing bits are 1!)
        //     After clean: word[3] = 0b00000000_xxxxxxxx  (trailing bits zeroed)
        let mut result = Bitset {
            words: result_words,
            len: self.len,
        };
        result.clean_trailing_bits();
        result
    }

    /// AND-NOT (set difference): bits in `self` that are NOT in `other`.
    ///
    /// This is equivalent to `self & (~other)`, but more efficient because
    /// we don't need to create an intermediate NOT result.
    ///
    /// # Truth table
    ///
    /// ```text
    /// A  B  A & ~B
    /// 0  0    0
    /// 0  1    0
    /// 1  0    1
    /// 1  1    0
    /// ```
    ///
    /// AND-NOT is used for **set difference**: elements in A but not in B.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let a = Bitset::from_integer(0b1110);  // bits 1,2,3
    /// let b = Bitset::from_integer(0b1010);  // bits 1,3
    /// let c = a.and_not(&b);
    /// assert_eq!(c.to_integer(), Some(0b0100));  // only bit 2
    /// ```
    pub fn and_not(&self, other: &Bitset) -> Bitset {
        let result_len = self.len.max(other.len);
        let max_words = self.words.len().max(other.words.len());
        let mut result_words = Vec::with_capacity(max_words);

        for i in 0..max_words {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            // a & !b: keep bits from a that are NOT in b
            result_words.push(a & !b);
        }

        let mut result = Bitset {
            words: result_words,
            len: result_len,
        };
        result.clean_trailing_bits();
        result
    }

    // ------------------------------------------------------------------
    // Counting and query operations
    // ------------------------------------------------------------------

    /// Count the number of set (1) bits. Named after the CPU instruction
    /// `POPCNT` (population count) that does this for a single word.
    ///
    /// # How it works
    ///
    /// We call `count_ones()` on each word and sum the results. Rust's
    /// `count_ones()` compiles to the hardware POPCNT instruction on
    /// modern x86 CPUs, making this extremely fast.
    ///
    /// For a bitset with N bits, this runs in O(N/64) time -- we process
    /// 64 bits per loop iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::from_integer(0b10110);  // bits 1,2,4 set
    /// assert_eq!(bs.popcount(), 3);
    /// ```
    pub fn popcount(&self) -> usize {
        // sum the popcount of each word
        self.words.iter().map(|&w| w.count_ones() as usize).sum()
    }

    /// Returns the logical length: the number of addressable bits.
    ///
    /// This is the value passed to `new(size)`, or the highest bit index + 1
    /// after any auto-growth operations.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the capacity: the total allocated bits (always a multiple of 64).
    ///
    /// Capacity >= len. The difference (capacity - len) is "slack space" --
    /// bits that exist in memory but are always zero.
    pub fn capacity(&self) -> usize {
        self.words.len() * BITS_PER_WORD
    }

    /// Returns `true` if at least one bit is set.
    ///
    /// Short-circuits: returns as soon as it finds a non-zero word,
    /// without scanning the rest. This is O(1) in the best case
    /// (first word is non-zero) and O(N/64) in the worst case.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let mut bs = Bitset::new(100);
    /// assert!(!bs.any());
    /// bs.set(50);
    /// assert!(bs.any());
    /// ```
    pub fn any(&self) -> bool {
        self.words.iter().any(|&w| w != 0)
    }

    /// Returns `true` if ALL bits in `0..len` are set.
    ///
    /// For an empty bitset (`len = 0`), returns `true` -- this is
    /// **vacuous truth**, the same convention used by Python's `all([])`,
    /// Rust's `Iterator::all`, and mathematical logic.
    ///
    /// # How it works
    ///
    /// For each full word (words 0 through second-to-last), we check if
    /// every bit is set (word == u64::MAX, i.e., all 64 bits are 1).
    ///
    /// For the last word, we only check the bits within `len`. We create
    /// a mask of the valid bits and check that all valid bits are set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::new(0);
    /// assert!(bs.all());  // vacuous truth
    ///
    /// let bs = Bitset::from_binary_str("1111").unwrap();
    /// assert!(bs.all());
    ///
    /// let bs = Bitset::from_binary_str("1110").unwrap();
    /// assert!(!bs.all());
    /// ```
    pub fn all(&self) -> bool {
        // Vacuous truth: all bits of nothing are set.
        if self.len == 0 {
            return true;
        }

        let num_words = self.words.len();

        // Check all full words (all bits must be 1 = u64::MAX).
        for i in 0..num_words.saturating_sub(1) {
            if self.words[i] != u64::MAX {
                return false;
            }
        }

        // Check the last word: only the bits within `len` matter.
        let remaining = bit_offset(self.len);
        if remaining == 0 {
            // len is a multiple of 64, so the last word is a full word.
            self.words[num_words - 1] == u64::MAX
        } else {
            // Create a mask for the valid bits: (1 << remaining) - 1
            // Example: remaining = 8 → mask = 0xFF (bits 0-7)
            let mask = (1u64 << remaining) - 1;
            self.words[num_words - 1] == mask
        }
    }

    /// Returns `true` if no bits are set. Equivalent to `!self.any()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::new(100);
    /// assert!(bs.none());
    /// ```
    pub fn none(&self) -> bool {
        !self.any()
    }

    /// Returns `true` if the bitset has zero length.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // ------------------------------------------------------------------
    // Iteration
    // ------------------------------------------------------------------

    /// Iterate over the indices of all set bits in ascending order.
    ///
    /// # How it works: trailing-zero-count trick
    ///
    /// For each non-zero word, we use `trailing_zeros()` to find the lowest
    /// set bit, yield its index, then clear it with `word &= word - 1`:
    ///
    /// ```text
    /// word = 0b10100100   (bits 2, 5, 7 are set)
    ///
    /// Step 1: trailing_zeros = 2  -> yield base + 2
    ///         word &= word - 1   -> 0b10100000  (clear bit 2)
    ///
    /// Step 2: trailing_zeros = 5  -> yield base + 5
    ///         word &= word - 1   -> 0b10000000  (clear bit 5)
    ///
    /// Step 3: trailing_zeros = 7  -> yield base + 7
    ///         word &= word - 1   -> 0b00000000  (clear bit 7)
    ///
    /// word == 0, move to next word.
    /// ```
    ///
    /// The trick `word &= word - 1` clears the lowest set bit. Here's why:
    ///
    /// ```text
    /// word     = 0b10100100
    /// word - 1 = 0b10100011  (borrow propagates through trailing zeros)
    /// AND      = 0b10100000  (lowest set bit is cleared)
    /// ```
    ///
    /// This is O(k) where k is the number of set bits, and it skips zero
    /// words entirely, making it very efficient for sparse bitsets.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::from_integer(0b10100101);
    /// let bits: Vec<usize> = bs.iter_set_bits().collect();
    /// assert_eq!(bits, vec![0, 2, 5, 7]);
    /// ```
    pub fn iter_set_bits(&self) -> SetBitIterator<'_> {
        SetBitIterator {
            bitset: self,
            word_idx: 0,
            current_word: self.words.first().copied().unwrap_or(0),
        }
    }

    // ------------------------------------------------------------------
    // Conversion operations
    // ------------------------------------------------------------------

    /// Convert the bitset to a u64 integer, if it fits.
    ///
    /// Returns `None` if the bitset has set bits beyond position 63
    /// (i.e., it requires more than one word to represent).
    ///
    /// Returns `Some(0)` for an empty bitset.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::from_integer(42);
    /// assert_eq!(bs.to_integer(), Some(42));
    ///
    /// let bs = Bitset::new(0);
    /// assert_eq!(bs.to_integer(), Some(0));
    /// ```
    pub fn to_integer(&self) -> Option<u64> {
        // Empty bitset = 0.
        if self.words.is_empty() {
            return Some(0);
        }

        // Check that all words beyond the first are zero.
        for i in 1..self.words.len() {
            if self.words[i] != 0 {
                return None;
            }
        }

        Some(self.words[0])
    }

    /// Convert the bitset to a binary string with the highest bit on the left.
    ///
    /// This is the inverse of `from_binary_str`. An empty bitset produces
    /// an empty string `""`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bitset::Bitset;
    ///
    /// let bs = Bitset::from_integer(5);  // binary 101
    /// assert_eq!(bs.to_binary_str(), "101");
    ///
    /// let bs = Bitset::new(0);
    /// assert_eq!(bs.to_binary_str(), "");
    /// ```
    pub fn to_binary_str(&self) -> String {
        if self.len == 0 {
            return String::new();
        }

        // Build the string from the highest bit (len-1) down to bit 0.
        // This produces conventional binary notation: MSB on the left.
        let mut s = String::with_capacity(self.len);
        for i in (0..self.len).rev() {
            if self.test(i) {
                s.push('1');
            } else {
                s.push('0');
            }
        }
        s
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Ensure the bitset has capacity for bit `i`. If not, grow by doubling.
    ///
    /// After this call, `i < capacity` and `len >= i + 1`.
    ///
    /// # Growth strategy
    ///
    /// We double the capacity repeatedly until it exceeds `i`. The minimum
    /// capacity after growth is 64 (one word). This doubling strategy gives
    /// amortized O(1) growth, just like `Vec::push`.
    ///
    /// ```text
    /// Example: capacity=128, set(500)
    ///   128 -> 256 -> 512 -> 1024  (stop: 500 < 1024)
    /// ```
    fn ensure_capacity(&mut self, i: usize) {
        if i < self.capacity() {
            // Already have room. But we might need to update len.
            if i >= self.len {
                self.len = i + 1;
            }
            return;
        }

        // Need to grow. Start with current capacity (or 64 as minimum).
        let mut new_cap = self.capacity().max(BITS_PER_WORD);
        while new_cap <= i {
            new_cap *= 2;
        }

        // Extend the word vector with zeros.
        let new_word_count = new_cap / BITS_PER_WORD;
        self.words.resize(new_word_count, 0);

        // Update len to include the new bit.
        self.len = i + 1;
    }

    /// Zero out any bits beyond `len` in the last word.
    ///
    /// This maintains the clean-trailing-bits invariant. It must be called
    /// after any operation that might set bits beyond `len`:
    ///   - not() flips all bits, including trailing ones
    ///   - from_binary_str might have rounding issues
    ///   - toggle() on the last word
    ///   - bulk operations (AND, OR, XOR) when operands have different sizes
    ///
    /// # How it works
    ///
    /// ```text
    /// len = 200, capacity = 256
    /// The last word holds bits 192-255, but only 192-199 are "real".
    /// remaining = 200 % 64 = 8
    /// mask = (1 << 8) - 1 = 0xFF  (bits 0-7)
    /// words[3] &= 0xFF  -> zeroes out bits 8-63 of word 3
    /// ```
    ///
    /// If `len` is a multiple of 64, there are no trailing bits to clean.
    fn clean_trailing_bits(&mut self) {
        if self.len == 0 || self.words.is_empty() {
            return;
        }

        let remaining = bit_offset(self.len);
        if remaining != 0 {
            let last_idx = self.words.len() - 1;
            let mask = (1u64 << remaining) - 1;
            self.words[last_idx] &= mask;
        }
    }
}

// ---------------------------------------------------------------------------
// Iterator for set bits
// ---------------------------------------------------------------------------
//
// This is a separate struct because Rust iterators need to hold state between
// calls to `next()`. We track which word we're scanning and a mutable copy
// of the current word (so we can clear bits as we yield them).

/// An iterator over the indices of set bits in a [`Bitset`].
///
/// Created by [`Bitset::iter_set_bits`]. Yields bit indices in ascending order.
pub struct SetBitIterator<'a> {
    bitset: &'a Bitset,
    word_idx: usize,
    current_word: u64,
}

impl<'a> Iterator for SetBitIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        // Skip over zero words -- they have no set bits.
        while self.current_word == 0 {
            self.word_idx += 1;
            if self.word_idx >= self.bitset.words.len() {
                return None; // exhausted all words
            }
            self.current_word = self.bitset.words[self.word_idx];
        }

        // Find the lowest set bit using trailing_zeros.
        //
        //     current_word = 0b00101000
        //     trailing_zeros = 3 → bit 3 within this word
        //     bit index = word_idx * 64 + 3
        let bit_pos = self.current_word.trailing_zeros() as usize;
        let index = self.word_idx * BITS_PER_WORD + bit_pos;

        // Only yield bits within len (don't yield trailing garbage).
        if index >= self.bitset.len {
            return None;
        }

        // Clear the lowest set bit: word &= word - 1
        //
        //     0b00101000 & 0b00100111 = 0b00100000
        //     (bit 3 is cleared, bit 5 remains)
        self.current_word &= self.current_word - 1;

        Some(index)
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

// Display: human-readable representation like "Bitset(101)"
impl fmt::Display for Bitset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bitset({})", self.to_binary_str())
    }
}

// PartialEq and Eq: two bitsets are equal if they have the same len and
// the same bits set. Thanks to the clean-trailing-bits invariant, we can
// compare words directly -- trailing bits are always zero, so two bitsets
// with the same logical content will have identical word vectors (up to
// the number of words needed for the longer one).
impl PartialEq for Bitset {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }

        // Compare word-by-word. If one has more words allocated, the
        // extra words must all be zero (due to clean-trailing-bits).
        let max_words = self.words.len().max(other.words.len());
        for i in 0..max_words {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            if a != b {
                return false;
            }
        }
        true
    }
}

impl Eq for Bitset {}

// ---------------------------------------------------------------------------
// Operator overloading via std::ops traits
// ---------------------------------------------------------------------------
//
// These let you write `a & b`, `a | b`, `a ^ b`, `!a` instead of
// `a.and(&b)`, `a.or(&b)`, etc. Rust uses the BitAnd, BitOr, BitXor,
// and Not traits from std::ops for this.
//
// We implement them for references (&Bitset) so you can write `&a & &b`
// without consuming the operands. We also implement for owned values.

// &Bitset & &Bitset
impl<'a, 'b> BitAnd<&'b Bitset> for &'a Bitset {
    type Output = Bitset;
    fn bitand(self, rhs: &'b Bitset) -> Bitset {
        self.and(rhs)
    }
}

// Bitset & Bitset (owned)
impl BitAnd for Bitset {
    type Output = Bitset;
    fn bitand(self, rhs: Bitset) -> Bitset {
        self.and(&rhs)
    }
}

// &Bitset | &Bitset
impl<'a, 'b> BitOr<&'b Bitset> for &'a Bitset {
    type Output = Bitset;
    fn bitor(self, rhs: &'b Bitset) -> Bitset {
        self.or(rhs)
    }
}

// Bitset | Bitset (owned)
impl BitOr for Bitset {
    type Output = Bitset;
    fn bitor(self, rhs: Bitset) -> Bitset {
        self.or(&rhs)
    }
}

// &Bitset ^ &Bitset
impl<'a, 'b> BitXor<&'b Bitset> for &'a Bitset {
    type Output = Bitset;
    fn bitxor(self, rhs: &'b Bitset) -> Bitset {
        self.xor(rhs)
    }
}

// Bitset ^ Bitset (owned)
impl BitXor for Bitset {
    type Output = Bitset;
    fn bitxor(self, rhs: Bitset) -> Bitset {
        self.xor(&rhs)
    }
}

// !&Bitset
impl<'a> Not for &'a Bitset {
    type Output = Bitset;
    fn not(self) -> Bitset {
        Bitset::not(self)
    }
}

// !Bitset (owned)
impl Not for Bitset {
    type Output = Bitset;
    fn not(self) -> Bitset {
        Bitset::not(&self)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Constructor tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_new_zero_size() {
        let bs = Bitset::new(0);
        assert_eq!(bs.len(), 0);
        assert_eq!(bs.capacity(), 0);
        assert_eq!(bs.popcount(), 0);
        assert!(bs.none());
        assert!(bs.all()); // vacuous truth
    }

    #[test]
    fn test_new_various_sizes() {
        // Size 1: needs 1 word (64 bits capacity)
        let bs = Bitset::new(1);
        assert_eq!(bs.len(), 1);
        assert_eq!(bs.capacity(), 64);

        // Size 64: fits exactly in 1 word
        let bs = Bitset::new(64);
        assert_eq!(bs.len(), 64);
        assert_eq!(bs.capacity(), 64);

        // Size 65: needs 2 words (128 bits capacity)
        let bs = Bitset::new(65);
        assert_eq!(bs.len(), 65);
        assert_eq!(bs.capacity(), 128);

        // Size 128: fits exactly in 2 words
        let bs = Bitset::new(128);
        assert_eq!(bs.len(), 128);
        assert_eq!(bs.capacity(), 128);

        // Size 200: needs ceil(200/64) = 4 words = 256 bits
        let bs = Bitset::new(200);
        assert_eq!(bs.len(), 200);
        assert_eq!(bs.capacity(), 256);
    }

    #[test]
    fn test_new_all_zeros() {
        let bs = Bitset::new(1000);
        assert_eq!(bs.popcount(), 0);
        for i in 0..1000 {
            assert!(!bs.test(i), "bit {} should be 0", i);
        }
    }

    #[test]
    fn test_from_integer_zero() {
        let bs = Bitset::from_integer(0);
        assert_eq!(bs.len(), 0);
        assert_eq!(bs.to_integer(), Some(0));
    }

    #[test]
    fn test_from_integer_small_values() {
        // 1 = binary 1 → bit 0 set, len = 1
        let bs = Bitset::from_integer(1);
        assert_eq!(bs.len(), 1);
        assert!(bs.test(0));
        assert_eq!(bs.to_integer(), Some(1));

        // 5 = binary 101 → bits 0,2 set, len = 3
        let bs = Bitset::from_integer(5);
        assert_eq!(bs.len(), 3);
        assert!(bs.test(0));
        assert!(!bs.test(1));
        assert!(bs.test(2));
        assert_eq!(bs.to_integer(), Some(5));

        // 255 = binary 11111111 → bits 0-7 set, len = 8
        let bs = Bitset::from_integer(255);
        assert_eq!(bs.len(), 8);
        assert_eq!(bs.popcount(), 8);
    }

    #[test]
    fn test_from_integer_powers_of_two() {
        // Power of two: only one bit set
        for exp in 0..63u32 {
            let val = 1u128 << exp;
            let bs = Bitset::from_integer(val);
            assert_eq!(bs.len(), exp as usize + 1);
            assert_eq!(bs.popcount(), 1);
            assert!(bs.test(exp as usize));
        }
    }

    #[test]
    fn test_from_integer_u64_max() {
        let bs = Bitset::from_integer(u64::MAX as u128);
        assert_eq!(bs.len(), 64);
        assert_eq!(bs.popcount(), 64);
        assert_eq!(bs.to_integer(), Some(u64::MAX));
    }

    #[test]
    fn test_from_integer_large_u128() {
        // A value that requires 2 words
        let val: u128 = (1u128 << 64) | 42; // bit 64 set, plus 42 in low word
        let bs = Bitset::from_integer(val);
        assert_eq!(bs.len(), 65);
        assert!(bs.test(64)); // high bit
        assert!(bs.test(1));  // from 42 = 0b101010
        assert!(bs.test(3));
        assert!(bs.test(5));
    }

    #[test]
    fn test_from_binary_str_empty() {
        let bs = Bitset::from_binary_str("").unwrap();
        assert_eq!(bs.len(), 0);
        assert_eq!(bs.to_binary_str(), "");
    }

    #[test]
    fn test_from_binary_str_single_bits() {
        let bs = Bitset::from_binary_str("0").unwrap();
        assert_eq!(bs.len(), 1);
        assert!(!bs.test(0));

        let bs = Bitset::from_binary_str("1").unwrap();
        assert_eq!(bs.len(), 1);
        assert!(bs.test(0));
    }

    #[test]
    fn test_from_binary_str_various() {
        // "1010" → bits 1,3 set (reading right to left)
        let bs = Bitset::from_binary_str("1010").unwrap();
        assert_eq!(bs.len(), 4);
        assert!(!bs.test(0));
        assert!(bs.test(1));
        assert!(!bs.test(2));
        assert!(bs.test(3));
        assert_eq!(bs.to_integer(), Some(10));

        // "11111111" → all 8 bits set
        let bs = Bitset::from_binary_str("11111111").unwrap();
        assert_eq!(bs.len(), 8);
        assert_eq!(bs.popcount(), 8);
        assert_eq!(bs.to_integer(), Some(255));
    }

    #[test]
    fn test_from_binary_str_leading_zeros() {
        // "0001" → len=4, only bit 0 set
        let bs = Bitset::from_binary_str("0001").unwrap();
        assert_eq!(bs.len(), 4);
        assert_eq!(bs.to_integer(), Some(1));
        assert_eq!(bs.to_binary_str(), "0001");
    }

    #[test]
    fn test_from_binary_str_invalid() {
        assert!(Bitset::from_binary_str("102").is_err());
        assert!(Bitset::from_binary_str("abc").is_err());
        assert!(Bitset::from_binary_str("10 01").is_err());
        assert!(Bitset::from_binary_str("1.0").is_err());

        // Check the error type
        match Bitset::from_binary_str("bad") {
            Err(BitsetError::InvalidBinaryString(s)) => assert_eq!(s, "bad"),
            _ => panic!("expected InvalidBinaryString error"),
        }
    }

    #[test]
    fn test_from_binary_str_long() {
        // 65 characters → spans 2 words
        let s = "1".to_string() + &"0".repeat(64);
        let bs = Bitset::from_binary_str(&s).unwrap();
        assert_eq!(bs.len(), 65);
        assert!(bs.test(64));  // the leading '1'
        assert_eq!(bs.popcount(), 1);
    }

    // -----------------------------------------------------------------------
    // Single-bit operation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_and_test() {
        let mut bs = Bitset::new(100);
        assert!(!bs.test(50));
        bs.set(50);
        assert!(bs.test(50));
        assert_eq!(bs.popcount(), 1);
    }

    #[test]
    fn test_set_idempotent() {
        // Setting a bit twice should be the same as setting it once
        let mut bs = Bitset::new(100);
        bs.set(42);
        bs.set(42);
        assert_eq!(bs.popcount(), 1);
    }

    #[test]
    fn test_clear() {
        let mut bs = Bitset::new(100);
        bs.set(50);
        assert!(bs.test(50));
        bs.clear(50);
        assert!(!bs.test(50));
        assert_eq!(bs.popcount(), 0);
    }

    #[test]
    fn test_clear_idempotent() {
        let mut bs = Bitset::new(100);
        bs.clear(50); // clear an already-clear bit
        assert!(!bs.test(50));
        assert_eq!(bs.popcount(), 0);
    }

    #[test]
    fn test_clear_beyond_len() {
        let mut bs = Bitset::new(10);
        bs.clear(999); // no-op, no panic, no growth
        assert_eq!(bs.len(), 10);
    }

    #[test]
    fn test_test_beyond_len() {
        let bs = Bitset::new(10);
        assert!(!bs.test(999)); // returns false, no panic
        assert_eq!(bs.len(), 10);
    }

    #[test]
    fn test_toggle() {
        let mut bs = Bitset::new(10);
        assert!(!bs.test(5));
        bs.toggle(5); // 0 → 1
        assert!(bs.test(5));
        bs.toggle(5); // 1 → 0
        assert!(!bs.test(5));
    }

    #[test]
    fn test_set_auto_growth() {
        let mut bs = Bitset::new(10);
        assert_eq!(bs.len(), 10);
        assert_eq!(bs.capacity(), 64);

        // Set a bit beyond capacity → triggers growth
        bs.set(200);
        assert!(bs.test(200));
        assert_eq!(bs.len(), 201);
        assert!(bs.capacity() >= 201);

        // Previous bits still work
        bs.set(5);
        assert!(bs.test(5));
    }

    #[test]
    fn test_toggle_auto_growth() {
        let mut bs = Bitset::new(10);
        bs.toggle(100); // grows and sets bit 100
        assert_eq!(bs.len(), 101);
        assert!(bs.test(100));
    }

    #[test]
    fn test_set_at_word_boundary() {
        let mut bs = Bitset::new(64);
        // Bit 63 is the last bit of word 0
        bs.set(63);
        assert!(bs.test(63));

        // Bit 64 is the first bit of word 1 — triggers growth
        bs.set(64);
        assert!(bs.test(64));
        assert_eq!(bs.len(), 65);
        assert_eq!(bs.capacity(), 128);
    }

    #[test]
    fn test_growth_doubling() {
        // Verify the doubling strategy
        let mut bs = Bitset::new(0);
        assert_eq!(bs.capacity(), 0);

        bs.set(0);
        assert_eq!(bs.capacity(), 64); // minimum is 64

        bs.set(63);
        assert_eq!(bs.capacity(), 64); // still fits in 1 word

        bs.set(64);
        assert_eq!(bs.capacity(), 128); // doubled to 2 words

        bs.set(200);
        assert!(bs.capacity() >= 201);
        // Should have doubled: 128 → 256
        assert_eq!(bs.capacity(), 256);
    }

    // -----------------------------------------------------------------------
    // Bulk operation tests with truth table verification
    // -----------------------------------------------------------------------

    #[test]
    fn test_and_truth_table() {
        // Verify the AND truth table for each bit combination:
        //   A=0,B=0 → 0    A=0,B=1 → 0    A=1,B=0 → 0    A=1,B=1 → 1
        let a = Bitset::from_integer(0b1100); // bits 2,3
        let b = Bitset::from_integer(0b1010); // bits 1,3
        let c = a.and(&b);
        assert_eq!(c.to_integer(), Some(0b1000)); // only bit 3

        // Verify each bit:
        assert!(!c.test(0)); // 0 & 0 = 0
        assert!(!c.test(1)); // 0 & 1 = 0
        assert!(!c.test(2)); // 1 & 0 = 0
        assert!(c.test(3));  // 1 & 1 = 1
    }

    #[test]
    fn test_or_truth_table() {
        let a = Bitset::from_integer(0b1100);
        let b = Bitset::from_integer(0b1010);
        let c = a.or(&b);
        assert_eq!(c.to_integer(), Some(0b1110)); // bits 1,2,3

        assert!(!c.test(0)); // 0 | 0 = 0
        assert!(c.test(1));  // 0 | 1 = 1
        assert!(c.test(2));  // 1 | 0 = 1
        assert!(c.test(3));  // 1 | 1 = 1
    }

    #[test]
    fn test_xor_truth_table() {
        let a = Bitset::from_integer(0b1100);
        let b = Bitset::from_integer(0b1010);
        let c = a.xor(&b);
        assert_eq!(c.to_integer(), Some(0b0110)); // bits 1,2

        assert!(!c.test(0)); // 0 ^ 0 = 0
        assert!(c.test(1));  // 0 ^ 1 = 1
        assert!(c.test(2));  // 1 ^ 0 = 1
        assert!(!c.test(3)); // 1 ^ 1 = 0
    }

    #[test]
    fn test_not_truth_table() {
        let a = Bitset::from_integer(0b1010); // len=4, bits 1,3
        let b = a.not();
        assert_eq!(b.len(), 4);
        assert_eq!(b.to_integer(), Some(0b0101)); // bits 0,2

        assert!(b.test(0));  // ~0 = 1
        assert!(!b.test(1)); // ~1 = 0
        assert!(b.test(2));  // ~0 = 1
        assert!(!b.test(3)); // ~1 = 0
    }

    #[test]
    fn test_and_not_truth_table() {
        let a = Bitset::from_integer(0b1110); // bits 1,2,3
        let b = Bitset::from_integer(0b1010); // bits 1,3
        let c = a.and_not(&b);
        assert_eq!(c.to_integer(), Some(0b0100)); // only bit 2

        assert!(!c.test(0)); // 0 & ~0 = 0
        assert!(!c.test(1)); // 1 & ~1 = 0
        assert!(c.test(2));  // 1 & ~0 = 1
        assert!(!c.test(3)); // 1 & ~1 = 0
    }

    #[test]
    fn test_bulk_ops_different_sizes() {
        // a has 4 bits, b has 8 bits → result has 8 bits
        let a = Bitset::from_integer(0b1010);     // len=4
        let b = Bitset::from_integer(0b11001100); // len=8
        let c = a.or(&b);
        assert_eq!(c.len(), 8);
        // a zero-extended: 0b00001010
        // b:               0b11001100
        // OR:              0b11001110
        assert_eq!(c.to_integer(), Some(0b11001110));
    }

    #[test]
    fn test_bulk_ops_with_empty() {
        let a = Bitset::from_integer(42);
        let empty = Bitset::new(0);

        // AND with empty → all zeros (but len = max of the two)
        let c = a.and(&empty);
        assert_eq!(c.len(), a.len());
        assert_eq!(c.popcount(), 0);

        // OR with empty → same as a
        let c = a.or(&empty);
        assert_eq!(c.to_integer(), Some(42));

        // XOR with empty → same as a
        let c = a.xor(&empty);
        assert_eq!(c.to_integer(), Some(42));
    }

    #[test]
    fn test_not_clean_trailing_bits() {
        // NOT must clean trailing bits. If len=5, capacity=64, then
        // NOT should only flip bits 0-4, not bits 5-63.
        let a = Bitset::from_binary_str("10101").unwrap(); // len=5
        let b = a.not();
        assert_eq!(b.len(), 5);
        assert_eq!(b.to_binary_str(), "01010");
        assert_eq!(b.popcount(), 2); // only 2 bits set, not 62
    }

    #[test]
    fn test_not_involution() {
        // NOT applied twice should give back the original: ~~a == a
        let a = Bitset::from_integer(0b11001010);
        let b = Bitset::not(&Bitset::not(&a));
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // Operator overloading tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_operator_bitand() {
        let a = Bitset::from_integer(0b1100);
        let b = Bitset::from_integer(0b1010);
        let c = &a & &b;
        assert_eq!(c.to_integer(), Some(0b1000));

        // Owned version
        let c = a.clone() & b.clone();
        assert_eq!(c.to_integer(), Some(0b1000));
    }

    #[test]
    fn test_operator_bitor() {
        let a = Bitset::from_integer(0b1100);
        let b = Bitset::from_integer(0b1010);
        let c = &a | &b;
        assert_eq!(c.to_integer(), Some(0b1110));
    }

    #[test]
    fn test_operator_bitxor() {
        let a = Bitset::from_integer(0b1100);
        let b = Bitset::from_integer(0b1010);
        let c = &a ^ &b;
        assert_eq!(c.to_integer(), Some(0b0110));
    }

    #[test]
    fn test_operator_not() {
        let a = Bitset::from_integer(0b1010); // len=4
        let b = !&a;
        assert_eq!(b.to_integer(), Some(0b0101));

        // Owned version
        let b = !a;
        assert_eq!(b.to_integer(), Some(0b0101));
    }

    // -----------------------------------------------------------------------
    // Counting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_popcount_empty() {
        assert_eq!(Bitset::new(0).popcount(), 0);
        assert_eq!(Bitset::new(1000).popcount(), 0);
    }

    #[test]
    fn test_popcount_various() {
        let bs = Bitset::from_integer(0b10110); // 3 bits set
        assert_eq!(bs.popcount(), 3);

        let bs = Bitset::from_integer(u64::MAX as u128); // 64 bits set
        assert_eq!(bs.popcount(), 64);
    }

    #[test]
    fn test_any_none() {
        let bs = Bitset::new(100);
        assert!(!bs.any());
        assert!(bs.none());

        let mut bs = Bitset::new(100);
        bs.set(50);
        assert!(bs.any());
        assert!(!bs.none());
    }

    #[test]
    fn test_all() {
        // Empty: vacuous truth
        assert!(Bitset::new(0).all());

        // All set
        let bs = Bitset::from_binary_str("1111").unwrap();
        assert!(bs.all());

        // Not all set
        let bs = Bitset::from_binary_str("1110").unwrap();
        assert!(!bs.all());

        // Single bit, set
        let bs = Bitset::from_binary_str("1").unwrap();
        assert!(bs.all());

        // Single bit, not set
        let bs = Bitset::from_binary_str("0").unwrap();
        assert!(!bs.all());
    }

    #[test]
    fn test_all_full_words() {
        // 64 bits all set → all() should be true
        let mut bs = Bitset::new(64);
        for i in 0..64 {
            bs.set(i);
        }
        assert!(bs.all());
        assert_eq!(bs.popcount(), 64);
    }

    #[test]
    fn test_all_partial_last_word() {
        // 70 bits all set → last word is partial
        let mut bs = Bitset::new(70);
        for i in 0..70 {
            bs.set(i);
        }
        assert!(bs.all());

        // Clear one bit → all() should be false
        bs.clear(69);
        assert!(!bs.all());
    }

    #[test]
    fn test_is_empty() {
        assert!(Bitset::new(0).is_empty());
        assert!(!Bitset::new(1).is_empty());
    }

    // -----------------------------------------------------------------------
    // Iteration tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_iter_set_bits_empty() {
        let bs = Bitset::new(0);
        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert!(bits.is_empty());

        let bs = Bitset::new(100);
        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert!(bits.is_empty());
    }

    #[test]
    fn test_iter_set_bits_single() {
        let mut bs = Bitset::new(100);
        bs.set(42);
        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert_eq!(bits, vec![42]);
    }

    #[test]
    fn test_iter_set_bits_multiple() {
        let bs = Bitset::from_integer(0b10100101); // bits 0,2,5,7
        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert_eq!(bits, vec![0, 2, 5, 7]);
    }

    #[test]
    fn test_iter_set_bits_across_words() {
        let mut bs = Bitset::new(200);
        bs.set(0);
        bs.set(63);  // last bit of word 0
        bs.set(64);  // first bit of word 1
        bs.set(127); // last bit of word 1
        bs.set(128); // first bit of word 2
        bs.set(199); // last addressable bit

        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert_eq!(bits, vec![0, 63, 64, 127, 128, 199]);
    }

    #[test]
    fn test_iter_set_bits_dense() {
        // All bits set in a small bitset
        let bs = Bitset::from_binary_str("11111111").unwrap();
        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert_eq!(bits, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    // -----------------------------------------------------------------------
    // Conversion tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_to_integer_empty() {
        assert_eq!(Bitset::new(0).to_integer(), Some(0));
    }

    #[test]
    fn test_to_integer_roundtrip() {
        for val in [0u64, 1, 5, 42, 255, 1000, u64::MAX] {
            let bs = Bitset::from_integer(val as u128);
            assert_eq!(bs.to_integer(), Some(val), "roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_to_integer_overflow() {
        // A bitset with bits set in word 1 can't fit in a u64
        let val: u128 = (1u128 << 64) | 1;
        let bs = Bitset::from_integer(val);
        assert_eq!(bs.to_integer(), None);
    }

    #[test]
    fn test_to_binary_str_empty() {
        assert_eq!(Bitset::new(0).to_binary_str(), "");
    }

    #[test]
    fn test_to_binary_str_roundtrip() {
        for s in ["1", "0", "101", "1010", "11111111", "10000000", "0001"] {
            let bs = Bitset::from_binary_str(s).unwrap();
            assert_eq!(bs.to_binary_str(), s, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn test_display() {
        let bs = Bitset::from_integer(5);
        assert_eq!(format!("{}", bs), "Bitset(101)");

        let bs = Bitset::new(0);
        assert_eq!(format!("{}", bs), "Bitset()");
    }

    // -----------------------------------------------------------------------
    // Equality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_equality_basic() {
        let a = Bitset::from_integer(42);
        let b = Bitset::from_integer(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_equality_different_values() {
        let a = Bitset::from_integer(42);
        let b = Bitset::from_integer(43);
        assert_ne!(a, b);
    }

    #[test]
    fn test_equality_different_len_same_bits() {
        // Same bits set, but different len → NOT equal
        // (because len is part of the bitset's identity)
        let a = Bitset::from_binary_str("101").unwrap();   // len=3
        let b = Bitset::from_binary_str("0101").unwrap();  // len=4
        assert_ne!(a, b);
    }

    #[test]
    fn test_equality_empty() {
        let a = Bitset::new(0);
        let b = Bitset::new(0);
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_word_boundary_63_64() {
        // Bit 63 is the last bit of word 0, bit 64 is the first of word 1
        let mut bs = Bitset::new(128);
        bs.set(63);
        bs.set(64);
        assert!(bs.test(63));
        assert!(bs.test(64));
        assert!(!bs.test(62));
        assert!(!bs.test(65));
        assert_eq!(bs.popcount(), 2);
    }

    #[test]
    fn test_word_boundary_127_128() {
        let mut bs = Bitset::new(256);
        bs.set(127);
        bs.set(128);
        assert!(bs.test(127));
        assert!(bs.test(128));
        assert_eq!(bs.popcount(), 2);
    }

    #[test]
    fn test_large_bitset() {
        let mut bs = Bitset::new(10000);
        // Set every 100th bit
        for i in (0..10000).step_by(100) {
            bs.set(i);
        }
        assert_eq!(bs.popcount(), 100);

        let bits: Vec<usize> = bs.iter_set_bits().collect();
        assert_eq!(bits.len(), 100);
        assert_eq!(bits[0], 0);
        assert_eq!(bits[1], 100);
        assert_eq!(bits[99], 9900);
    }

    #[test]
    fn test_set_clear_all_bits_in_word() {
        let mut bs = Bitset::new(64);
        // Set all 64 bits
        for i in 0..64 {
            bs.set(i);
        }
        assert_eq!(bs.popcount(), 64);
        assert!(bs.all());

        // Clear all 64 bits
        for i in 0..64 {
            bs.clear(i);
        }
        assert_eq!(bs.popcount(), 0);
        assert!(bs.none());
    }

    #[test]
    fn test_clean_trailing_bits_invariant() {
        // After NOT, the trailing bits must be zero.
        // Create a bitset with len=5, capacity=64.
        let bs = Bitset::from_binary_str("10101").unwrap();
        let orig_popcount = bs.popcount();
        let notted = Bitset::not(&bs);

        // popcount of NOT should be len - popcount(original)
        assert_eq!(notted.popcount(), 5 - orig_popcount);

        // The capacity is 64, but only 5 bits should be addressable.
        // If trailing bits leaked, popcount would be wrong.
        assert_eq!(notted.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Property tests: algebraic laws
    // -----------------------------------------------------------------------
    //
    // These tests verify that our bitset operations obey the standard
    // boolean algebra laws, which gives us confidence in correctness.

    #[test]
    fn test_and_commutativity() {
        // a & b == b & a
        let a = Bitset::from_integer(0b11001010);
        let b = Bitset::from_integer(0b10101100);
        assert_eq!(a.and(&b), b.and(&a));
    }

    #[test]
    fn test_or_commutativity() {
        // a | b == b | a
        let a = Bitset::from_integer(0b11001010);
        let b = Bitset::from_integer(0b10101100);
        assert_eq!(a.or(&b), b.or(&a));
    }

    #[test]
    fn test_xor_commutativity() {
        // a ^ b == b ^ a
        let a = Bitset::from_integer(0b11001010);
        let b = Bitset::from_integer(0b10101100);
        assert_eq!(a.xor(&b), b.xor(&a));
    }

    #[test]
    fn test_and_idempotence() {
        // a & a == a
        let a = Bitset::from_integer(0b11001010);
        assert_eq!(a.and(&a), a);
    }

    #[test]
    fn test_or_idempotence() {
        // a | a == a
        let a = Bitset::from_integer(0b11001010);
        assert_eq!(a.or(&a), a);
    }

    #[test]
    fn test_xor_self_is_zero() {
        // a ^ a == 0 (all bits cancel out)
        let a = Bitset::from_integer(0b11001010);
        let z = a.xor(&a);
        assert_eq!(z.popcount(), 0);
    }

    #[test]
    fn test_de_morgans_law_1() {
        // De Morgan's first law: ~(a & b) == (~a) | (~b)
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();

        let lhs = a.and(&b).not();
        let rhs = a.not().or(&b.not());
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_de_morgans_law_2() {
        // De Morgan's second law: ~(a | b) == (~a) & (~b)
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();

        let lhs = a.or(&b).not();
        let rhs = a.not().and(&b.not());
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_and_associativity() {
        // (a & b) & c == a & (b & c)
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();
        let c = Bitset::from_binary_str("01110011").unwrap();

        let lhs = a.and(&b).and(&c);
        let rhs = a.and(&b.and(&c));
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_or_associativity() {
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();
        let c = Bitset::from_binary_str("01110011").unwrap();

        let lhs = a.or(&b).or(&c);
        let rhs = a.or(&b.or(&c));
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_distributive_law() {
        // a & (b | c) == (a & b) | (a & c)
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();
        let c = Bitset::from_binary_str("01110011").unwrap();

        let lhs = a.and(&b.or(&c));
        let rhs = a.and(&b).or(&a.and(&c));
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_and_not_equals_and_with_not() {
        // a.and_not(b) == a & ~b
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();

        let lhs = a.and_not(&b);
        let rhs = a.and(&b.not());
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_double_not_identity() {
        // ~~a == a
        let a = Bitset::from_binary_str("11001010").unwrap();
        assert_eq!(Bitset::not(&Bitset::not(&a)), a);
    }

    // -----------------------------------------------------------------------
    // Multi-word property tests
    // -----------------------------------------------------------------------
    //
    // The algebraic law tests above use small bitsets (8 bits). These tests
    // use larger bitsets that span multiple words to verify correctness
    // at word boundaries.

    #[test]
    fn test_de_morgans_multi_word() {
        // Same De Morgan's test but with 200-bit bitsets
        let mut a = Bitset::new(200);
        let mut b = Bitset::new(200);

        // Set various bits across multiple words
        for i in (0..200).step_by(3) {
            a.set(i);
        }
        for i in (0..200).step_by(5) {
            b.set(i);
        }

        // ~(a & b) == (~a) | (~b)
        let lhs = Bitset::not(&a.and(&b));
        let rhs = Bitset::not(&a).or(&Bitset::not(&b));
        assert_eq!(lhs, rhs);

        // ~(a | b) == (~a) & (~b)
        let lhs = Bitset::not(&a.or(&b));
        let rhs = Bitset::not(&a).and(&Bitset::not(&b));
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_xor_inverse_multi_word() {
        // (a ^ b) ^ b == a
        let mut a = Bitset::new(200);
        let mut b = Bitset::new(200);
        for i in (0..200).step_by(7) {
            a.set(i);
        }
        for i in (0..200).step_by(11) {
            b.set(i);
        }
        let result = a.xor(&b).xor(&b);
        assert_eq!(result, a);
    }

    #[test]
    fn test_popcount_after_or() {
        // popcount(a | b) = popcount(a) + popcount(b) - popcount(a & b)
        // This is the inclusion-exclusion principle.
        let a = Bitset::from_binary_str("11001010").unwrap();
        let b = Bitset::from_binary_str("10101100").unwrap();

        let or_count = a.or(&b).popcount();
        let expected = a.popcount() + b.popcount() - a.and(&b).popcount();
        assert_eq!(or_count, expected);
    }

    // -----------------------------------------------------------------------
    // Error type tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_bitset_error_display() {
        let err = BitsetError::InvalidBinaryString("abc".to_string());
        assert_eq!(format!("{}", err), "invalid binary string: \"abc\"");
    }

    #[test]
    fn test_bitset_error_is_std_error() {
        // Verify that BitsetError implements std::error::Error
        let err: Box<dyn std::error::Error> =
            Box::new(BitsetError::InvalidBinaryString("x".to_string()));
        assert!(err.to_string().contains("invalid binary string"));
    }

    // -----------------------------------------------------------------------
    // Clone tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_independence() {
        let mut a = Bitset::from_integer(42);
        let b = a.clone();
        a.set(10); // modify original
        assert_ne!(a, b); // clone should be unaffected
    }

    // -----------------------------------------------------------------------
    // Conversion round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_integer_binary_str_roundtrip() {
        // integer → bitset → binary_str → bitset → integer
        for val in [0u64, 1, 5, 42, 255, 1000, 65535] {
            let bs1 = Bitset::from_integer(val as u128);
            let s = bs1.to_binary_str();
            let bs2 = Bitset::from_binary_str(&s).unwrap();
            assert_eq!(bs1, bs2, "roundtrip failed for {}", val);
            assert_eq!(bs2.to_integer(), Some(val), "value roundtrip failed for {}", val);
        }
    }

    #[test]
    fn test_from_binary_str_to_integer() {
        // "1010" = 10
        let bs = Bitset::from_binary_str("1010").unwrap();
        assert_eq!(bs.to_integer(), Some(10));

        // "101" = 5
        let bs = Bitset::from_binary_str("101").unwrap();
        assert_eq!(bs.to_integer(), Some(5));
    }

    // -----------------------------------------------------------------------
    // Stress tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_all_then_clear_all() {
        let mut bs = Bitset::new(256);
        for i in 0..256 {
            bs.set(i);
        }
        assert!(bs.all());
        assert_eq!(bs.popcount(), 256);

        for i in 0..256 {
            bs.clear(i);
        }
        assert!(bs.none());
        assert_eq!(bs.popcount(), 0);
    }

    #[test]
    fn test_toggle_all_twice_restores_original() {
        let mut bs = Bitset::from_integer(0b10110011);
        let original = bs.clone();

        // Toggle all bits twice → should restore original
        for i in 0..bs.len() {
            bs.toggle(i);
        }
        for i in 0..bs.len() {
            bs.toggle(i);
        }
        assert_eq!(bs, original);
    }

    #[test]
    fn test_iter_set_bits_matches_test() {
        // Every index from iter_set_bits should return true from test(),
        // and every index NOT in the iterator should return false.
        let bs = Bitset::from_integer(0b10110100101);
        let set_bits: Vec<usize> = bs.iter_set_bits().collect();

        for i in 0..bs.len() {
            if set_bits.contains(&i) {
                assert!(bs.test(i), "bit {} should be set", i);
            } else {
                assert!(!bs.test(i), "bit {} should not be set", i);
            }
        }
    }

    #[test]
    fn test_popcount_matches_iter_count() {
        let bs = Bitset::from_integer(0b1010101010101);
        assert_eq!(bs.popcount(), bs.iter_set_bits().count());
    }
}
