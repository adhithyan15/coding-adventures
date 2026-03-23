// index.d.ts -- TypeScript type definitions for @coding-adventures/immutable-list-native
// ======================================================================================
//
// These type definitions describe the native ImmutableList class exposed by the
// Rust addon. The actual implementation is in src/lib.rs; these types exist
// so TypeScript consumers get full IntelliSense and type checking.

/**
 * A persistent (immutable) list backed by a 32-way trie with structural sharing.
 *
 * Wraps the Rust `immutable_list::ImmutableList` crate via N-API for native
 * performance. Every "mutation" operation (push, set, pop) returns a *new* list,
 * leaving the original unchanged. The new and old lists share most of their
 * internal memory via reference counting.
 *
 * ## Why immutable?
 *
 * 1. **Safety** -- no iterator invalidation, no data races, no spooky
 *    action-at-a-distance where one function modifies a list another is reading.
 * 2. **Time travel** -- every version persists. Keep a reference to "the list
 *    before the last 50 pushes" and it's still valid, still O(1) to access.
 *
 * ## Construction
 *
 * ```typescript
 * // Create an empty list
 * const empty = new ImmutableList();
 *
 * // Create from an array of strings
 * const list = new ImmutableList(["a", "b", "c"]);
 * ```
 */
export class ImmutableList {
  /**
   * Create a new, empty immutable list.
   */
  constructor();

  /**
   * Create an immutable list from an array of strings.
   *
   * @param items - The initial elements for the list
   */
  constructor(items: string[]);

  // -- Mutation operations (return new lists) --------------------------------

  /**
   * Append an element to the end of the list, returning a new list.
   *
   * The original list is not modified. The new list shares most of its
   * internal structure with the original (structural sharing).
   *
   * Time: O(log32 n) -- effectively O(1) for practical sizes.
   *
   * ```typescript
   * const list1 = new ImmutableList();
   * const list2 = list1.push("hello");
   * list1.length(); // 0 -- original unchanged
   * list2.length(); // 1
   * ```
   */
  push(value: string): ImmutableList;

  /**
   * Get the element at the given index.
   *
   * Returns the string at that position, or `undefined` if the index
   * is out of bounds.
   *
   * Time: O(log32 n) -- effectively O(1).
   *
   * ```typescript
   * const list = new ImmutableList(["a", "b", "c"]);
   * list.get(0); // "a"
   * list.get(5); // undefined
   * ```
   */
  get(index: number): string | undefined;

  /**
   * Replace the element at `index` with `value`, returning a new list.
   *
   * The original list is not modified. Throws if the index is out of bounds.
   *
   * Time: O(log32 n) -- effectively O(1).
   *
   * ```typescript
   * const list = new ImmutableList(["a", "b", "c"]);
   * const updated = list.set(1, "B");
   * list.get(1);    // "b" -- original unchanged
   * updated.get(1); // "B"
   * ```
   */
  set(index: number, value: string): ImmutableList;

  /**
   * Remove the last element, returning a tuple [newList, removedValue].
   *
   * Returns a two-element array: the first element is the new list (with
   * the last element removed), and the second is the removed string.
   * Throws if the list is empty.
   *
   * ```typescript
   * const list = new ImmutableList(["a", "b", "c"]);
   * const [shorter, removed] = list.pop();
   * removed;          // "c"
   * shorter.length();  // 2
   * list.length();     // 3 -- original unchanged
   * ```
   */
  pop(): [ImmutableList, string];

  // -- Query operations ------------------------------------------------------

  /**
   * Return the number of elements in the list.
   *
   * Time: O(1).
   */
  length(): number;

  /**
   * Return true if the list has zero elements.
   *
   * Time: O(1).
   */
  isEmpty(): boolean;

  // -- Conversion ------------------------------------------------------------

  /**
   * Collect all elements into a plain JavaScript array.
   *
   * Time: O(n).
   *
   * ```typescript
   * const list = new ImmutableList(["a", "b", "c"]);
   * list.toArray(); // ["a", "b", "c"]
   * ```
   */
  toArray(): string[];
}
