defmodule CodingAdventures.ContentAddressableStorage.LocalDiskStore do
  @moduledoc """
  Filesystem-backed `BlobStore` using Git's 2/38 fanout directory layout.

  ## The 2/38 Fanout Layout

  Objects are stored at paths of the form:

      <root>/<xx>/<remaining-38-hex-chars>

  where `xx` is the first byte of the SHA-1 hash encoded as two lowercase hex
  digits, and the 38-char filename is the rest of the hash.

  Example — key `a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5`:

      <root>/
        a3/
          f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5

  ### Why Split at 2?

  Git pioneered this layout to avoid filesystem performance problems. Most
  filesystems degrade when a directory contains hundreds of thousands of entries.
  Splitting on the first byte creates up to 256 sub-directories (`00/` through
  `ff/`), keeping each sub-directory to a manageable size even in large
  repositories. A repo with 1 million objects averages ~3900 objects per bucket.

  ## Atomic Writes

  To avoid a reader seeing a partial write (which could be misidentified as
  corruption), writes use the rename trick:

  1. Write `data` to a temp file in the same directory as the final object.
  2. Call `File.rename/2` — on POSIX this is atomic; on Windows it is best-effort.
  3. If the final path already exists (race with another writer storing the same
     object), treat it as a successful no-op.

  ## Struct

  `%LocalDiskStore{root: path}` where `root` is an absolute path string.

      store = %CodingAdventures.ContentAddressableStorage.LocalDiskStore{root: "/tmp/myrepo/objects"}
  """

  alias CodingAdventures.ContentAddressableStorage.Hex

  @behaviour CodingAdventures.ContentAddressableStorage.BlobStore

  # The struct carries a single field: the root directory of the object store.
  @enforce_keys [:root]
  defstruct [:root]

  @type t :: %__MODULE__{root: String.t()}

  @doc """
  Create a new `LocalDiskStore` rooted at `root`.

  Creates `root` (and any parent directories) if it does not exist.

  ## Examples

      iex> store = CodingAdventures.ContentAddressableStorage.LocalDiskStore.new!(System.tmp_dir!() <> "/content_addressable_storage-test-new")
      iex> is_struct(store, CodingAdventures.ContentAddressableStorage.LocalDiskStore)
      true
  """
  @spec new!(String.t()) :: t()
  def new!(root) when is_binary(root) do
    :ok = File.mkdir_p!(root)
    %__MODULE__{root: root}
  end

  # ─── BlobStore Callbacks ───────────────────────────────────────────────────

  @doc """
  Persist `data` under `key` using an atomic rename.

  If the object already exists on disk (same key = same content by construction),
  the write is skipped and `:ok` is returned immediately.
  """
  @impl CodingAdventures.ContentAddressableStorage.BlobStore
  @spec put(t(), binary(), binary()) :: :ok | {:error, term()}
  def put(%__MODULE__{root: root}, key, data)
      when is_binary(key) and byte_size(key) == 20 and is_binary(data) do
    final_path = object_path(root, key)

    # Short-circuit: if the file already exists, the object is stored.
    # Content-addressing guarantees the stored bytes are identical — no need
    # to overwrite.
    if File.exists?(final_path) do
      :ok
    else
      write_atomic(final_path, data)
    end
  end

  @doc """
  Retrieve the blob stored under `key`.

  Returns `{:ok, data}` if found, `{:error, :not_found}` if the file is absent,
  or `{:error, {:store_error, reason}}` for other I/O failures.
  """
  @impl CodingAdventures.ContentAddressableStorage.BlobStore
  @spec get(t(), binary()) :: {:ok, binary()} | {:error, :not_found} | {:error, term()}
  def get(%__MODULE__{root: root}, key)
      when is_binary(key) and byte_size(key) == 20 do
    path = object_path(root, key)

    case File.read(path) do
      {:ok, data} -> {:ok, data}
      {:error, :enoent} -> {:error, :not_found}
      {:error, reason} -> {:error, {:store_error, reason}}
    end
  end

  @doc """
  Check whether `key` exists in the store without reading the blob.
  """
  @impl CodingAdventures.ContentAddressableStorage.BlobStore
  @spec exists?(t(), binary()) :: {:ok, boolean()} | {:error, term()}
  def exists?(%__MODULE__{root: root}, key)
      when is_binary(key) and byte_size(key) == 20 do
    {:ok, File.exists?(object_path(root, key))}
  end

  @doc """
  Return all stored keys whose first `byte_size(prefix)` bytes equal `prefix`.

  Scans the single fanout bucket determined by `prefix[0]` and collects entries
  whose reconstructed full key starts with the full prefix.
  """
  @impl CodingAdventures.ContentAddressableStorage.BlobStore
  @spec keys_with_prefix(t(), binary()) :: {:ok, [binary()]} | {:error, term()}
  def keys_with_prefix(%__MODULE__{root: root}, prefix)
      when is_binary(prefix) and byte_size(prefix) >= 1 do
    # The first byte of the prefix determines the fanout bucket directory.
    # E.g., prefix <<0xa3, ...>> → bucket directory "a3".
    <<first_byte, _rest::binary>> = prefix
    bucket_name = :io_lib.format("~2.16.0b", [first_byte]) |> IO.chardata_to_string()
    bucket_path = Path.join(root, bucket_name)

    if not File.dir?(bucket_path) do
      {:ok, []}
    else
      case File.ls(bucket_path) do
        {:error, reason} ->
          {:error, {:store_error, reason}}

        {:ok, entries} ->
          keys =
            entries
            |> Enum.filter(&valid_object_filename?/1)
            |> Enum.flat_map(fn filename ->
              # Each filename is a 38-char hex string (the last 38 chars of the
              # 40-char hash). Prepend the 2-char bucket name to reconstruct the
              # full 40-char hex, then parse it back to a 20-byte key.
              full_hex = bucket_name <> filename

              case Hex.hex_to_key(full_hex) do
                {:ok, key} ->
                  # Accept only if this key actually starts with the full prefix.
                  # The bucket narrows us to the right first-byte, but the prefix
                  # may be longer (e.g., 3 bytes), so we check the rest here.
                  if :binary.part(key, 0, byte_size(prefix)) == prefix do
                    [key]
                  else
                    []
                  end

                {:error, _} ->
                  # Skip files with non-hex names (temp files, etc.)
                  []
              end
            end)

          {:ok, keys}
      end
    end
  end

  # ─── Private Helpers ──────────────────────────────────────────────────────

  # Compute the object storage path for a 20-byte key.
  #
  # Key: <<0xa3, 0xf4, 0xb2, ...>>
  # Hex: "a3f4b2..."
  # Dir: "<root>/a3/"
  # File: "<root>/a3/f4b2..."
  @spec object_path(String.t(), binary()) :: String.t()
  defp object_path(root, key) when byte_size(key) == 20 do
    hex = Hex.key_to_hex(key)
    # Split the 40-char hex at position 2 to get the 2-char dir and 38-char file.
    {dir_name, file_name} = String.split_at(hex, 2)
    Path.join([root, dir_name, file_name])
  end

  # Write data atomically: write to a temp file in the same directory, then
  # rename into place.
  #
  # The temp file shares the object's directory so the rename stays on the same
  # filesystem (cross-device renames fail on POSIX). The name uses PID +
  # monotonic time to be practically unpredictable, avoiding a TOCTOU attack
  # where an adversary pre-creates a symlink at a predictable temp path.
  @spec write_atomic(String.t(), binary()) :: :ok | {:error, term()}
  defp write_atomic(final_path, data) do
    dir = Path.dirname(final_path)

    with :ok <- File.mkdir_p(dir) do
      # Build an unpredictable temp filename.
      # :erlang.unique_integer([:monotonic]) returns a strictly increasing integer
      # unique to this BEAM node, which combined with the OS PID makes collisions
      # essentially impossible.
      pid = System.pid()
      mono = :erlang.unique_integer([:monotonic])
      base = Path.basename(final_path)
      tmp_name = "#{base}.#{pid}.#{mono}.tmp"
      tmp_path = Path.join(dir, tmp_name)

      case File.write(tmp_path, data) do
        {:error, reason} ->
          {:error, {:store_error, reason}}

        :ok ->
          case File.rename(tmp_path, final_path) do
            :ok ->
              :ok

            {:error, reason} ->
              # Clean up the temp file to avoid orphans.
              File.rm(tmp_path)

              # If the final path now exists, a concurrent writer stored the same
              # object. That is fine — content-addressing guarantees same bytes.
              if File.exists?(final_path) do
                :ok
              else
                {:error, {:store_error, reason}}
              end
          end
      end
    end
  end

  # A valid object filename is exactly 38 lowercase hex characters.
  # This filters out temp files (which have extra dots and suffixes).
  @spec valid_object_filename?(String.t()) :: boolean()
  defp valid_object_filename?(name) do
    byte_size(name) == 38 and
      name =~ ~r/\A[0-9a-f]{38}\z/
  end
end
