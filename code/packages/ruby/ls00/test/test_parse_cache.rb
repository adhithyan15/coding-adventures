# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# ParseCache Tests
# ================================================================
#
# Tests for the ParseCache which avoids re-parsing unchanged documents.
#
# ================================================================

class TestParseCache < Minitest::Test
  def test_hit_and_miss
    bridge = MockBridge.new
    cache = CodingAdventures::Ls00::ParseCache.new

    # First call -- cache miss -> parse
    r1 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
    refute_nil r1

    # Second call same version -- cache hit -> same object
    r2 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
    assert_same r1, r2

    # Different version -- cache miss -> new result
    r3 = cache.get_or_parse("file:///a.txt", 2, "hello world", bridge)
    refute_same r1, r3
  end

  def test_evict
    bridge = MockBridge.new
    cache = CodingAdventures::Ls00::ParseCache.new

    r1 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
    cache.evict("file:///a.txt")

    # After eviction, same (uri, version) produces a new parse
    r2 = cache.get_or_parse("file:///a.txt", 1, "hello", bridge)
    refute_same r1, r2
  end

  def test_diagnostics_populated
    bridge = MockBridge.new
    cache = CodingAdventures::Ls00::ParseCache.new

    result = cache.get_or_parse("file:///a.txt", 1, "source with ERROR token", bridge)
    refute_empty result.diagnostics
  end

  def test_no_diagnostics_for_clean_source
    bridge = MockBridge.new
    cache = CodingAdventures::Ls00::ParseCache.new

    result = cache.get_or_parse("file:///clean.txt", 1, "hello world", bridge)
    assert_empty result.diagnostics
  end
end
