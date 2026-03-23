# @coding-adventures/immutable-list-native

Native Node.js addon wrapping the Rust `immutable-list` crate via the `node-bridge` FFI layer. Provides a persistent (immutable) list data structure with near-constant-time push, get, set, and pop operations.

## What is an Immutable List?

An immutable list is a persistent data structure -- once created, it never changes. Every "modification" (push, set, pop) returns a *new* list, leaving the original intact. This sounds wasteful, but the trick is **structural sharing**: the new list reuses most of the old list's memory, only allocating new nodes along the path that changed.

The implementation uses a 32-way trie with a tail buffer (the same design as Clojure's PersistentVector). This gives O(log32 n) performance for all operations, which is effectively O(1) for practical sizes -- a million-element list is only 4 levels deep.

## How it fits in the stack

```
┌─────────────────────────────────────────┐
│  JavaScript / TypeScript (your code)    │
├─────────────────────────────────────────┤
│  index.js (ESM loader)                  │
├─────────────────────────────────────────┤
│  src/lib.rs (N-API glue via node-bridge)│
├─────────────────────────────────────────┤
│  immutable-list (Rust crate)            │
│  node-bridge (N-API FFI crate)          │
└─────────────────────────────────────────┘
```

- **immutable-list** -- Pure Rust implementation of the persistent vector
- **node-bridge** -- Zero-dependency Rust wrapper for Node.js N-API
- **src/lib.rs** -- Glue code that wraps ImmutableList methods as N-API callbacks
- **index.js** -- ESM loader that imports the compiled `.node` binary

## Usage

```typescript
import { ImmutableList } from "@coding-adventures/immutable-list-native";

// Create from an array
const list = new ImmutableList(["a", "b", "c"]);

// Push returns a NEW list (original unchanged)
const longer = list.push("d");
console.log(list.length());   // 3
console.log(longer.length()); // 4

// Get by index
console.log(longer.get(0)); // "a"
console.log(longer.get(3)); // "d"

// Set returns a NEW list
const updated = list.set(1, "B");
console.log(list.get(1));    // "b" (original)
console.log(updated.get(1)); // "B"

// Pop returns [newList, removedValue]
const [shorter, removed] = list.pop();
console.log(removed);          // "c"
console.log(shorter.length()); // 2

// Convert to a plain JS array
console.log(list.toArray()); // ["a", "b", "c"]
```

## Building

Requires Rust and Node.js.

```bash
# Install JS dev dependencies
npm ci

# Build the native addon
cargo build --release

# Copy the binary (platform-specific)
# Linux:
cp target/release/libimmutable_list_native_node.so immutable_list_native_node.node
# macOS:
cp target/release/libimmutable_list_native_node.dylib immutable_list_native_node.node
# Windows:
copy target\release\immutable_list_native_node.dll immutable_list_native_node.node

# Run tests
npx vitest run
```

## API Reference

### `new ImmutableList()`
Create an empty list.

### `new ImmutableList(items: string[])`
Create a list from an array of strings.

### `list.push(value: string): ImmutableList`
Append an element, returning a new list.

### `list.get(index: number): string | undefined`
Get element at index. Returns undefined if out of bounds.

### `list.set(index: number, value: string): ImmutableList`
Replace element at index, returning a new list. Throws if out of bounds.

### `list.pop(): [ImmutableList, string]`
Remove last element. Returns [newList, removedValue]. Throws if empty.

### `list.length(): number`
Return the number of elements.

### `list.isEmpty(): boolean`
Return true if the list has zero elements.

### `list.toArray(): string[]`
Collect all elements into a plain JS array.
