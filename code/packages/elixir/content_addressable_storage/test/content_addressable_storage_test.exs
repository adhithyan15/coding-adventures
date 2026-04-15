defmodule CodingAdventuresCasTest do
  use ExUnit.Case, async: true

  # Aliases for brevity throughout the test suite.
  alias CodingAdventures.ContentAddressableStorage.Store
  alias CodingAdventures.ContentAddressableStorage.LocalDiskStore
  alias CodingAdventures.ContentAddressableStorage.Hex
  alias CodingAdventures.ContentAddressableStorage.Error

  # ─── MemStore — an in-memory BlobStore for unit testing ─────────────────────
  #
  # A minimal `BlobStore` implementation backed by an ETS table. We use ETS
  # (Erlang Term Storage) rather than a plain map + Agent so that the store
  # can satisfy the `@behaviour` contract without involving process state in
  # non-process tests.
  #
  # This also exercises the BlobStore behaviour directly, confirming that any
  # module implementing the four callbacks plugs into Store correctly.

  defmodule MemStore do
    @moduledoc """
    In-memory `BlobStore` backed by an ETS table.

    ## Why ETS?

    The `BlobStore` behaviour's callbacks take `&self` (the struct) — there is
    no mutable state pointer. ETS lets us store state in a named table that the
    struct identifies by a reference, without requiring a GenServer. This mirrors
    how a real networked store might work: the struct is a handle (a URL + auth
    token), and each call reaches out to the remote state.
    """

    @behaviour CodingAdventures.ContentAddressableStorage.BlobStore

    @enforce_keys [:table]
    defstruct [:table]

    @type t :: %__MODULE__{table: :ets.tid()}

    @doc "Create a new, empty in-memory store."
    @spec new() :: t()
    def new do
      table = :ets.new(:mem_store, [:set, :public])
      %__MODULE__{table: table}
    end

    @impl CodingAdventures.ContentAddressableStorage.BlobStore
    def put(%__MODULE__{table: table} = _store, key, data) do
      :ets.insert(table, {key, data})
      :ok
    end

    @impl CodingAdventures.ContentAddressableStorage.BlobStore
    def get(%__MODULE__{table: table} = _store, key) do
      case :ets.lookup(table, key) do
        [{^key, data}] -> {:ok, data}
        [] -> {:error, :not_found}
      end
    end

    @impl CodingAdventures.ContentAddressableStorage.BlobStore
    def exists?(%__MODULE__{table: table} = _store, key) do
      {:ok, :ets.member(table, key)}
    end

    @impl CodingAdventures.ContentAddressableStorage.BlobStore
    def keys_with_prefix(%__MODULE__{table: table} = _store, prefix) do
      prefix_len = byte_size(prefix)

      keys =
        :ets.tab2list(table)
        |> Enum.filter(fn {key, _data} ->
          byte_size(key) == 20 and :binary.part(key, 0, prefix_len) == prefix
        end)
        |> Enum.map(fn {key, _data} -> key end)

      {:ok, keys}
    end
  end

  # ─── Helpers ────────────────────────────────────────────────────────────────

  # Create a fresh MemStore-backed Store for each test.
  defp mem_store, do: Store.new(MemStore.new())

  # Create a LocalDiskStore-backed Store in a unique temp directory.
  # Returns {store, tmp_dir} so tests can verify filesystem state.
  defp disk_store(test_name) do
    tmp = Path.join(System.tmp_dir!(), "cas_test_#{test_name}_#{System.unique_integer([:positive])}")
    backend = LocalDiskStore.new!(tmp)
    store = Store.new(backend)
    {store, tmp}
  end

  # Clean up a temp directory after a test.
  defp rm_rf(path), do: File.rm_rf!(path)

  # ─── Hex Utilities ──────────────────────────────────────────────────────────

  describe "Hex.key_to_hex/1" do
    test "encodes the empty-string SHA-1 correctly" do
      # SHA-1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
      # This is a well-known test vector (RFC 3174 and NIST FIPS 180-4).
      {:ok, key} = Hex.hex_to_key("da39a3ee5e6b4b0d3255bfef95601890afd80709")
      assert Hex.key_to_hex(key) == "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    end

    test "round-trips all-zero key" do
      key = <<0::160>>
      assert Hex.key_to_hex(key) == String.duplicate("0", 40)
    end

    test "round-trips all-0xff key" do
      key = :binary.copy(<<0xFF>>, 20)
      assert Hex.key_to_hex(key) == String.duplicate("f", 40)
    end
  end

  describe "Hex.hex_to_key/1" do
    test "parses valid 40-char lowercase hex" do
      {:ok, key} = Hex.hex_to_key("a9993e364706816aba3e25717850c26c9cd0d89d")
      assert byte_size(key) == 20
    end

    test "parses valid 40-char uppercase hex" do
      {:ok, key} = Hex.hex_to_key("A9993E364706816ABA3E25717850C26C9CD0D89D")
      assert byte_size(key) == 20
    end

    test "rejects strings shorter than 40 chars" do
      assert {:error, :invalid_hex} = Hex.hex_to_key("a9993e")
    end

    test "rejects strings longer than 40 chars" do
      assert {:error, :invalid_hex} = Hex.hex_to_key(String.duplicate("a", 41))
    end

    test "rejects non-hex characters" do
      assert {:error, :invalid_hex} = Hex.hex_to_key("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz")
    end
  end

  describe "Hex.decode_hex_prefix/1" do
    test "decodes even-length hex prefix" do
      assert {:ok, <<0xa3, 0xf4>>} = Hex.decode_hex_prefix("a3f4")
    end

    test "decodes odd-length hex prefix by right-padding with 0" do
      # "a3f" → pad → "a3f0" → <<0xa3, 0xf0>>
      assert {:ok, <<0xa3, 0xf0>>} = Hex.decode_hex_prefix("a3f")
    end

    test "single nibble prefix" do
      # "a" → pad → "a0" → <<0xa0>>
      assert {:ok, <<0xa0>>} = Hex.decode_hex_prefix("a")
    end

    test "full 40-char prefix" do
      hex = "da39a3ee5e6b4b0d3255bfef95601890afd80709"
      {:ok, prefix} = Hex.decode_hex_prefix(hex)
      assert byte_size(prefix) == 20
    end

    test "empty string returns error" do
      assert {:error, :invalid_hex} = Hex.decode_hex_prefix("")
    end

    test "non-hex characters return error" do
      assert {:error, :invalid_hex} = Hex.decode_hex_prefix("zz")
    end

    test "mixed valid/invalid characters return error" do
      assert {:error, :invalid_hex} = Hex.decode_hex_prefix("a3z4")
    end
  end

  # ─── Error Formatting ───────────────────────────────────────────────────────

  describe "Error.format/1" do
    test "formats :not_found" do
      assert Error.format(:not_found) == "object not found"
    end

    test "formats {:corrupted, key}" do
      key = <<0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55,
              0xbf, 0xef, 0x95, 0x60, 0x18, 0x90, 0xaf, 0xd8, 0x07, 0x09>>
      result = Error.format({:corrupted, key})
      assert result =~ "corrupted"
      assert result =~ "da39a3ee"
    end

    test "formats {:ambiguous_prefix, prefix}" do
      assert Error.format({:ambiguous_prefix, "a3f4"}) =~ "ambiguous"
    end

    test "formats {:prefix_not_found, prefix}" do
      assert Error.format({:prefix_not_found, "dead"}) =~ "not found"
    end

    test "formats {:invalid_prefix, prefix}" do
      assert Error.format({:invalid_prefix, "xyz!"}) =~ "invalid"
    end

    test "formats {:store_error, reason}" do
      assert Error.format({:store_error, :enoent}) =~ "store error"
    end
  end

  # ─── BlobStore Behaviour — MemStore ─────────────────────────────────────────

  describe "MemStore behaviour" do
    test "put then get returns the same data" do
      m = MemStore.new()
      key = <<1::160>>
      assert :ok = MemStore.put(m, key, "hello")
      assert {:ok, "hello"} = MemStore.get(m, key)
    end

    test "get missing key returns :not_found" do
      m = MemStore.new()
      assert {:error, :not_found} = MemStore.get(m, <<0::160>>)
    end

    test "exists? returns false before put, true after" do
      m = MemStore.new()
      key = <<2::160>>
      assert {:ok, false} = MemStore.exists?(m, key)
      :ok = MemStore.put(m, key, "data")
      assert {:ok, true} = MemStore.exists?(m, key)
    end

    test "keys_with_prefix returns matching keys" do
      m = MemStore.new()
      key_a = <<0xa3, 0::152>>
      key_b = <<0xa3, 1::152>>
      key_c = <<0xb0, 0::152>>
      :ok = MemStore.put(m, key_a, "a")
      :ok = MemStore.put(m, key_b, "b")
      :ok = MemStore.put(m, key_c, "c")
      {:ok, results} = MemStore.keys_with_prefix(m, <<0xa3>>)
      assert length(results) == 2
      assert key_a in results
      assert key_b in results
      refute key_c in results
    end
  end

  # ─── Store — Round Trip Tests ─────────────────────────────────────────────

  describe "Store round-trip (MemStore backend)" do
    test "empty blob round-trip" do
      store = mem_store()
      {:ok, key} = Store.put(store, "")
      assert {:ok, ""} = Store.get(store, key)
    end

    test "small blob round-trip" do
      store = mem_store()
      data = "hello, world"
      {:ok, key} = Store.put(store, data)
      assert {:ok, ^data} = Store.get(store, key)
    end

    test "1 MiB blob round-trip" do
      store = mem_store()
      # Generate 1 MiB of pseudo-random bytes deterministically.
      # We repeat a pattern rather than using :crypto.strong_rand_bytes so that
      # this test is reproducible without any crypto dependency.
      data = :binary.copy(<<0xDE, 0xAD, 0xBE, 0xEF>>, 256 * 1024)
      assert byte_size(data) == 1_048_576
      {:ok, key} = Store.put(store, data)
      assert {:ok, ^data} = Store.get(store, key)
    end

    test "put returns correct SHA-1 key" do
      store = mem_store()
      # SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
      {:ok, key} = Store.put(store, "abc")
      assert Hex.key_to_hex(key) == "a9993e364706816aba3e25717850c26c9cd0d89d"
    end
  end

  describe "Store idempotent put" do
    test "putting the same content twice returns the same key" do
      store = mem_store()
      {:ok, key1} = Store.put(store, "idempotent data")
      {:ok, key2} = Store.put(store, "idempotent data")
      assert key1 == key2
    end

    test "put is truly idempotent — no error on second put" do
      store = mem_store()
      assert {:ok, _key} = Store.put(store, "foo")
      assert {:ok, _key} = Store.put(store, "foo")
    end
  end

  describe "Store.get/2 — not found" do
    test "get on unknown key returns {:error, :not_found}" do
      store = mem_store()
      fake_key = <<0::160>>
      assert {:error, :not_found} = Store.get(store, fake_key)
    end
  end

  describe "Store.exists?/2" do
    test "returns false before put" do
      store = mem_store()
      key_hash = CodingAdventures.Sha1.sha1("existence-test")
      assert {:ok, false} = Store.exists?(store, key_hash)
    end

    test "returns true after put" do
      store = mem_store()
      {:ok, key} = Store.put(store, "existence-test")
      assert {:ok, true} = Store.exists?(store, key)
    end
  end

  # ─── Store — Integrity Check ───────────────────────────────────────────────

  describe "Store integrity check (corrupted data)" do
    test "corrupted file returns {:error, {:corrupted, key}}" do
      # To simulate corruption we use a MemStore where we manually overwrite
      # the raw bytes after put, bypassing the Store layer.
      backend = MemStore.new()
      store = Store.new(backend)

      {:ok, key} = Store.put(store, "original data")

      # Corrupt the stored bytes directly in the ETS table.
      :ets.insert(backend.table, {key, "tampered bytes that do not hash to key"})

      assert {:error, {:corrupted, ^key}} = Store.get(store, key)
    end
  end

  # ─── Store — find_by_prefix ────────────────────────────────────────────────

  describe "Store.find_by_prefix/2" do
    test "unique prefix resolves to the correct key" do
      store = mem_store()
      {:ok, key} = Store.put(store, "prefix search target")
      hex = Hex.key_to_hex(key)
      # Use first 8 chars (4 bytes) as the prefix.
      short = String.slice(hex, 0, 8)
      assert {:ok, ^key} = Store.find_by_prefix(store, short)
    end

    test "full 40-char prefix resolves exactly" do
      store = mem_store()
      {:ok, key} = Store.put(store, "exact prefix")
      hex = Hex.key_to_hex(key)
      assert {:ok, ^key} = Store.find_by_prefix(store, hex)
    end

    test "ambiguous prefix returns {:error, {:ambiguous_prefix, prefix}}" do
      # We need two objects that share a common prefix. We construct the objects
      # such that their SHA-1 digests share the first byte by inserting known
      # keys directly into the MemStore, bypassing SHA-1 computation.
      #
      # Both keys start with <<0xa3>>, so prefix "a3" is ambiguous.
      backend = MemStore.new()
      store = Store.new(backend)

      key1 = <<0xa3, 1::152>>
      key2 = <<0xa3, 2::152>>
      :ok = MemStore.put(backend, key1, "blob1")
      :ok = MemStore.put(backend, key2, "blob2")

      assert {:error, {:ambiguous_prefix, "a3"}} = Store.find_by_prefix(store, "a3")
    end

    test "no match returns {:error, {:prefix_not_found, prefix}}" do
      store = mem_store()
      # Store is empty, so any prefix will not be found.
      assert {:error, {:prefix_not_found, "deadbeef"}} =
               Store.find_by_prefix(store, "deadbeef")
    end

    test "invalid hex characters return {:error, {:invalid_prefix, prefix}}" do
      store = mem_store()
      assert {:error, {:invalid_prefix, "xyz!"}} = Store.find_by_prefix(store, "xyz!")
    end

    test "empty string returns {:error, {:invalid_prefix, \"\"}}" do
      store = mem_store()
      assert {:error, {:invalid_prefix, ""}} = Store.find_by_prefix(store, "")
    end

    test "odd-length valid prefix works" do
      backend = MemStore.new()
      store = Store.new(backend)

      # Key starts with 0xa3 — prefix "a" → <<0xa0>> won't match it.
      # Prefix "a3" will match. Odd prefix "a3f" will match if second nibble is f.
      # Use a key with second byte 0xf4 so "a3f" (→ <<0xa3, 0xf0>>) won't match.
      # Use a key with second byte 0xf0 so "a3f" (→ <<0xa3, 0xf0>>) will match.
      key = <<0xa3, 0xf0, 0::144>>
      :ok = MemStore.put(backend, key, "odd prefix test")

      # "a3f" → <<0xa3, 0xf0>> — matches key exactly in first 2 bytes.
      assert {:ok, ^key} = Store.find_by_prefix(store, "a3f")
    end
  end

  # ─── Store.inner/1 ──────────────────────────────────────────────────────────

  describe "Store.inner/1" do
    test "returns the underlying backend struct" do
      backend = MemStore.new()
      store = Store.new(backend)
      assert Store.inner(store) == backend
    end
  end

  # ─── LocalDiskStore — path layout ────────────────────────────────────────────

  describe "LocalDiskStore path layout" do
    test "2/38 directory structure is created on put" do
      {store, tmp} = disk_store("path_layout")

      try do
        {:ok, key} = Store.put(store, "layout test")
        hex = Hex.key_to_hex(key)

        # The first 2 hex chars become the directory name.
        dir_name = String.slice(hex, 0, 2)
        # The remaining 38 chars become the file name.
        file_name = String.slice(hex, 2, 38)

        dir_path = Path.join(tmp, dir_name)
        file_path = Path.join(dir_path, file_name)

        assert File.dir?(dir_path), "Expected directory #{dir_path} to exist"
        assert File.exists?(file_path), "Expected file #{file_path} to exist"
      after
        rm_rf(tmp)
      end
    end

    test "directory name is exactly 2 lowercase hex chars" do
      {store, tmp} = disk_store("dir_name")

      try do
        {:ok, key} = Store.put(store, "dir name test")
        hex = Hex.key_to_hex(key)
        dir_name = String.slice(hex, 0, 2)

        # Must be exactly 2 chars of lowercase hex.
        assert String.length(dir_name) == 2
        assert dir_name =~ ~r/\A[0-9a-f]{2}\z/
        assert File.dir?(Path.join(tmp, dir_name))
      after
        rm_rf(tmp)
      end
    end

    test "file name is exactly 38 lowercase hex chars" do
      {store, tmp} = disk_store("file_name")

      try do
        {:ok, key} = Store.put(store, "file name test")
        hex = Hex.key_to_hex(key)
        dir_name = String.slice(hex, 0, 2)
        file_name = String.slice(hex, 2, 38)

        assert String.length(file_name) == 38
        assert file_name =~ ~r/\A[0-9a-f]{38}\z/
        assert File.exists?(Path.join([tmp, dir_name, file_name]))
      after
        rm_rf(tmp)
      end
    end

    test "file contents match the stored data" do
      {store, tmp} = disk_store("file_contents")

      try do
        data = "contents verification"
        {:ok, key} = Store.put(store, data)
        hex = Hex.key_to_hex(key)
        dir_name = String.slice(hex, 0, 2)
        file_name = String.slice(hex, 2, 38)
        file_path = Path.join([tmp, dir_name, file_name])

        assert File.read!(file_path) == data
      after
        rm_rf(tmp)
      end
    end
  end

  # ─── LocalDiskStore — round trip ─────────────────────────────────────────────

  describe "LocalDiskStore round-trip" do
    test "empty blob" do
      {store, tmp} = disk_store("empty")

      try do
        {:ok, key} = Store.put(store, "")
        assert {:ok, ""} = Store.get(store, key)
      after
        rm_rf(tmp)
      end
    end

    test "small blob" do
      {store, tmp} = disk_store("small")

      try do
        data = "disk store test data"
        {:ok, key} = Store.put(store, data)
        assert {:ok, ^data} = Store.get(store, key)
      after
        rm_rf(tmp)
      end
    end

    test "1 MiB blob" do
      {store, tmp} = disk_store("large")

      try do
        data = :binary.copy(<<0xCA, 0xFE>>, 524_288)
        assert byte_size(data) == 1_048_576
        {:ok, key} = Store.put(store, data)
        assert {:ok, ^data} = Store.get(store, key)
      after
        rm_rf(tmp)
      end
    end

    test "idempotent put does not error" do
      {store, tmp} = disk_store("idempotent")

      try do
        assert {:ok, key1} = Store.put(store, "idempotent disk")
        assert {:ok, key2} = Store.put(store, "idempotent disk")
        assert key1 == key2
      after
        rm_rf(tmp)
      end
    end

    test "exists? returns false before put, true after" do
      {store, tmp} = disk_store("exists")

      try do
        key = CodingAdventures.Sha1.sha1("exists disk test")
        assert {:ok, false} = Store.exists?(store, key)
        Store.put(store, "exists disk test")
        assert {:ok, true} = Store.exists?(store, key)
      after
        rm_rf(tmp)
      end
    end

    test "get missing key returns :not_found" do
      {store, tmp} = disk_store("not_found")

      try do
        assert {:error, :not_found} = Store.get(store, <<0::160>>)
      after
        rm_rf(tmp)
      end
    end
  end

  # ─── LocalDiskStore — integrity check ────────────────────────────────────────

  describe "LocalDiskStore integrity check" do
    test "corrupted file on disk returns {:error, {:corrupted, key}}" do
      {store, tmp} = disk_store("corrupted")

      try do
        data = "data to corrupt"
        {:ok, key} = Store.put(store, data)

        # Locate the object file and overwrite it with garbage.
        hex = Hex.key_to_hex(key)
        dir_name = String.slice(hex, 0, 2)
        file_name = String.slice(hex, 2, 38)
        file_path = Path.join([tmp, dir_name, file_name])

        File.write!(file_path, "this is not the original data — CORRUPTED")

        assert {:error, {:corrupted, ^key}} = Store.get(store, key)
      after
        rm_rf(tmp)
      end
    end
  end

  # ─── LocalDiskStore — find_by_prefix ─────────────────────────────────────────

  describe "LocalDiskStore.find_by_prefix/2" do
    test "unique prefix resolves to the correct key" do
      {store, tmp} = disk_store("prefix_unique")

      try do
        {:ok, key} = Store.put(store, "disk prefix test")
        hex = Hex.key_to_hex(key)
        short = String.slice(hex, 0, 10)
        assert {:ok, ^key} = Store.find_by_prefix(store, short)
      after
        rm_rf(tmp)
      end
    end

    test "prefix not found returns {:error, {:prefix_not_found, _}}" do
      {store, tmp} = disk_store("prefix_not_found")

      try do
        assert {:error, {:prefix_not_found, "deadbeef"}} =
                 Store.find_by_prefix(store, "deadbeef")
      after
        rm_rf(tmp)
      end
    end

    test "invalid hex prefix returns {:error, {:invalid_prefix, _}}" do
      {store, tmp} = disk_store("prefix_invalid")

      try do
        assert {:error, {:invalid_prefix, "not-hex"}} =
                 Store.find_by_prefix(store, "not-hex")
      after
        rm_rf(tmp)
      end
    end
  end
end
