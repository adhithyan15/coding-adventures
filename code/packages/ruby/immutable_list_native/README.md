# coding_adventures_immutable_list_native

A Ruby native extension wrapping the Rust `immutable-list` crate via
`ruby-bridge`. Provides a persistent (immutable) list data structure where
every mutation returns a new list, leaving the original unchanged.

## What is a Persistent Immutable List?

An immutable list is a data structure that never changes after creation.
Operations like `push`, `set`, and `pop` return **new** lists while the
original remains intact. Under the hood, the new and old lists share most of
their internal structure (structural sharing via a 32-way trie), making
mutations efficient -- effectively O(1) for practical sizes.

This is the same design used by Clojure's PersistentVector.

## How It Fits in the Stack

```
Ruby code
  |
  v
coding_adventures_immutable_list_native (this gem)
  |  -- loads immutable_list_native.so/.dll/.bundle
  v
ruby-bridge (Rust crate)
  |  -- raw extern "C" wrappers for Ruby's C API
  v
immutable-list (Rust crate)
  |  -- 32-way trie persistent vector implementation
  v
Ruby interpreter (libruby)
```

## Installation

```ruby
# In your Gemfile
gem "coding_adventures_immutable_list_native"
```

Requires Rust toolchain (`cargo`) for compilation.

## Usage

```ruby
require "coding_adventures_immutable_list_native"

ImmutableList = CodingAdventures::ImmutableListNative::ImmutableList

# Create an empty list
empty = ImmutableList.new
empty.size   # => 0
empty.empty? # => true

# Push returns a new list (original unchanged)
a = empty.push("hello")
b = a.push("world")
empty.size  # => 0 (unchanged!)
a.size      # => 1
b.size      # => 2

# Index access
b.get(0)  # => "hello"
b.get(1)  # => "world"
b.get(99) # => nil (out of bounds)

# Set returns a new list (original unchanged)
c = b.set(0, "hi")
c.get(0)  # => "hi"
b.get(0)  # => "hello" (unchanged!)

# Pop returns [new_list, removed_value]
new_list, val = b.pop
val            # => "world"
new_list.size  # => 1
b.size         # => 2 (unchanged!)

# Build from array
list = ImmutableList.from_array(["a", "b", "c"])
list.to_a  # => ["a", "b", "c"]

# Iteration
list.each { |elem| puts elem }

# Equality
ImmutableList.from_array(["a", "b"]) == ImmutableList.from_array(["a", "b"])
# => true

# String representation
list.inspect  # => "ImmutableList[a, b, c]"
```

## Building

```bash
bundle install
bundle exec rake compile  # Build the Rust extension
bundle exec rake test     # Run tests (compiles first)
```

## API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `ImmutableList.new` | `ImmutableList` | Create an empty list |
| `ImmutableList.from_array(arr)` | `ImmutableList` | Create from a Ruby Array of Strings |
| `push(value)` | `ImmutableList` | Append element, return new list |
| `get(index)` | `String` or `nil` | Access element by index |
| `set(index, value)` | `ImmutableList` | Replace element, return new list |
| `pop` | `[ImmutableList, String]` | Remove last element |
| `size` | `Integer` | Number of elements |
| `empty?` | `Boolean` | True if no elements |
| `to_a` | `Array<String>` | Convert to Ruby Array |
| `each { \|e\| }` | `self` | Iterate over elements |
| `inspect` / `to_s` | `String` | Human-readable representation |
| `==` | `Boolean` | Element-wise equality |

## License

MIT
