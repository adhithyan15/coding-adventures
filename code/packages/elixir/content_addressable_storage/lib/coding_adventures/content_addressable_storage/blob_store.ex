defmodule CodingAdventures.ContentAddressableStorage.BlobStore do
  @moduledoc """
  Behaviour (interface) that any CAS storage backend must implement.

  ## What Is a Behaviour?

  In Elixir, a `behaviour` is a contract — a list of callbacks that a module
  promises to implement. It is analogous to an interface in Java or a trait in
  Rust. Any module that says `@behaviour CodingAdventures.ContentAddressableStorage.BlobStore` and
  implements all four callbacks can be plugged into `CodingAdventures.ContentAddressableStorage.Store`
  as the storage layer.

  ## The Contract

  A `BlobStore` maps 20-byte binary keys to raw byte blobs. The keys are always
  SHA-1 digests computed by the `Store` layer above — implementations treat them
  as opaque identifiers. The store does NOT need to verify hashes; that is the
  `Store` layer's responsibility.

  ```
  put(key, data)         → :ok | {:error, reason}
  get(key)               → {:ok, data} | {:error, reason}
  exists?(key)           → {:ok, boolean} | {:error, reason}
  keys_with_prefix(pfx)  → {:ok, [key]} | {:error, reason}
  ```

  ## Idempotence

  `put/2` must be idempotent: storing the same key twice with the same bytes is
  not an error. Storing a *different* blob under an existing key is undefined
  behaviour — the `Store` layer prevents this by construction (same content always
  produces the same SHA-1 key).

  ## Prefix Lookup

  `keys_with_prefix/1` accepts a byte-string prefix (1–20 bytes) and returns all
  stored keys whose first `byte_size(prefix)` bytes exactly match `prefix`. This
  powers the abbreviated-hash lookup used by Git (`git show a3f4b2`).
  """

  @doc """
  Persist `data` under `key`.

  Must be idempotent: calling `put/3` twice with the same arguments has the same
  effect as calling it once.

  - `store` — the backend struct (e.g., `%LocalDiskStore{}`)
  - `key` — a 20-byte binary (SHA-1 digest produced by the Store layer)
  - `data` — the raw bytes to store
  """
  @callback put(store :: struct(), key :: binary(), data :: binary()) ::
              :ok | {:error, term()}

  @doc """
  Retrieve the blob stored under `key`.

  Returns `{:ok, data}` if found, `{:error, :not_found}` if absent, or
  `{:error, reason}` for I/O failures. Implementations do NOT verify the hash —
  that is the Store layer's job.
  """
  @callback get(store :: struct(), key :: binary()) ::
              {:ok, binary()} | {:error, :not_found} | {:error, term()}

  @doc """
  Check whether `key` is present without fetching the blob.

  Returns `{:ok, true}` if present, `{:ok, false}` if absent, or
  `{:error, reason}` on failure.
  """
  @callback exists?(store :: struct(), key :: binary()) ::
              {:ok, boolean()} | {:error, term()}

  @doc """
  Return all stored keys whose first `byte_size(prefix)` bytes equal `prefix`.

  Used for abbreviated-hash lookup: the caller supplies a byte prefix decoded
  from a short hex string, and the store returns all matching full 20-byte keys.
  The Store layer checks for uniqueness and reports ambiguity.

  - `store` — the backend struct
  - `prefix` — 1 to 20 bytes
  """
  @callback keys_with_prefix(store :: struct(), prefix :: binary()) ::
              {:ok, [binary()]} | {:error, term()}
end
