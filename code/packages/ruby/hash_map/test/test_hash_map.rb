# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_hash_map"

class TestHashMap < Minitest::Test
  include CodingAdventures::HashMap

  class CollisionKey
    attr_reader :value

    def initialize(value)
      @value = value
    end

    def hash
      1
    end

    def eql?(other)
      other.is_a?(CollisionKey) && other.value == value
    end
  end

  def test_insert_fetch_and_delete
    map = HashMap.new
    map["name"] = "Ada"
    map["lang"] = "Ruby"

    assert_equal "Ada", map["name"]
    assert_equal "Ruby", map.fetch("lang")
    assert_equal "Ada", map.delete("name")
    refute map.key?("name")
    assert_equal 1, map.size
  end

  def test_collisions_resize_and_iteration
    map = HashMap.new(2)
    20.times { |i| map[CollisionKey.new(i)] = i }

    assert_equal 20, map.size
    assert_equal (0..19).to_a, map.values.sort
    assert_equal 20, map.keys.length
    assert_equal 20, map.to_h.length
  end

  def test_fetch_with_default_and_block
    map = HashMap.new
    assert_equal "missing", map.fetch("missing", "missing")
    assert_equal "computed", map.fetch("missing") { "computed" }
  end
end
