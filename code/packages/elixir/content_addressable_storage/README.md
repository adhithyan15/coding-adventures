# coding-adventures-content-addressable-storage (Elixir)

Content-Addressable Storage (CAS) for the coding-adventures monorepo — an Elixir
port of the [Rust `cas` crate](../../rust/content_addressable_storage/).

## What Is CAS?

In ordinary storage you look up content by a name you assign. CAS flips this:
the *hash of the content* is the address.

```
Traditional:  name  ──►  content   (name can lie; content can change)
CAS:          hash  ──►  content   (hash is derived from content; cannot lie)
```

The hash is simultaneously the lookup key and an integrity check. If the stored
bytes don't hash to the address you requested, the store is corrupt — caught
automatically on every read.

Git's entire object model works this way. Every blob, tree, commit, and tag is
stored by its SHA-1 hash. Two identical files share one object. Renaming a file
costs zero storage.

## Package Structure

| Module | Role |
|---|---|
| `CodingAdventures.ContentAddressableStorage.BlobStore` | Behaviour (interface) for storage backends |
| `CodingAdventures.ContentAddressableStorage.Store` | CAS logic: SHA-1 hashing, integrity, prefix search |
| `CodingAdventures.ContentAddressableStorage.LocalDiskStore` | Filesystem backend (Git 2/38 fanout layout) |
| `CodingAdventures.ContentAddressableStorage.Hex` | Hex encode/decode utilities |
| `CodingAdventures.ContentAddressableStorage.Error` | Typed error reason catalogue |

## Quick Start

```elixir
alias CodingAdventures.ContentAddressableStorage.{Store, LocalDiskStore, Hex}

# Create a filesystem-backed store
backend = LocalDiskStore.new!("/tmp/objects")
store   = Store.new(backend)

# Store some content — the key is the SHA-1 hash of the data
{:ok, key} = Store.put(store, "hello, world")

# Retrieve and verify integrity in one step
{:ok, data} = Store.get(store, key)
# => "hello, world"

# Check existence without fetching
{:ok, true} = Store.exists?(store, key)

# Abbreviated lookup (like `git show a3f4b2`)
short = String.slice(Hex.key_to_hex(key), 0, 8)
{:ok, full_key} = Store.find_by_prefix(store, short)
# full_key == key
```

## Writing a Custom Backend

Implement the `CodingAdventures.ContentAddressableStorage.BlobStore` behaviour:

```elixir
defmodule MyStore do
  @behaviour CodingAdventures.ContentAddressableStorage.BlobStore

  @impl true
  def put(%__MODULE__{} = store, key, data), do: ...

  @impl true
  def get(%__MODULE__{} = store, key), do: ...

  @impl true
  def exists?(%__MODULE__{} = store, key), do: ...

  @impl true
  def keys_with_prefix(%__MODULE__{} = store, prefix), do: ...
end

store = CodingAdventures.ContentAddressableStorage.Store.new(%MyStore{...})
```

## Filesystem Layout

`LocalDiskStore` uses the Git 2/38 fanout layout:

```
<root>/
  a3/
    f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5   ← 38-char hex remainder
  da/
    39a3ee5e6b4b0d3255bfef95601890afd80709
```

The first byte of the SHA-1 hash becomes a 2-char directory name. This creates
up to 256 sub-directories, keeping each to a manageable size even in large
repos (at 1 million objects, ~3900 files per bucket).

Writes are atomic: data goes to a temp file first, then `File.rename/2` moves
it into place. On POSIX systems `rename` is guaranteed atomic.

## Error Handling

All public functions return `{:ok, value}` or `{:error, reason}`:

| Reason | Meaning |
|---|---|
| `:not_found` | Key is not in the store |
| `{:corrupted, key}` | Stored bytes don't hash to the key |
| `{:ambiguous_prefix, prefix}` | Hex prefix matched 2+ keys |
| `{:prefix_not_found, prefix}` | Hex prefix matched 0 keys |
| `{:invalid_prefix, prefix}` | Hex prefix is not valid hexadecimal |
| `{:store_error, reason}` | Backend I/O failure |

## Dependencies

- [`coding-adventures-sha1`](../sha1/) — SHA-1 implementation (no stdlib crypto)

## Running Tests

```sh
mix deps.get
mix test
```
