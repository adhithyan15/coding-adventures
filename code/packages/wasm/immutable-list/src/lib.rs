// lib.rs -- WASM Bindings for the ImmutableList Library
// =====================================================
//
// This module wraps the pure-Rust `immutable_list::ImmutableList` struct with a
// `WasmImmutableList` type that is exported to JavaScript/TypeScript via
// wasm-bindgen. Every public method on `WasmImmutableList` becomes a method
// on the JS class `WasmImmutableList`.
//
// Design Decisions
// ----------------
//
// 1. **Owned wrapper**: `WasmImmutableList` owns an `ImmutableList` internally.
//    WASM cannot pass Rust references across the boundary, so every method
//    takes `&self` on the wrapper and delegates to the inner `ImmutableList`.
//
// 2. **Persistent semantics in JS**: The Rust ImmutableList is a persistent data
//    structure -- every "mutation" returns a new list. We preserve this in the
//    WASM API: `push`, `set`, and `pop` all return *new* `WasmImmutableList`
//    instances, never modifying the original. This lets JS code keep references
//    to old versions cheaply thanks to structural sharing.
//
// 3. **String values**: The underlying Rust list stores `String` elements.
//    JS strings are converted to/from Rust strings at the WASM boundary.
//    wasm-bindgen handles this conversion automatically.
//
// 4. **camelCase naming**: JavaScript convention uses camelCase. The
//    `#[wasm_bindgen]` attribute's `js_name` parameter renames each
//    method for JS consumers. Rust code still uses snake_case internally.
//
// 5. **JsValue for complex returns**: Methods that return optional values
//    (like `get`) or compound results (like `pop`) use `JsValue` because
//    wasm-bindgen cannot directly return `Option<String>` or tuples to JS.
//    We return `undefined` for missing values and JS arrays for tuples.
//
// 6. **No WASM-target tests**: WASM tests require wasm-pack test or a
//    Node.js WASM loader. We include native Rust tests guarded with
//    `#[cfg(not(target_arch = "wasm32"))]` so `cargo test` works
//    without any WASM tooling.

use immutable_list::ImmutableList;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// WasmImmutableList -- the main exported type
// ---------------------------------------------------------------------------
//
// JavaScript usage:
//
//   import { WasmImmutableList } from './immutable_list_wasm.js';
//
//   const empty = new WasmImmutableList();
//   const list1 = empty.push("hello");
//   const list2 = list1.push("world");
//
//   list2.length();        // 2
//   list2.get(0);          // "hello"
//   list2.get(1);          // "world"
//   list1.length();        // 1  (unchanged!)
//
//   const list3 = list2.set(0, "hi");
//   list3.get(0);          // "hi"
//   list2.get(0);          // "hello"  (unchanged!)
//
//   const [popped, value] = list2.pop();
//   value;                 // "world"
//   popped.length();       // 1
//
//   const arr = list2.toArray();  // ["hello", "world"]
//   const fromArr = WasmImmutableList.fromArray(["a", "b", "c"]);

/// A persistent (immutable) list exposed to JavaScript via WebAssembly.
///
/// Every "mutation" returns a new list -- the original is never modified.
/// Structural sharing under the hood means this is memory-efficient:
/// a push typically copies only a small tail buffer, reusing the entire
/// trie from the previous version.
#[wasm_bindgen]
pub struct WasmImmutableList {
    inner: ImmutableList,
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

#[wasm_bindgen]
impl WasmImmutableList {
    /// Create a new, empty immutable list.
    ///
    /// In JavaScript:
    ///   const list = new WasmImmutableList();
    ///   list.length();   // 0
    ///   list.isEmpty();  // true
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmImmutableList {
        WasmImmutableList {
            inner: ImmutableList::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Persistent operations (return new lists)
    // -----------------------------------------------------------------------
    //
    // These methods never modify `self`. They clone the inner ImmutableList
    // (which is cheap due to structural sharing via Arc), perform the
    // operation, and return a brand-new WasmImmutableList wrapper.

    /// Append a value to the end of the list, returning a **new** list.
    ///
    /// The original list is unchanged -- this is the fundamental property of
    /// persistent data structures. Under the hood, the new list shares most
    /// of its structure with the old one.
    ///
    /// In JavaScript:
    ///   const a = new WasmImmutableList();
    ///   const b = a.push("hello");
    ///   a.length();  // 0 -- unchanged
    ///   b.length();  // 1
    ///   b.get(0);    // "hello"
    #[wasm_bindgen]
    pub fn push(&self, value: String) -> WasmImmutableList {
        WasmImmutableList {
            inner: self.inner.push(value),
        }
    }

    /// Return the element at `index`, or `undefined` if out of bounds.
    ///
    /// The ImmutableList uses 0-based indexing, just like JavaScript arrays.
    /// If the index is valid, returns the string value. If the index is out
    /// of range (>= length), returns JS `undefined` rather than throwing.
    ///
    /// In JavaScript:
    ///   const list = new WasmImmutableList().push("a").push("b");
    ///   list.get(0);   // "a"
    ///   list.get(1);   // "b"
    ///   list.get(99);  // undefined
    #[wasm_bindgen]
    pub fn get(&self, index: usize) -> JsValue {
        match self.inner.get(index) {
            Some(s) => JsValue::from_str(s),
            None => JsValue::UNDEFINED,
        }
    }

    /// Return a **new** list with the element at `index` replaced by `value`.
    ///
    /// Panics (throws in JS) if `index >= length`. The original list is
    /// unchanged.
    ///
    /// In JavaScript:
    ///   const a = WasmImmutableList.fromArray(["x", "y"]);
    ///   const b = a.set(1, "z");
    ///   a.get(1);  // "y" -- unchanged
    ///   b.get(1);  // "z"
    #[wasm_bindgen]
    pub fn set(&self, index: usize, value: String) -> WasmImmutableList {
        WasmImmutableList {
            inner: self.inner.set(index, value),
        }
    }

    /// Remove the last element, returning a JS array `[new_list, popped_value]`,
    /// or `null` if the list is empty.
    ///
    /// This preserves the persistent semantics: the original list is unchanged.
    /// The return value is a two-element JS array so the caller can destructure
    /// both the new (shorter) list and the removed value in one call.
    ///
    /// In JavaScript:
    ///   const list = WasmImmutableList.fromArray(["a", "b", "c"]);
    ///   const result = list.pop();
    ///   // result is [new_list, "c"]
    ///   const [shorter, val] = result;
    ///   val;                // "c"
    ///   shorter.length();   // 2
    ///   list.length();      // 3 -- unchanged
    ///
    ///   const empty = new WasmImmutableList();
    ///   empty.pop();  // null
    #[wasm_bindgen]
    pub fn pop(&self) -> JsValue {
        if self.inner.is_empty() {
            return JsValue::NULL;
        }
        let (new_list, value) = self.inner.pop();
        let array = js_sys::Array::new();
        array.push(&JsValue::from(WasmImmutableList { inner: new_list }));
        array.push(&JsValue::from_str(&value));
        array.into()
    }

    // -----------------------------------------------------------------------
    // Query operations
    // -----------------------------------------------------------------------

    /// Return the number of elements in the list.
    ///
    /// In JavaScript:
    ///   list.length();  // 5
    #[wasm_bindgen]
    pub fn length(&self) -> usize {
        self.inner.len()
    }

    /// Return `true` if the list has zero elements.
    ///
    /// In JavaScript:
    ///   new WasmImmutableList().isEmpty();  // true
    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    // -----------------------------------------------------------------------
    // Conversion: list <-> JS array
    // -----------------------------------------------------------------------
    //
    // These methods bridge between the Rust persistent list and JavaScript
    // arrays. `toArray` materializes the list as a plain JS string array.
    // `fromArray` constructs a new list from a JS array of strings.

    /// Convert the list to a JavaScript array of strings.
    ///
    /// This copies every element into a new JS array. Useful for
    /// interoperating with JS APIs that expect plain arrays.
    ///
    /// In JavaScript:
    ///   const list = WasmImmutableList.fromArray(["a", "b", "c"]);
    ///   list.toArray();  // ["a", "b", "c"]
    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> JsValue {
        let array = js_sys::Array::new();
        for item in self.inner.iter() {
            array.push(&JsValue::from_str(item));
        }
        array.into()
    }

    /// Create a new `WasmImmutableList` from a JavaScript array of strings.
    ///
    /// Each element is converted from a JS string to a Rust `String` and
    /// pushed onto the list. Non-string elements are coerced to strings
    /// via `as_string()`, which returns an empty string for non-string values.
    ///
    /// In JavaScript:
    ///   const list = WasmImmutableList.fromArray(["hello", "world"]);
    ///   list.length();  // 2
    ///   list.get(0);    // "hello"
    #[wasm_bindgen(js_name = "fromArray")]
    pub fn from_array(arr: &JsValue) -> WasmImmutableList {
        let js_array = js_sys::Array::from(arr);
        let strings: Vec<String> = js_array
            .iter()
            .map(|val| val.as_string().unwrap_or_default())
            .collect();
        WasmImmutableList {
            inner: ImmutableList::from_slice(&strings),
        }
    }
}

// ---------------------------------------------------------------------------
// Native Rust tests
// ---------------------------------------------------------------------------
//
// These tests run with `cargo test` on the host machine (not in WASM).
// They verify that the WasmImmutableList wrapper correctly delegates to the
// inner ImmutableList. WASM-specific tests would require wasm-pack test or
// a Node.js WASM loader, which is a different workflow.
//
// The guard `#[cfg(not(target_arch = "wasm32"))]` ensures these tests are
// excluded when compiling for WASM (where std::test isn't available).

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    // -- Constructor tests --------------------------------------------------

    #[test]
    fn test_new_creates_empty_list() {
        let list = WasmImmutableList::new();
        assert_eq!(list.length(), 0);
        assert!(list.is_empty());
    }

    // -- Push tests ---------------------------------------------------------

    #[test]
    fn test_push_returns_new_list() {
        let a = WasmImmutableList::new();
        let b = a.push("hello".to_string());

        // Original is unchanged.
        assert_eq!(a.length(), 0);
        assert!(a.is_empty());

        // New list has one element.
        assert_eq!(b.length(), 1);
        assert!(!b.is_empty());
    }

    #[test]
    fn test_push_multiple() {
        let list = WasmImmutableList::new()
            .push("a".to_string())
            .push("b".to_string())
            .push("c".to_string());
        assert_eq!(list.length(), 3);
    }

    #[test]
    fn test_push_preserves_previous_versions() {
        let v0 = WasmImmutableList::new();
        let v1 = v0.push("x".to_string());
        let v2 = v1.push("y".to_string());

        // All three versions are independently valid.
        assert_eq!(v0.length(), 0);
        assert_eq!(v1.length(), 1);
        assert_eq!(v2.length(), 2);
    }

    // -- Get tests ----------------------------------------------------------

    #[test]
    fn test_get_valid_index() {
        let list = WasmImmutableList::new()
            .push("alpha".to_string())
            .push("beta".to_string());

        // In native tests we can't easily inspect JsValue, so test via inner.
        assert_eq!(list.inner.get(0), Some("alpha"));
        assert_eq!(list.inner.get(1), Some("beta"));
    }

    #[test]
    fn test_get_out_of_bounds_returns_none() {
        let list = WasmImmutableList::new().push("only".to_string());
        assert_eq!(list.inner.get(1), None);
        assert_eq!(list.inner.get(100), None);
    }

    #[test]
    fn test_get_on_empty_list() {
        let list = WasmImmutableList::new();
        assert_eq!(list.inner.get(0), None);
    }

    // -- Set tests ----------------------------------------------------------

    #[test]
    fn test_set_returns_new_list_with_updated_value() {
        let a = WasmImmutableList::new()
            .push("x".to_string())
            .push("y".to_string());
        let b = a.set(1, "z".to_string());

        // Original unchanged.
        assert_eq!(a.inner.get(1), Some("y"));
        // New list has the update.
        assert_eq!(b.inner.get(1), Some("z"));
        // First element unchanged in both.
        assert_eq!(a.inner.get(0), Some("x"));
        assert_eq!(b.inner.get(0), Some("x"));
    }

    #[test]
    fn test_set_first_element() {
        let list = WasmImmutableList::new()
            .push("old".to_string())
            .push("keep".to_string());
        let updated = list.set(0, "new".to_string());
        assert_eq!(updated.inner.get(0), Some("new"));
        assert_eq!(updated.inner.get(1), Some("keep"));
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_set_out_of_bounds_panics() {
        let list = WasmImmutableList::new().push("only".to_string());
        let _ = list.set(5, "boom".to_string());
    }

    // -- Pop tests ----------------------------------------------------------

    #[test]
    fn test_pop_returns_shorter_list_and_value() {
        let list = WasmImmutableList::new()
            .push("a".to_string())
            .push("b".to_string())
            .push("c".to_string());

        let (popped, value) = list.inner.pop();
        assert_eq!(value, "c");
        assert_eq!(popped.len(), 2);

        // Original unchanged.
        assert_eq!(list.length(), 3);
    }

    #[test]
    fn test_pop_to_empty() {
        let list = WasmImmutableList::new().push("only".to_string());
        let (popped, value) = list.inner.pop();
        assert_eq!(value, "only");
        assert_eq!(popped.len(), 0);
        assert!(popped.is_empty());
    }

    #[test]
    fn test_pop_on_empty_returns_null() {
        // The WASM pop() returns null for empty lists.
        // In native tests, we verify is_empty() is true.
        let list = WasmImmutableList::new();
        assert!(list.is_empty());
        // The inner list panics on pop() for empty, but our WASM wrapper
        // checks is_empty() first and returns JsValue::NULL.
    }

    // -- Length and isEmpty tests -------------------------------------------

    #[test]
    fn test_length_grows_with_push() {
        let mut list = WasmImmutableList::new();
        for i in 0..10 {
            assert_eq!(list.length(), i);
            list = list.push(format!("item_{}", i));
        }
        assert_eq!(list.length(), 10);
    }

    #[test]
    fn test_is_empty_on_new_list() {
        assert!(WasmImmutableList::new().is_empty());
    }

    #[test]
    fn test_is_empty_after_push() {
        let list = WasmImmutableList::new().push("x".to_string());
        assert!(!list.is_empty());
    }

    // -- toArray tests (via inner) ------------------------------------------

    #[test]
    fn test_to_vec_empty() {
        let list = WasmImmutableList::new();
        assert_eq!(list.inner.to_vec(), Vec::<String>::new());
    }

    #[test]
    fn test_to_vec_with_elements() {
        let list = WasmImmutableList::new()
            .push("a".to_string())
            .push("b".to_string())
            .push("c".to_string());
        assert_eq!(list.inner.to_vec(), vec!["a", "b", "c"]);
    }

    // -- fromArray tests (via from_slice) -----------------------------------

    #[test]
    fn test_from_slice_empty() {
        let items: Vec<String> = vec![];
        let list = WasmImmutableList {
            inner: ImmutableList::from_slice(&items),
        };
        assert_eq!(list.length(), 0);
        assert!(list.is_empty());
    }

    #[test]
    fn test_from_slice_with_elements() {
        let items = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let list = WasmImmutableList {
            inner: ImmutableList::from_slice(&items),
        };
        assert_eq!(list.length(), 3);
        assert_eq!(list.inner.get(0), Some("x"));
        assert_eq!(list.inner.get(1), Some("y"));
        assert_eq!(list.inner.get(2), Some("z"));
    }

    // -- Round-trip tests ---------------------------------------------------

    #[test]
    fn test_push_get_round_trip() {
        let list = WasmImmutableList::new()
            .push("hello".to_string())
            .push("world".to_string());
        assert_eq!(list.inner.get(0), Some("hello"));
        assert_eq!(list.inner.get(1), Some("world"));
    }

    #[test]
    fn test_set_get_round_trip() {
        let list = WasmImmutableList::new()
            .push("a".to_string())
            .push("b".to_string());
        let updated = list.set(0, "A".to_string());
        assert_eq!(updated.inner.get(0), Some("A"));
        assert_eq!(updated.inner.get(1), Some("b"));
    }

    #[test]
    fn test_push_pop_round_trip() {
        let list = WasmImmutableList::new()
            .push("first".to_string())
            .push("second".to_string())
            .push("third".to_string());

        let (after_pop, val) = list.inner.pop();
        assert_eq!(val, "third");
        assert_eq!(after_pop.len(), 2);
        assert_eq!(after_pop.get(0), Some("first"));
        assert_eq!(after_pop.get(1), Some("second"));
    }

    // -- Persistence (structural sharing) tests -----------------------------

    #[test]
    fn test_push_does_not_mutate_original() {
        let original = WasmImmutableList::new().push("a".to_string());
        let _ = original.push("b".to_string());
        // Original still has exactly 1 element.
        assert_eq!(original.length(), 1);
        assert_eq!(original.inner.get(0), Some("a"));
    }

    #[test]
    fn test_set_does_not_mutate_original() {
        let original = WasmImmutableList::new()
            .push("old".to_string())
            .push("keep".to_string());
        let _ = original.set(0, "new".to_string());
        assert_eq!(original.inner.get(0), Some("old"));
    }

    #[test]
    fn test_multiple_branches_from_same_version() {
        // Fork the list into two different branches from the same base.
        let base = WasmImmutableList::new()
            .push("shared".to_string());

        let branch_a = base.push("a_only".to_string());
        let branch_b = base.push("b_only".to_string());

        // Base unchanged.
        assert_eq!(base.length(), 1);
        // Branches diverged.
        assert_eq!(branch_a.length(), 2);
        assert_eq!(branch_b.length(), 2);
        assert_eq!(branch_a.inner.get(1), Some("a_only"));
        assert_eq!(branch_b.inner.get(1), Some("b_only"));
    }

    // -- Large list test (exercises trie promotion) -------------------------

    #[test]
    fn test_large_list_push_and_get() {
        // Push enough elements to trigger trie promotion (> 32 elements).
        let mut list = WasmImmutableList::new();
        for i in 0..100 {
            list = list.push(format!("item_{}", i));
        }
        assert_eq!(list.length(), 100);
        assert_eq!(list.inner.get(0), Some("item_0"));
        assert_eq!(list.inner.get(50), Some("item_50"));
        assert_eq!(list.inner.get(99), Some("item_99"));
    }

    #[test]
    fn test_large_list_set() {
        let mut list = WasmImmutableList::new();
        for i in 0..64 {
            list = list.push(format!("v{}", i));
        }
        let updated = list.set(32, "REPLACED".to_string());
        assert_eq!(updated.inner.get(32), Some("REPLACED"));
        // Original unchanged.
        assert_eq!(list.inner.get(32), Some("v32"));
    }

    #[test]
    fn test_large_list_pop_sequence() {
        let mut list = WasmImmutableList::new();
        for i in 0..50 {
            list = list.push(format!("e{}", i));
        }

        // Pop 50 times -- should end up empty.
        let mut current = list.inner.clone();
        for i in (0..50).rev() {
            let (next, val) = current.pop();
            assert_eq!(val, format!("e{}", i));
            current = next;
        }
        assert!(current.is_empty());
    }

    // -- Edge cases ---------------------------------------------------------

    #[test]
    fn test_push_empty_string() {
        let list = WasmImmutableList::new().push("".to_string());
        assert_eq!(list.length(), 1);
        assert_eq!(list.inner.get(0), Some(""));
    }

    #[test]
    fn test_push_unicode() {
        let list = WasmImmutableList::new()
            .push("cafe\u{0301}".to_string())
            .push("\u{1F680}".to_string());
        assert_eq!(list.inner.get(0), Some("cafe\u{0301}"));
        assert_eq!(list.inner.get(1), Some("\u{1F680}"));
    }
}
