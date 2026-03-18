# frozen_string_literal: true

# test_cache.rb -- Tests for build cache management
# ==================================================
#
# These tests verify cache loading, saving, the needs_build? logic, and
# atomic write safety.

require_relative "test_helper"

class TestCache < Minitest::Test
  include TestHelper

  # -- needs_build? tests ------------------------------------------------------

  def test_needs_build_true_when_not_cached
    cache = BuildTool::BuildCache.new
    assert cache.needs_build?("pkg-a", "hash1", "deps1")
  end

  def test_needs_build_false_when_cached_and_unchanged
    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "success")
    refute cache.needs_build?("pkg-a", "hash1", "deps1")
  end

  def test_needs_build_true_when_package_hash_changed
    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "success")
    assert cache.needs_build?("pkg-a", "hash2", "deps1")
  end

  def test_needs_build_true_when_deps_hash_changed
    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "success")
    assert cache.needs_build?("pkg-a", "hash1", "deps2")
  end

  def test_needs_build_true_when_last_build_failed
    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "failed")
    assert cache.needs_build?("pkg-a", "hash1", "deps1")
  end

  # -- record tests ------------------------------------------------------------

  def test_record_creates_entry
    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "success")

    entry = cache.entries["pkg-a"]
    assert_equal "hash1", entry.package_hash
    assert_equal "deps1", entry.deps_hash
    assert_equal "success", entry.status
    refute_nil entry.last_built
  end

  def test_record_overwrites_existing
    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "failed")
    cache.record("pkg-a", "hash2", "deps2", "success")

    entry = cache.entries["pkg-a"]
    assert_equal "hash2", entry.package_hash
    assert_equal "success", entry.status
  end

  # -- save/load round-trip tests ----------------------------------------------

  def test_save_and_load_round_trip
    dir = create_temp_dir
    cache_path = dir / ".build-cache.json"

    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "hash1", "deps1", "success")
    cache.record("pkg-b", "hash2", "deps2", "failed")
    cache.save(cache_path)

    # Load into a new cache instance.
    loaded = BuildTool::BuildCache.new
    loaded.load(cache_path)

    assert_equal 2, loaded.entries.size
    assert_equal "hash1", loaded.entries["pkg-a"].package_hash
    assert_equal "failed", loaded.entries["pkg-b"].status
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_load_missing_file_gives_empty_cache
    cache = BuildTool::BuildCache.new
    cache.load(Pathname("/nonexistent/.build-cache.json"))
    assert_equal({}, cache.entries)
  end

  def test_load_malformed_json_gives_empty_cache
    dir = create_temp_dir
    cache_path = dir / ".build-cache.json"
    write_file(cache_path, "not valid json{{{")

    cache = BuildTool::BuildCache.new
    cache.load(cache_path)
    assert_equal({}, cache.entries)
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_load_malformed_entry_skipped
    dir = create_temp_dir
    cache_path = dir / ".build-cache.json"
    # One valid entry, one missing required fields.
    write_file(cache_path, JSON.generate({
      "pkg-a" => { "package_hash" => "h1", "deps_hash" => "d1", "last_built" => "ts", "status" => "success" },
      "pkg-b" => { "bad" => "data" }
    }))

    cache = BuildTool::BuildCache.new
    cache.load(cache_path)
    assert_equal 1, cache.entries.size
    assert cache.entries.key?("pkg-a")
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_save_atomic_write_removes_tmp
    dir = create_temp_dir
    cache_path = dir / ".build-cache.json"

    cache = BuildTool::BuildCache.new
    cache.record("pkg-a", "h", "d", "success")
    cache.save(cache_path)

    assert cache_path.exist?
    refute (dir / ".build-cache.json.tmp").exist?
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- CacheEntry Data.define test ---------------------------------------------

  def test_cache_entry_is_data_define
    entry = BuildTool::CacheEntry.new(
      package_hash: "abc", deps_hash: "def",
      last_built: "2024-01-01", status: "success"
    )
    assert_equal "abc", entry.package_hash
    assert_equal "success", entry.status
  end
end
