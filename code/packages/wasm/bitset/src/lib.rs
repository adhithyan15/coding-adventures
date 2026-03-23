// lib.rs -- WASM Bindings for the Bitset Library
// ================================================
//
// This module wraps the pure-Rust `bitset::Bitset` struct with a
// `WasmBitset` type that is exported to JavaScript/TypeScript via
// wasm-bindgen. Every public method on `WasmBitset` becomes a method
// on the JS class `WasmBitset`.
//
// Design Decisions
// ----------------
//
// 1. **Owned wrapper**: `WasmBitset` owns a `Bitset` internally. WASM
//    cannot pass Rust references across the boundary, so every method
//    takes `&self` or `&mut self` on the wrapper, which delegates to
//    the inner `Bitset`.
//
// 2. **camelCase naming**: JavaScript convention uses camelCase. The
//    `#[wasm_bindgen]` attribute's `js_name` parameter renames each
//    method for JS consumers. Rust code still uses snake_case internally.
//
// 3. **JsValue for complex returns**: Methods that return collections
//    (like `iterSetBits`) or optional values (like `toInteger`) use
//    `JsValue` because wasm-bindgen cannot directly return `Vec<usize>`
//    or `Option<u64>` to JS. We serialize them as JS arrays or null.
//
// 4. **Error handling**: Methods that can fail (like `fromBinaryStr`)
//    return `Result<WasmBitset, JsValue>` which wasm-bindgen translates
//    to a thrown JS exception on the `Err` variant.
//
// 5. **No WASM-target tests**: WASM tests require wasm-pack test or a
//    Node.js WASM loader. We include native Rust tests guarded with
//    `#[cfg(not(target_arch = "wasm32"))]` so `cargo test` works
//    without any WASM tooling.

use bitset::Bitset;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// WasmBitset -- the main exported type
// ---------------------------------------------------------------------------
//
// JavaScript usage:
//
//   import { WasmBitset } from './bitset_wasm.js';
//
//   const bs = new WasmBitset(64);      // 64-bit bitset
//   bs.set(0);
//   bs.set(5);
//   bs.test(5);                          // true
//   bs.popcount();                       // 2
//   bs.toBinaryStr();                    // "100001"
//   bs.iterSetBits();                    // [0, 5]
//
//   const a = WasmBitset.fromBinaryStr("1010");
//   const b = WasmBitset.fromBinaryStr("1100");
//   const c = a.and(b);
//   c.toBinaryStr();                     // "1000"

/// A compact bitset data structure exposed to JavaScript via WebAssembly.
///
/// Each `WasmBitset` packs boolean values into 64-bit words for space
/// efficiency and fast bulk bitwise operations. This is the WASM wrapper
/// around the Rust `bitset::Bitset` type.
#[wasm_bindgen]
pub struct WasmBitset {
    inner: Bitset,
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

#[wasm_bindgen]
impl WasmBitset {
    /// Create a new bitset with `size` bits, all initialized to 0.
    ///
    /// In JavaScript:
    ///   const bs = new WasmBitset(128);  // 128 zero bits
    #[wasm_bindgen(constructor)]
    pub fn new(size: usize) -> WasmBitset {
        WasmBitset {
            inner: Bitset::new(size),
        }
    }

    /// Create a bitset from a non-negative integer.
    ///
    /// JavaScript only has 64-bit floats, so we accept a u64 here which
    /// covers integers up to 2^53 - 1 safely from JS (Number.MAX_SAFE_INTEGER).
    /// For larger values, use `fromBinaryStr`.
    ///
    /// In JavaScript:
    ///   const bs = WasmBitset.fromInteger(42);  // binary: 101010
    #[wasm_bindgen(js_name = "fromInteger")]
    pub fn from_integer(value: u64) -> WasmBitset {
        // We pass the u64 as a u128 to the inner constructor. This is safe
        // because u64 fits in u128 without loss.
        WasmBitset {
            inner: Bitset::from_integer(value as u128),
        }
    }

    /// Create a bitset from a binary string like `"1010"`.
    ///
    /// The leftmost character is the highest bit. Returns an error (throws
    /// in JS) if the string contains characters other than '0' and '1'.
    ///
    /// In JavaScript:
    ///   const bs = WasmBitset.fromBinaryStr("1010");  // bits 1 and 3 set
    #[wasm_bindgen(js_name = "fromBinaryStr")]
    pub fn from_binary_str(s: &str) -> Result<WasmBitset, JsValue> {
        Bitset::from_binary_str(s)
            .map(|inner| WasmBitset { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // -----------------------------------------------------------------------
    // Bit manipulation
    // -----------------------------------------------------------------------

    /// Set bit `i` to 1. Grows the bitset if `i` is beyond the current length.
    ///
    /// In JavaScript:
    ///   bs.set(42);  // turn on bit 42
    #[wasm_bindgen]
    pub fn set(&mut self, i: usize) {
        self.inner.set(i);
    }

    /// Clear bit `i` to 0. No effect if `i` is beyond the current length.
    ///
    /// In JavaScript:
    ///   bs.clear(42);  // turn off bit 42
    #[wasm_bindgen]
    pub fn clear(&mut self, i: usize) {
        self.inner.clear(i);
    }

    /// Test whether bit `i` is set (returns true if 1, false if 0).
    ///
    /// In JavaScript:
    ///   if (bs.test(42)) { ... }
    #[wasm_bindgen]
    pub fn test(&self, i: usize) -> bool {
        self.inner.test(i)
    }

    /// Toggle bit `i`: if it is 0, set it to 1; if it is 1, set it to 0.
    /// Grows the bitset if `i` is beyond the current length.
    ///
    /// In JavaScript:
    ///   bs.toggle(7);  // flip bit 7
    #[wasm_bindgen]
    pub fn toggle(&mut self, i: usize) {
        self.inner.toggle(i);
    }

    // -----------------------------------------------------------------------
    // Bitwise operations
    // -----------------------------------------------------------------------
    //
    // Each bitwise operation returns a NEW WasmBitset. The originals are
    // not modified. Both operands must exist but can be different sizes --
    // the inner Bitset handles size mismatches by zero-extending.

    /// Bitwise AND: returns a new bitset where bit `i` is 1 only if it is
    /// 1 in both `self` and `other`.
    ///
    /// Truth table:
    ///   0 AND 0 = 0
    ///   0 AND 1 = 0
    ///   1 AND 0 = 0
    ///   1 AND 1 = 1
    #[wasm_bindgen]
    pub fn and(&self, other: &WasmBitset) -> WasmBitset {
        WasmBitset {
            inner: self.inner.and(&other.inner),
        }
    }

    /// Bitwise OR: returns a new bitset where bit `i` is 1 if it is 1 in
    /// either `self` or `other` (or both).
    ///
    /// Truth table:
    ///   0 OR 0 = 0
    ///   0 OR 1 = 1
    ///   1 OR 0 = 1
    ///   1 OR 1 = 1
    #[wasm_bindgen]
    pub fn or(&self, other: &WasmBitset) -> WasmBitset {
        WasmBitset {
            inner: self.inner.or(&other.inner),
        }
    }

    /// Bitwise XOR: returns a new bitset where bit `i` is 1 if it is set
    /// in exactly one of `self` or `other`, but not both.
    ///
    /// Truth table:
    ///   0 XOR 0 = 0
    ///   0 XOR 1 = 1
    ///   1 XOR 0 = 1
    ///   1 XOR 1 = 0
    #[wasm_bindgen]
    pub fn xor(&self, other: &WasmBitset) -> WasmBitset {
        WasmBitset {
            inner: self.inner.xor(&other.inner),
        }
    }

    /// Bitwise NOT: returns a new bitset where every bit is flipped.
    /// Only flips bits within the logical length -- does not create
    /// spurious 1s beyond the bitset's size.
    #[wasm_bindgen]
    pub fn not(&self) -> WasmBitset {
        WasmBitset {
            inner: self.inner.not(),
        }
    }

    /// AND-NOT (also called "bit clear"): returns a new bitset where bit `i`
    /// is 1 only if it is 1 in `self` AND 0 in `other`.
    ///
    /// This is equivalent to `self AND (NOT other)` but computed in a single
    /// pass without allocating the intermediate NOT result.
    ///
    /// Use case: "remove all elements of set B from set A."
    #[wasm_bindgen(js_name = "andNot")]
    pub fn and_not(&self, other: &WasmBitset) -> WasmBitset {
        WasmBitset {
            inner: self.inner.and_not(&other.inner),
        }
    }

    // -----------------------------------------------------------------------
    // Query operations
    // -----------------------------------------------------------------------

    /// Count the number of set bits (1s) in the bitset.
    ///
    /// This is also known as the "population count" or "Hamming weight."
    /// Modern CPUs have a dedicated POPCNT instruction that counts bits
    /// in a single clock cycle per word.
    ///
    /// In JavaScript:
    ///   bs.popcount();  // 7
    #[wasm_bindgen]
    pub fn popcount(&self) -> usize {
        self.inner.popcount()
    }

    /// Return the logical length of the bitset (number of addressable bits).
    ///
    /// This is NOT the number of set bits -- use `popcount()` for that.
    /// It is the number of bits the bitset tracks, which grows automatically
    /// when you `set()` a bit beyond the current length.
    #[wasm_bindgen(js_name = "len")]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Return the capacity (number of bits that fit without reallocation).
    ///
    /// Always a multiple of 64 (one word = 64 bits).
    #[wasm_bindgen]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Returns true if ANY bit is set to 1.
    #[wasm_bindgen]
    pub fn any(&self) -> bool {
        self.inner.any()
    }

    /// Returns true if ALL bits (within the logical length) are set to 1.
    #[wasm_bindgen]
    pub fn all(&self) -> bool {
        self.inner.all()
    }

    /// Returns true if NO bits are set (all zeros).
    #[wasm_bindgen]
    pub fn none(&self) -> bool {
        self.inner.none()
    }

    /// Returns true if the logical length is 0 (no bits at all).
    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    // -----------------------------------------------------------------------
    // Iteration
    // -----------------------------------------------------------------------

    /// Return a JavaScript array of indices where bits are set to 1.
    ///
    /// This replaces the Rust iterator (`iter_set_bits`) which cannot cross
    /// the WASM boundary directly. The returned value is a JS `Array` of
    /// numbers.
    ///
    /// In JavaScript:
    ///   const bits = bs.iterSetBits();  // [0, 2, 5, 7]
    #[wasm_bindgen(js_name = "iterSetBits")]
    pub fn iter_set_bits(&self) -> JsValue {
        let indices: Vec<usize> = self.inner.iter_set_bits().collect();
        // serde_wasm_bindgen would be cleaner, but to avoid extra deps we
        // build the JS array manually.
        let array = js_sys::Array::new();
        for idx in indices {
            array.push(&JsValue::from_f64(idx as f64));
        }
        array.into()
    }

    // -----------------------------------------------------------------------
    // Conversion
    // -----------------------------------------------------------------------

    /// Convert the bitset to a u64 integer, if it fits.
    ///
    /// Returns the integer as a JS number, or `null` if the bitset has set
    /// bits beyond position 63 (requires more than 64 bits to represent).
    ///
    /// In JavaScript:
    ///   const n = bs.toInteger();  // 42, or null if too large
    #[wasm_bindgen(js_name = "toInteger")]
    pub fn to_integer(&self) -> JsValue {
        match self.inner.to_integer() {
            Some(v) => JsValue::from_f64(v as f64),
            None => JsValue::NULL,
        }
    }

    /// Convert the bitset to a binary string like `"101010"`.
    ///
    /// The leftmost character is the highest bit. An empty bitset returns
    /// an empty string.
    ///
    /// In JavaScript:
    ///   bs.toBinaryStr();  // "101010"
    #[wasm_bindgen(js_name = "toBinaryStr")]
    pub fn to_binary_str(&self) -> String {
        self.inner.to_binary_str()
    }
}

// ---------------------------------------------------------------------------
// Native Rust tests
// ---------------------------------------------------------------------------
//
// These tests run with `cargo test` on the host machine (not in WASM).
// They verify that the WasmBitset wrapper correctly delegates to the inner
// Bitset. WASM-specific tests would require wasm-pack test or a Node.js
// WASM loader, which is a different workflow.
//
// The guard `#[cfg(not(target_arch = "wasm32"))]` ensures these tests are
// excluded when compiling for WASM (where std::test isn't available).

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    // -- Constructor tests --------------------------------------------------

    #[test]
    fn test_new_creates_empty_bitset() {
        let bs = WasmBitset::new(64);
        assert_eq!(bs.len(), 64);
        assert!(!bs.any());
        assert!(bs.none());
    }

    #[test]
    fn test_new_zero_size() {
        let bs = WasmBitset::new(0);
        assert_eq!(bs.len(), 0);
        assert!(bs.is_empty());
    }

    #[test]
    fn test_from_integer() {
        let bs = WasmBitset::from_integer(42); // binary: 101010
        assert!(bs.test(1));
        assert!(bs.test(3));
        assert!(bs.test(5));
        assert!(!bs.test(0));
        assert!(!bs.test(2));
        assert!(!bs.test(4));
    }

    #[test]
    fn test_from_integer_zero() {
        let bs = WasmBitset::from_integer(0);
        assert_eq!(bs.len(), 0);
        assert!(bs.none());
    }

    #[test]
    fn test_from_binary_str_valid() {
        let bs = WasmBitset::from_binary_str("1010").unwrap();
        assert_eq!(bs.len(), 4);
        assert!(bs.test(1));
        assert!(bs.test(3));
        assert!(!bs.test(0));
        assert!(!bs.test(2));
    }

    #[test]
    fn test_from_binary_str_invalid() {
        // We cannot call WasmBitset::from_binary_str with invalid input in
        // native tests because it constructs a JsValue (which panics on
        // non-wasm32 targets). Instead, verify the inner Bitset correctly
        // rejects invalid strings.
        let result = Bitset::from_binary_str("10201");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_binary_str_empty() {
        let bs = WasmBitset::from_binary_str("").unwrap();
        assert_eq!(bs.len(), 0);
    }

    // -- Bit manipulation tests ---------------------------------------------

    #[test]
    fn test_set_and_test() {
        let mut bs = WasmBitset::new(64);
        assert!(!bs.test(10));
        bs.set(10);
        assert!(bs.test(10));
    }

    #[test]
    fn test_set_grows_bitset() {
        let mut bs = WasmBitset::new(8);
        bs.set(100);
        assert!(bs.test(100));
        assert!(bs.len() > 100);
    }

    #[test]
    fn test_clear() {
        let mut bs = WasmBitset::new(64);
        bs.set(5);
        assert!(bs.test(5));
        bs.clear(5);
        assert!(!bs.test(5));
    }

    #[test]
    fn test_toggle() {
        let mut bs = WasmBitset::new(64);
        assert!(!bs.test(3));
        bs.toggle(3);
        assert!(bs.test(3));
        bs.toggle(3);
        assert!(!bs.test(3));
    }

    // -- Bitwise operation tests --------------------------------------------

    #[test]
    fn test_and() {
        let a = WasmBitset::from_binary_str("1100").unwrap();
        let b = WasmBitset::from_binary_str("1010").unwrap();
        let c = a.and(&b);
        assert_eq!(c.to_binary_str(), "1000");
    }

    #[test]
    fn test_or() {
        let a = WasmBitset::from_binary_str("1100").unwrap();
        let b = WasmBitset::from_binary_str("1010").unwrap();
        let c = a.or(&b);
        assert_eq!(c.to_binary_str(), "1110");
    }

    #[test]
    fn test_xor() {
        let a = WasmBitset::from_binary_str("1100").unwrap();
        let b = WasmBitset::from_binary_str("1010").unwrap();
        let c = a.xor(&b);
        // XOR preserves the length, so 4-bit result: "0110"
        assert_eq!(c.to_binary_str(), "0110");
    }

    #[test]
    fn test_not() {
        let a = WasmBitset::from_binary_str("1010").unwrap();
        let b = a.not();
        // NOT preserves the length of the bitset, so a 4-bit "1010"
        // becomes 4-bit "0101".
        assert_eq!(b.to_binary_str(), "0101");
    }

    #[test]
    fn test_and_not() {
        let a = WasmBitset::from_binary_str("1110").unwrap();
        let b = WasmBitset::from_binary_str("1010").unwrap();
        let c = a.and_not(&b);
        // AND-NOT preserves the length, so 4-bit result: "0100"
        assert_eq!(c.to_binary_str(), "0100");
    }

    // -- Query tests --------------------------------------------------------

    #[test]
    fn test_popcount() {
        let bs = WasmBitset::from_integer(0b10100101);
        assert_eq!(bs.popcount(), 4);
    }

    #[test]
    fn test_len_and_capacity() {
        let bs = WasmBitset::new(100);
        assert_eq!(bs.len(), 100);
        // Capacity is rounded up to the next multiple of 64.
        assert!(bs.capacity() >= 100);
        assert_eq!(bs.capacity() % 64, 0);
    }

    #[test]
    fn test_any_all_none() {
        let mut bs = WasmBitset::new(4);
        assert!(bs.none());
        assert!(!bs.any());
        assert!(!bs.all());

        bs.set(0);
        assert!(bs.any());
        assert!(!bs.all());
        assert!(!bs.none());

        bs.set(1);
        bs.set(2);
        bs.set(3);
        assert!(bs.any());
        assert!(bs.all());
        assert!(!bs.none());
    }

    #[test]
    fn test_is_empty() {
        let bs = WasmBitset::new(0);
        assert!(bs.is_empty());

        let bs = WasmBitset::new(1);
        assert!(!bs.is_empty());
    }

    // -- Conversion tests ---------------------------------------------------

    #[test]
    fn test_to_integer() {
        let bs = WasmBitset::from_integer(42);
        // We can't easily unwrap JsValue in native tests, so test via
        // the inner bitset's method directly.
        assert_eq!(bs.inner.to_integer(), Some(42));
    }

    #[test]
    fn test_to_binary_str() {
        let bs = WasmBitset::from_integer(5);
        assert_eq!(bs.to_binary_str(), "101");
    }

    #[test]
    fn test_to_binary_str_empty() {
        let bs = WasmBitset::new(0);
        assert_eq!(bs.to_binary_str(), "");
    }

    // -- Round-trip tests ---------------------------------------------------

    #[test]
    fn test_binary_str_round_trip() {
        let original = "11010110";
        let bs = WasmBitset::from_binary_str(original).unwrap();
        assert_eq!(bs.to_binary_str(), original);
    }

    #[test]
    fn test_integer_round_trip() {
        let bs = WasmBitset::from_integer(255);
        assert_eq!(bs.inner.to_integer(), Some(255));
    }

    // -- Bitwise identity tests ---------------------------------------------

    #[test]
    fn test_and_identity() {
        // A AND A = A
        let a = WasmBitset::from_binary_str("10110").unwrap();
        let result = a.and(&a);
        assert_eq!(result.to_binary_str(), a.to_binary_str());
    }

    #[test]
    fn test_or_identity() {
        // A OR A = A
        let a = WasmBitset::from_binary_str("10110").unwrap();
        let result = a.or(&a);
        assert_eq!(result.to_binary_str(), a.to_binary_str());
    }

    #[test]
    fn test_xor_self_is_zero() {
        // A XOR A = 0
        let a = WasmBitset::from_binary_str("10110").unwrap();
        let result = a.xor(&a);
        assert_eq!(result.popcount(), 0);
    }

    #[test]
    fn test_double_not_is_identity() {
        // NOT(NOT(A)) = A
        let a = WasmBitset::from_binary_str("10110").unwrap();
        let result = a.not().not();
        assert_eq!(result.to_binary_str(), a.to_binary_str());
    }

    // -- Edge case tests ----------------------------------------------------

    #[test]
    fn test_large_bitset() {
        let mut bs = WasmBitset::new(1000);
        bs.set(999);
        assert!(bs.test(999));
        assert_eq!(bs.popcount(), 1);
    }

    #[test]
    fn test_set_clear_toggle_sequence() {
        let mut bs = WasmBitset::new(8);
        bs.set(3);
        bs.set(5);
        bs.clear(3);
        bs.toggle(5);
        bs.toggle(7);
        // Only bit 7 should be set.
        assert!(!bs.test(3));
        assert!(!bs.test(5));
        assert!(bs.test(7));
        assert_eq!(bs.popcount(), 1);
    }
}
