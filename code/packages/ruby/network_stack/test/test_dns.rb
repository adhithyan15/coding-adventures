# frozen_string_literal: true

require_relative "test_helper"

class TestDNSResolver < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_resolve_localhost
    resolver = DNSResolver.new
    ip = resolver.resolve("localhost")
    assert_equal [127, 0, 0, 1], ip
  end

  def test_resolve_unknown_returns_nil
    resolver = DNSResolver.new
    assert_nil resolver.resolve("unknown.example.com")
  end

  def test_add_static_and_resolve
    resolver = DNSResolver.new
    resolver.add_static("example.com", [93, 184, 216, 34])

    ip = resolver.resolve("example.com")
    assert_equal [93, 184, 216, 34], ip
  end

  def test_add_multiple_static_entries
    resolver = DNSResolver.new
    resolver.add_static("a.com", [1, 2, 3, 4])
    resolver.add_static("b.com", [5, 6, 7, 8])

    assert_equal [1, 2, 3, 4], resolver.resolve("a.com")
    assert_equal [5, 6, 7, 8], resolver.resolve("b.com")
  end

  def test_overwrite_static_entry
    resolver = DNSResolver.new
    resolver.add_static("example.com", [1, 1, 1, 1])
    resolver.add_static("example.com", [8, 8, 8, 8])

    assert_equal [8, 8, 8, 8], resolver.resolve("example.com")
  end

  def test_resolve_returns_copy
    resolver = DNSResolver.new
    ip1 = resolver.resolve("localhost")
    ip2 = resolver.resolve("localhost")

    # Modifying one should not affect the other
    ip1[0] = 255
    assert_equal [127, 0, 0, 1], ip2
  end

  def test_static_table_accessor
    resolver = DNSResolver.new
    assert_instance_of Hash, resolver.static_table
    assert resolver.static_table.key?("localhost")
  end
end
