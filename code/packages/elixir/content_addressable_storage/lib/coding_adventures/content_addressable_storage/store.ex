defmodule CodingAdventures.ContentAddressableStorage.Store do
  @moduledoc """
  Content-Addressable Store — wraps a `BlobStore` backend with SHA-1 hashing,
  integrity verification, and prefix-based key resolution.

  ## Mental Model

  Imagine a library where every book's call number *is* a cryptographic
  fingerprint of the book's text. You cannot file a different book under that
  number. And if someone swaps pages, the fingerprint changes and the librarian
  catches it before handing the book over.

  ```
  Traditional storage:  name  ──►  content   (name can lie; content can change)
  Content-addressed:    hash  ──►  content   (hash is derived from content; cannot lie)
  ```

  ## What This Module Adds

  A raw `BlobStore` maps keys to bytes. `Store` adds three things the raw store
  cannot provide on its own:

  1. **Automatic keying** — callers supply content; SHA-1 is computed internally.
     You never choose the key; the content dictates it.

  2. **Integrity verification on read** — after `blob_store.get(key)`, `Store`
     re-hashes the returned bytes. If they don't match `key`, the data is corrupt
     and `{:error, {:corrupted, key}}` is returned before the caller ever sees
     the bytes.

  3. **Prefix resolution** — translates abbreviated hex strings (like Git's
     `a3f4b2`) to full 20-byte keys, with proper "not found" / "ambiguous"
     discrimination.

  ## Struct

  `%Store{backend: blob_store_struct}` where `backend` is any struct implementing
  the `CodingAdventures.ContentAddressableStorage.BlobStore` behaviour.

      store = CodingAdventures.ContentAddressableStorage.Store.new(
        CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!("/tmp/objects")
      )

  ## Return Values

  All public functions return `{:ok, value}` on success and `{:error, reason}`
  on failure. See `CodingAdventures.ContentAddressableStorage.Error` for the full reason catalogue.
  """

  alias CodingAdventures.ContentAddressableStorage.Hex
  alias CodingAdventures.Sha1

  # The store wraps any BlobStore implementation.  We store the backend struct
  # directly — no process, no GenServer — just plain data and function calls.
  @enforce_keys [:backend]
  defstruct [:backend]

  @type t :: %__MODULE__{backend: struct()}

  @doc """
  Create a new `Store` wrapping `backend`.

  `backend` must be a struct whose module implements the
  `CodingAdventures.ContentAddressableStorage.BlobStore` behaviour.

  ## Examples

      iex> backend = CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!(System.tmp_dir!() <> "/content_addressable_storage-new")
      iex> store = CodingAdventures.ContentAddressableStorage.Store.new(backend)
      iex> is_struct(store, CodingAdventures.ContentAddressableStorage.Store)
      true
  """
  @spec new(struct()) :: t()
  def new(backend) when is_struct(backend) do
    %__MODULE__{backend: backend}
  end

  @doc """
  Hash `data` with SHA-1, store it in the backend, and return the 20-byte key.

  This operation is idempotent: if the same content has already been stored, the
  existing key is returned and no write is performed (the backend handles this).

  ## Examples

      iex> backend = CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!(System.tmp_dir!() <> "/content_addressable_storage-put")
      iex> store = CodingAdventures.ContentAddressableStorage.Store.new(backend)
      iex> {:ok, key1} = CodingAdventures.ContentAddressableStorage.Store.put(store, "hello")
      iex> {:ok, key2} = CodingAdventures.ContentAddressableStorage.Store.put(store, "hello")
      iex> key1 == key2
      true
  """
  @spec put(t(), binary()) :: {:ok, binary()} | {:error, term()}
  def put(%__MODULE__{backend: backend}, data) when is_binary(data) do
    # Compute the SHA-1 digest of the content. This is the canonical key.
    key = Sha1.sha1(data)

    # Delegate the raw write to the backend. The BlobStore contract requires
    # idempotence, so no pre-check is needed — calling put twice with the same
    # key is always safe. Skipping the exists?/put two-step also eliminates
    # a TOCTOU window.
    backend_module = backend.__struct__

    case backend_module.put(backend, key, data) do
      :ok -> {:ok, key}
      {:error, reason} -> {:error, wrap_store_error(reason)}
    end
  end

  @doc """
  Retrieve the blob stored under `key` and verify its integrity.

  The returned bytes are guaranteed to re-hash to `key`. If the store returns
  bytes that don't match, `{:error, {:corrupted, key}}` is returned instead of
  the corrupt data.

  ## Examples

      iex> backend = CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!(System.tmp_dir!() <> "/content_addressable_storage-get")
      iex> store = CodingAdventures.ContentAddressableStorage.Store.new(backend)
      iex> {:ok, key} = CodingAdventures.ContentAddressableStorage.Store.put(store, "hello, world")
      iex> CodingAdventures.ContentAddressableStorage.Store.get(store, key)
      {:ok, "hello, world"}
  """
  @spec get(t(), binary()) :: {:ok, binary()} | {:error, term()}
  def get(%__MODULE__{backend: backend}, key)
      when is_binary(key) and byte_size(key) == 20 do
    backend_module = backend.__struct__

    case backend_module.get(backend, key) do
      {:error, :not_found} ->
        {:error, :not_found}

      {:error, reason} ->
        {:error, wrap_store_error(reason)}

      {:ok, data} ->
        # Integrity check: re-hash the returned bytes.
        #
        # If the stored file was corrupted (bit rot, truncation, accidental edit),
        # the digest won't match and we surface a typed :corrupted error rather
        # than silently returning bad data to the caller.
        actual_key = Sha1.sha1(data)

        if actual_key == key do
          {:ok, data}
        else
          {:error, {:corrupted, key}}
        end
    end
  end

  @doc """
  Check whether `key` is present in the store.

  Returns `{:ok, true}` or `{:ok, false}` without fetching the blob.
  """
  @spec exists?(t(), binary()) :: {:ok, boolean()} | {:error, term()}
  def exists?(%__MODULE__{backend: backend}, key)
      when is_binary(key) and byte_size(key) == 20 do
    backend_module = backend.__struct__

    case backend_module.exists?(backend, key) do
      {:ok, result} -> {:ok, result}
      {:error, reason} -> {:error, wrap_store_error(reason)}
    end
  end

  @doc """
  Resolve an abbreviated hex string to a full 20-byte key.

  Accepts any non-empty hex string of 1–40 characters. Odd-length strings are
  treated as nibble prefixes (e.g., `"a3f"` matches any key starting with
  `0xa3, 0xf_`).

  ## Error Reasons

  - `{:error, {:invalid_prefix, hex}}` — empty string or non-hex characters
  - `{:error, {:prefix_not_found, hex}}` — no keys match
  - `{:error, {:ambiguous_prefix, hex}}` — two or more keys match
  - `{:error, {:store_error, reason}}` — backend I/O failure

  ## Examples

      iex> backend = CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!(System.tmp_dir!() <> "/content_addressable_storage-prefix")
      iex> store = CodingAdventures.ContentAddressableStorage.Store.new(backend)
      iex> {:ok, key} = CodingAdventures.ContentAddressableStorage.Store.put(store, "hello")
      iex> short = String.slice(CodingAdventures.ContentAddressableStorage.Hex.key_to_hex(key), 0, 8)
      iex> {:ok, found} = CodingAdventures.ContentAddressableStorage.Store.find_by_prefix(store, short)
      iex> found == key
      true
  """
  @spec find_by_prefix(t(), String.t()) :: {:ok, binary()} | {:error, term()}
  def find_by_prefix(%__MODULE__{backend: backend}, hex_prefix)
      when is_binary(hex_prefix) do
    # Step 1: Validate and decode the hex prefix to bytes.
    case Hex.decode_hex_prefix(hex_prefix) do
      {:error, :invalid_hex} ->
        {:error, {:invalid_prefix, hex_prefix}}

      {:ok, prefix_bytes} ->
        backend_module = backend.__struct__

        # Step 2: Ask the backend for all keys matching this byte prefix.
        case backend_module.keys_with_prefix(backend, prefix_bytes) do
          {:error, reason} ->
            {:error, wrap_store_error(reason)}

          {:ok, matches} ->
            # Sort for deterministic behaviour when there are multiple matches.
            # This mirrors the Rust implementation's `sort_unstable()` and ensures
            # that tests can predict the "ambiguous" error behaviour.
            sorted = Enum.sort(matches)

            # Step 3: Exactly one match is success; 0 or 2+ is an error.
            case sorted do
              [] ->
                {:error, {:prefix_not_found, hex_prefix}}

              [key] ->
                {:ok, key}

              _many ->
                {:error, {:ambiguous_prefix, hex_prefix}}
            end
        end
    end
  end

  @doc """
  Access the underlying `BlobStore` struct directly.

  Useful when you need backend-specific operations not exposed by the CAS
  interface (e.g., listing all keys for garbage collection, or querying storage
  statistics).
  """
  @spec inner(t()) :: struct()
  def inner(%__MODULE__{backend: backend}), do: backend

  # ─── Private Helpers ──────────────────────────────────────────────────────

  # Wrap an already-typed error reason, avoiding double-wrapping.
  # If the reason is already a :store_error tuple, return it unchanged.
  # Otherwise, wrap it so callers can distinguish backend errors from CAS errors.
  @spec wrap_store_error(term()) :: term()
  defp wrap_store_error({:store_error, _} = already_wrapped), do: already_wrapped
  defp wrap_store_error(reason), do: {:store_error, reason}
end
