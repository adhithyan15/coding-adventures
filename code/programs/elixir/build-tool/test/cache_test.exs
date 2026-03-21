defmodule BuildTool.CacheTest do
  use ExUnit.Case, async: true

  alias BuildTool.Cache

  # ---------------------------------------------------------------------------
  # Setup
  # ---------------------------------------------------------------------------

  setup do
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_cache_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

    {:ok, cache} = Cache.start_link()

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir, cache: cache}
  end

  # ---------------------------------------------------------------------------
  # start_link/0
  # ---------------------------------------------------------------------------

  describe "start_link/0" do
    test "starts with empty entries" do
      {:ok, cache} = Cache.start_link()
      assert Cache.entries(cache) == %{}
    end
  end

  # ---------------------------------------------------------------------------
  # load/2
  # ---------------------------------------------------------------------------

  describe "load/2" do
    test "loads entries from a JSON file", %{tmp_dir: tmp_dir, cache: cache} do
      path = Path.join(tmp_dir, "cache.json")

      File.write!(path, """
      {
        "python/logic-gates": {
          "package_hash": "abc123",
          "deps_hash": "def456",
          "last_built": "2024-01-15T10:30:00Z",
          "status": "success"
        }
      }
      """)

      Cache.load(cache, path)
      entries = Cache.entries(cache)
      assert Map.has_key?(entries, "python/logic-gates")
      assert entries["python/logic-gates"]["package_hash"] == "abc123"
    end

    test "starts empty on missing file", %{cache: cache} do
      Cache.load(cache, "/nonexistent/cache.json")
      assert Cache.entries(cache) == %{}
    end

    test "starts empty on malformed JSON", %{tmp_dir: tmp_dir, cache: cache} do
      path = Path.join(tmp_dir, "bad.json")
      File.write!(path, "not valid json {{{}}")
      Cache.load(cache, path)
      assert Cache.entries(cache) == %{}
    end
  end

  # ---------------------------------------------------------------------------
  # save/2
  # ---------------------------------------------------------------------------

  describe "save/2" do
    test "writes entries to a JSON file", %{tmp_dir: tmp_dir, cache: cache} do
      path = Path.join(tmp_dir, "output.json")

      Cache.record(cache, "go/graph", "hash1", "hash2", "success")
      assert :ok = Cache.save(cache, path)

      {:ok, data} = File.read(path)
      decoded = Jason.decode!(data)
      assert Map.has_key?(decoded, "go/graph")
      assert decoded["go/graph"]["status"] == "success"
    end

    test "round-trips through load and save", %{tmp_dir: tmp_dir, cache: cache} do
      path = Path.join(tmp_dir, "round-trip.json")

      Cache.record(cache, "python/gates", "pkg_h", "dep_h", "success")
      Cache.save(cache, path)

      {:ok, cache2} = Cache.start_link()
      Cache.load(cache2, path)

      assert Cache.entries(cache2)["python/gates"]["package_hash"] == "pkg_h"
    end

    test "atomic write uses temp file", %{tmp_dir: tmp_dir, cache: cache} do
      path = Path.join(tmp_dir, "atomic.json")
      Cache.record(cache, "test", "a", "b", "success")
      Cache.save(cache, path)

      # The .tmp file should not exist after a successful save.
      refute File.exists?(path <> ".tmp")
      assert File.exists?(path)
    end
  end

  # ---------------------------------------------------------------------------
  # needs_build?/4
  # ---------------------------------------------------------------------------

  describe "needs_build?/4" do
    test "returns true for unknown package", %{cache: cache} do
      assert Cache.needs_build?(cache, "unknown/pkg", "hash1", "hash2")
    end

    test "returns true when package hash changed", %{cache: cache} do
      Cache.record(cache, "pkg", "old_hash", "dep_hash", "success")
      assert Cache.needs_build?(cache, "pkg", "new_hash", "dep_hash")
    end

    test "returns true when deps hash changed", %{cache: cache} do
      Cache.record(cache, "pkg", "pkg_hash", "old_deps", "success")
      assert Cache.needs_build?(cache, "pkg", "pkg_hash", "new_deps")
    end

    test "returns true when last build failed", %{cache: cache} do
      Cache.record(cache, "pkg", "pkg_hash", "dep_hash", "failed")
      assert Cache.needs_build?(cache, "pkg", "pkg_hash", "dep_hash")
    end

    test "returns false when nothing changed", %{cache: cache} do
      Cache.record(cache, "pkg", "pkg_hash", "dep_hash", "success")
      refute Cache.needs_build?(cache, "pkg", "pkg_hash", "dep_hash")
    end
  end

  # ---------------------------------------------------------------------------
  # record/5
  # ---------------------------------------------------------------------------

  describe "record/5" do
    test "stores a build result", %{cache: cache} do
      Cache.record(cache, "python/gates", "abc", "def", "success")
      entries = Cache.entries(cache)

      assert entries["python/gates"]["package_hash"] == "abc"
      assert entries["python/gates"]["deps_hash"] == "def"
      assert entries["python/gates"]["status"] == "success"
      assert entries["python/gates"]["last_built"] != nil
    end

    test "overwrites previous entry", %{cache: cache} do
      Cache.record(cache, "pkg", "v1", "d1", "success")
      Cache.record(cache, "pkg", "v2", "d2", "failed")

      entries = Cache.entries(cache)
      assert entries["pkg"]["package_hash"] == "v2"
      assert entries["pkg"]["status"] == "failed"
    end
  end
end
